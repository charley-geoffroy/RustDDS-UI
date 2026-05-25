//! Lightweight publisher-side counters: how many samples were sent,
//! how long the local `write()` call took, and what the effective rate
//! is over the run.
//!
//! Designed to live next to a `TypedPublisher` as an `Arc<PubMetrics>`,
//! so a heartbeat thread can read it concurrently with publishing.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

/// Concurrent, lock-free publisher counters.
pub struct PubMetrics {
    started_ns: u64,
    sent: AtomicU64,
    errors: AtomicU64,
    last_send_ns: AtomicU64,
    write_ns_total: AtomicU64,
    write_ns_max: AtomicU64,
}

impl PubMetrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            started_ns: now_ns(),
            sent: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            last_send_ns: AtomicU64::new(0),
            write_ns_total: AtomicU64::new(0),
            write_ns_max: AtomicU64::new(0),
        })
    }

    /// Record a successful `write()` call along with how long it took
    /// (the local DataWriter handoff — serialize + queue, not E2E).
    pub fn record_write(&self, write_ns: u64) {
        self.sent.fetch_add(1, Ordering::Relaxed);
        self.last_send_ns.store(now_ns(), Ordering::Relaxed);
        self.write_ns_total.fetch_add(write_ns, Ordering::Relaxed);
        self.write_ns_max.fetch_max(write_ns, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Current sample count. Used by `MatchTable` to snapshot how many
    /// samples were sent at the moment of a match/unmatch event.
    pub fn sent(&self) -> u64 {
        self.sent.load(Ordering::Relaxed)
    }

    pub fn snapshot(&self) -> PubMetricsSnapshot {
        let now = now_ns();
        let sent = self.sent.load(Ordering::Relaxed);
        let total_ns = self.write_ns_total.load(Ordering::Relaxed);
        let elapsed_s = ((now.saturating_sub(self.started_ns)) as f64) / 1e9;
        PubMetricsSnapshot {
            started_ns: self.started_ns,
            now_ns: now,
            elapsed_s,
            sent,
            errors: self.errors.load(Ordering::Relaxed),
            last_send_ns: self.last_send_ns.load(Ordering::Relaxed),
            rate_per_s: if elapsed_s > 0.0 { sent as f64 / elapsed_s } else { 0.0 },
            write_ns_avg: if sent > 0 { total_ns / sent } else { 0 },
            write_ns_max: self.write_ns_max.load(Ordering::Relaxed),
        }
    }
}

/// JSON-serializable view of `PubMetrics` at one instant.
#[derive(Serialize, Clone, Debug)]
pub struct PubMetricsSnapshot {
    pub started_ns: u64,
    pub now_ns: u64,
    pub elapsed_s: f64,
    pub sent: u64,
    pub errors: u64,
    pub last_send_ns: u64,
    pub rate_per_s: f64,
    pub write_ns_avg: u64,
    pub write_ns_max: u64,
}

impl PubMetricsSnapshot {
    /// One-line human summary for periodic heartbeats.
    pub fn format_line(&self) -> String {
        format!(
            "sent={} ({:.1}/s) errs={} write_avg={}µs write_max={}µs uptime={:.1}s",
            self.sent,
            self.rate_per_s,
            self.errors,
            self.write_ns_avg / 1_000,
            self.write_ns_max / 1_000,
            self.elapsed_s,
        )
    }

    /// Pretty-printed JSON suitable for a final dump on exit.
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

// ============================================================================
// MatchTable — tracks which remote readers our writer is/was matched with
// ============================================================================

/// One reader's lifetime as seen by our writer. Captures when the match
/// opened/closed and how many of our samples were sent during the window
/// (via snapshots of the global `sent` counter, not per-reader bookkeeping).
#[derive(Clone, Serialize, Debug)]
pub struct MatchSession {
    pub reader_guid: String,
    pub first_matched_ns: u64,
    pub last_unmatched_ns: Option<u64>,
    pub sent_at_match: u64,
    pub sent_at_unmatch: Option<u64>,
}

impl MatchSession {
    /// Samples sent while this match was open. For still-open sessions,
    /// pass the current `sent` counter.
    pub fn samples_during(&self, current_sent: u64) -> u64 {
        let end = self.sent_at_unmatch.unwrap_or(current_sent);
        end.saturating_sub(self.sent_at_match)
    }
}

#[derive(Clone, Debug)]
pub enum MatchEvent {
    Matched(MatchSession),
    Unmatched(MatchSession),
}

struct MatchTableInner {
    open: HashMap<String, MatchSession>,
    closed: Vec<MatchSession>,
    pending: Vec<MatchEvent>,
}

/// Shared, thread-safe history of writer↔reader match sessions. The
/// `pending` queue lets a reporting thread print events near-real-time
/// without diffing snapshots.
pub struct MatchTable {
    inner: Mutex<MatchTableInner>,
}

impl MatchTable {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(MatchTableInner {
                open: HashMap::new(),
                closed: Vec::new(),
                pending: Vec::new(),
            }),
        })
    }

    pub fn on_matched(&self, reader_guid: String, sent_now: u64) {
        let mut inner = self.inner.lock().unwrap();
        if inner.open.contains_key(&reader_guid) {
            return; // already open — defensive against duplicate events
        }
        let session = MatchSession {
            reader_guid: reader_guid.clone(),
            first_matched_ns: now_ns(),
            last_unmatched_ns: None,
            sent_at_match: sent_now,
            sent_at_unmatch: None,
        };
        inner.open.insert(reader_guid, session.clone());
        inner.pending.push(MatchEvent::Matched(session));
    }

    pub fn on_unmatched(&self, reader_guid: &str, sent_now: u64) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(mut session) = inner.open.remove(reader_guid) {
            session.last_unmatched_ns = Some(now_ns());
            session.sent_at_unmatch = Some(sent_now);
            inner.closed.push(session.clone());
            inner.pending.push(MatchEvent::Unmatched(session));
        }
    }

    /// Take all events accumulated since the last drain. Intended for a
    /// reporter thread.
    pub fn drain_events(&self) -> Vec<MatchEvent> {
        std::mem::take(&mut self.inner.lock().unwrap().pending)
    }

    pub fn snapshot(&self) -> MatchTableSnapshot {
        let inner = self.inner.lock().unwrap();
        MatchTableSnapshot {
            open: inner.open.values().cloned().collect(),
            closed: inner.closed.clone(),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct MatchTableSnapshot {
    pub open: Vec<MatchSession>,
    pub closed: Vec<MatchSession>,
}

// ============================================================================
// Combined report — what a publisher dumps at exit
// ============================================================================

#[derive(Serialize, Clone, Debug)]
pub struct PubReport {
    pub metrics: PubMetricsSnapshot,
    pub matches: MatchTableSnapshot,
}

impl PubReport {
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

fn now_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ============================================================================
// TimeBuckets — fixed-window event histogram for ASCII sparklines
// ============================================================================

const HISTORY_CAP: usize = 60;
const BUCKET_SECS: u64 = 1;

/// 1-second resolution rolling histogram. Holds at most the last
/// `HISTORY_CAP` buckets; older buckets are dropped as new ones open.
struct TimeBuckets {
    buckets: VecDeque<u64>,
    current_start_s: u64,
}

impl TimeBuckets {
    fn new() -> Self {
        Self {
            buckets: VecDeque::new(),
            current_start_s: 0,
        }
    }

    fn record(&mut self, count: u64) {
        let now = now_s();
        if self.buckets.is_empty() {
            self.current_start_s = now;
            self.buckets.push_back(0);
        }
        while now >= self.current_start_s + BUCKET_SECS {
            self.current_start_s += BUCKET_SECS;
            self.buckets.push_back(0);
            if self.buckets.len() > HISTORY_CAP {
                self.buckets.pop_front();
            }
        }
        if let Some(last) = self.buckets.back_mut() {
            *last += count;
        }
    }

    fn sparkline(&self) -> String {
        const CHARS: [char; 9] = [' ', '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}',
                                  '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}'];
        let max = self.buckets.iter().copied().max().unwrap_or(0);
        if max == 0 {
            return " ".repeat(self.buckets.len());
        }
        self.buckets
            .iter()
            .map(|&v| {
                let idx = ((v as f64 / max as f64) * 8.0).round() as usize;
                CHARS[idx.min(8)]
            })
            .collect()
    }
}

// ============================================================================
// SubMetrics — subscriber-side counters + per-publisher loss tracking
// ============================================================================

/// Per-publisher counter-gap state. A "publisher" here is identified by
/// the `publisher_id` field in the message (caller's responsibility).
struct StreamState {
    last_counter: u64,
    first_counter_seen: u64,
    received: u64,
    lost: u64,
}

pub struct SubMetrics {
    started_ns: u64,
    received: AtomicU64,
    /// Wire loss detected by counter gaps from the published stream.
    lost_wire: AtomicU64,
    reordered: AtomicU64,
    duplicates: AtomicU64,
    /// Loss reported by the local DDS stack via `DataReaderStatus::SampleLost`.
    lost_dds_local: AtomicU64,
    streams: Mutex<HashMap<u32, StreamState>>,
    history: Mutex<SubHistory>,
}

struct SubHistory {
    recv: TimeBuckets,
    lost: TimeBuckets,
}

impl SubMetrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            started_ns: now_ns(),
            received: AtomicU64::new(0),
            lost_wire: AtomicU64::new(0),
            reordered: AtomicU64::new(0),
            duplicates: AtomicU64::new(0),
            lost_dds_local: AtomicU64::new(0),
            streams: Mutex::new(HashMap::new()),
            history: Mutex::new(SubHistory {
                recv: TimeBuckets::new(),
                lost: TimeBuckets::new(),
            }),
        })
    }

    /// Record one received sample. `publisher_id` and `counter` come
    /// from the message payload (the demo extracts them from `Chatter`).
    pub fn on_sample(&self, publisher_id: u32, counter: u64) {
        let mut streams = self.streams.lock().unwrap();
        let state = streams.entry(publisher_id).or_insert(StreamState {
            last_counter: counter,
            first_counter_seen: counter,
            received: 0,
            lost: 0,
        });

        // First sample from this publisher — establish the baseline,
        // don't count anything before it as loss.
        if state.received == 0 {
            state.received = 1;
            state.last_counter = counter;
            state.first_counter_seen = counter;
            self.received.fetch_add(1, Ordering::Relaxed);
            self.history.lock().unwrap().recv.record(1);
            return;
        }

        if counter == state.last_counter + 1 {
            state.last_counter = counter;
            state.received += 1;
            self.received.fetch_add(1, Ordering::Relaxed);
            self.history.lock().unwrap().recv.record(1);
        } else if counter > state.last_counter + 1 {
            let gap = counter - state.last_counter - 1;
            state.lost += gap;
            state.last_counter = counter;
            state.received += 1;
            self.lost_wire.fetch_add(gap, Ordering::Relaxed);
            self.received.fetch_add(1, Ordering::Relaxed);
            let mut h = self.history.lock().unwrap();
            h.recv.record(1);
            h.lost.record(gap);
        } else if counter == state.last_counter {
            self.duplicates.fetch_add(1, Ordering::Relaxed);
        } else if counter < state.first_counter_seen {
            // Publisher restarted with a reset counter — rebaseline.
            state.last_counter = counter;
            state.first_counter_seen = counter;
            state.received += 1;
            self.received.fetch_add(1, Ordering::Relaxed);
            self.history.lock().unwrap().recv.record(1);
        } else {
            self.reordered.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_sample_lost_dds(&self, n: u64) {
        self.lost_dds_local.fetch_add(n, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> SubMetricsSnapshot {
        let now = now_ns();
        let received = self.received.load(Ordering::Relaxed);
        let lost_wire = self.lost_wire.load(Ordering::Relaxed);
        let elapsed_s = ((now.saturating_sub(self.started_ns)) as f64) / 1e9;
        let denom = received + lost_wire;
        let streams = self
            .streams
            .lock()
            .unwrap()
            .iter()
            .map(|(id, s)| StreamSnapshot {
                publisher_id: *id,
                received: s.received,
                lost: s.lost,
                last_counter: s.last_counter,
                first_counter_seen: s.first_counter_seen,
            })
            .collect();
        SubMetricsSnapshot {
            elapsed_s,
            received,
            lost_wire,
            lost_dds_local: self.lost_dds_local.load(Ordering::Relaxed),
            reordered: self.reordered.load(Ordering::Relaxed),
            duplicates: self.duplicates.load(Ordering::Relaxed),
            loss_rate: if denom > 0 { lost_wire as f64 / denom as f64 } else { 0.0 },
            rate_per_s: if elapsed_s > 0.0 { received as f64 / elapsed_s } else { 0.0 },
            streams,
        }
    }

    /// Render the recv / lost history as two-line ASCII sparkline.
    pub fn render_history(&self) -> String {
        let h = self.history.lock().unwrap();
        format!(
            "  recv \u{2502}{}\n  lost \u{2502}{}",
            h.recv.sparkline(),
            h.lost.sparkline(),
        )
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct StreamSnapshot {
    pub publisher_id: u32,
    pub received: u64,
    pub lost: u64,
    pub last_counter: u64,
    pub first_counter_seen: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct SubMetricsSnapshot {
    pub elapsed_s: f64,
    pub received: u64,
    pub lost_wire: u64,
    pub lost_dds_local: u64,
    pub reordered: u64,
    pub duplicates: u64,
    pub loss_rate: f64,
    pub rate_per_s: f64,
    pub streams: Vec<StreamSnapshot>,
}

impl SubMetricsSnapshot {
    pub fn format_line(&self) -> String {
        format!(
            "recv={} lost_wire={} ({:.1}%) lost_dds={} reord={} dup={} rate={:.1}/s uptime={:.1}s",
            self.received,
            self.lost_wire,
            self.loss_rate * 100.0,
            self.lost_dds_local,
            self.reordered,
            self.duplicates,
            self.rate_per_s,
            self.elapsed_s,
        )
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_simple_gap() {
        let m = SubMetrics::new();
        m.on_sample(1, 0);
        m.on_sample(1, 1);
        m.on_sample(1, 2);
        m.on_sample(1, 5); // gap of 2 (3 and 4 lost)
        let s = m.snapshot();
        assert_eq!(s.received, 4);
        assert_eq!(s.lost_wire, 2);
    }

    #[test]
    fn ignores_late_join() {
        let m = SubMetrics::new();
        // Sub joined late — first counter it sees is 100.
        m.on_sample(1, 100);
        m.on_sample(1, 101);
        let s = m.snapshot();
        assert_eq!(s.received, 2);
        assert_eq!(s.lost_wire, 0);
    }

    #[test]
    fn counts_duplicate_and_reorder() {
        let m = SubMetrics::new();
        m.on_sample(1, 0);
        m.on_sample(1, 1);
        m.on_sample(1, 2);
        m.on_sample(1, 2); // duplicate
        m.on_sample(1, 1); // reorder
        let s = m.snapshot();
        assert_eq!(s.duplicates, 1);
        assert_eq!(s.reordered, 1);
        assert_eq!(s.lost_wire, 0);
    }

    #[test]
    fn handles_publisher_restart() {
        let m = SubMetrics::new();
        m.on_sample(1, 50);
        m.on_sample(1, 51);
        m.on_sample(1, 0); // restart: < first_counter_seen → rebaseline
        m.on_sample(1, 1);
        let s = m.snapshot();
        assert_eq!(s.received, 4);
        assert_eq!(s.lost_wire, 0);
        assert_eq!(s.reordered, 0);
    }

    #[test]
    fn multi_publisher_isolated() {
        let m = SubMetrics::new();
        m.on_sample(1, 0);
        m.on_sample(2, 0);
        m.on_sample(1, 1);
        m.on_sample(2, 3); // pub 2 gap of 2
        let s = m.snapshot();
        assert_eq!(s.received, 4);
        assert_eq!(s.lost_wire, 2);
        assert_eq!(s.streams.len(), 2);
    }
}
