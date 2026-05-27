//! Lightweight publisher-side counters: how many samples were sent,
//! how long the local `write()` call took, and what the effective rate
//! is over the run.
//!
//! Designed to live next to a `TypedPublisher` as an `Arc<PubMetrics>`,
//! so a heartbeat thread can read it concurrently with publishing.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;

/// Concurrent, lock-free publisher counters.
///
/// `started_ns` is a SystemTime nanos timestamp for human-readable
/// JSON. `started_at` is a monotonic `Instant` used for elapsed_s /
/// rate so a wall-clock jump (NTP, manual time change) during the run
/// doesn't poison the rate calculation.
pub struct PubMetrics {
    started_ns: AtomicU64,
    started_at: Mutex<Instant>,
    sent: AtomicU64,
    errors: AtomicU64,
    last_send_ns: AtomicU64,
    write_ns_total: AtomicU64,
    write_ns_max: AtomicU64,
}

impl PubMetrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            started_ns: AtomicU64::new(now_ns()),
            started_at: Mutex::new(Instant::now()),
            sent: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            last_send_ns: AtomicU64::new(0),
            write_ns_total: AtomicU64::new(0),
            write_ns_max: AtomicU64::new(0),
        })
    }

    /// Zero every counter and re-stamp the start time. Used at the
    /// warmup boundary so steady-state numbers aren't polluted by
    /// discovery / cold-cache effects.
    ///
    /// All counters are zeroed under the `started_at` mutex so a
    /// concurrent `snapshot()` observes either the pre-reset state or
    /// the post-reset state — never a torn mix.
    pub fn reset(&self) {
        let mut g = self.started_at.lock().unwrap();
        self.sent.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
        self.last_send_ns.store(0, Ordering::Relaxed);
        self.write_ns_total.store(0, Ordering::Relaxed);
        self.write_ns_max.store(0, Ordering::Relaxed);
        *g = Instant::now();
        self.started_ns.store(now_ns(), Ordering::Relaxed);
    }

    /// Swap the running max with 0 and return the previous value. Used
    /// by the bench heartbeat to read a per-window max (as opposed to
    /// the cumulative max that would dominate the rest of the run).
    pub fn take_write_ns_max(&self) -> u64 {
        self.write_ns_max.swap(0, Ordering::Relaxed)
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
        let g = self.started_at.lock().unwrap();
        let started = self.started_ns.load(Ordering::Relaxed);
        let elapsed_s = g.elapsed().as_secs_f64();
        let sent = self.sent.load(Ordering::Relaxed);
        let total_ns = self.write_ns_total.load(Ordering::Relaxed);
        let max_ns = self.write_ns_max.load(Ordering::Relaxed);
        let errors = self.errors.load(Ordering::Relaxed);
        let last_send_ns = self.last_send_ns.load(Ordering::Relaxed);
        drop(g);
        PubMetricsSnapshot {
            started_ns: started,
            now_ns: now,
            elapsed_s,
            sent,
            errors,
            last_send_ns,
            rate_per_s: if elapsed_s > 0.0 { sent as f64 / elapsed_s } else { 0.0 },
            write_ns_avg: if sent > 0 { total_ns / sent } else { 0 },
            write_ns_total: total_ns,
            write_ns_max: max_ns,
        }
    }
}

/// JSON-serializable view of `PubMetrics` at one instant.
///
/// `rate_per_s`, `write_ns_avg`, and `write_ns_max` are all cumulative
/// since the last `reset()`. For per-window stats, callers compute
/// deltas from successive snapshots using `sent` and `write_ns_total`,
/// and read a per-window max with `PubMetrics::take_write_ns_max`.
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
    /// Total nanoseconds spent in `writer.write()` since the last
    /// reset. Heartbeats diff this between ticks to get per-window
    /// write latency.
    pub write_ns_total: u64,
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

    /// Clamp every open session's `sent_at_match` to zero. Use right
    /// after `PubMetrics::reset()` so `samples_during` still measures
    /// "samples sent while matched" in the post-reset window.
    pub fn rebase_sent_counter(&self) {
        let mut inner = self.inner.lock().unwrap();
        for s in inner.open.values_mut() {
            s.sent_at_match = 0;
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
///
/// Time advancement is decoupled from recording so multiple buckets can
/// share an axis — see `SubHistory::advance_to`.
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

    /// Advance the rolling window so the trailing bucket covers `now`.
    /// Callers should advance all sibling buckets to the same `now`
    /// before reading or writing to keep the rows time-aligned.
    fn advance_to(&mut self, now: u64) {
        if self.buckets.is_empty() {
            self.current_start_s = now;
            self.buckets.push_back(0);
            return;
        }
        // After a long quiet period, fast-forward instead of pushing
        // thousands of empty buckets one at a time.
        let gap = now.saturating_sub(self.current_start_s);
        if gap > HISTORY_CAP as u64 {
            self.buckets.clear();
            self.current_start_s = now;
            self.buckets.push_back(0);
            return;
        }
        while now >= self.current_start_s + BUCKET_SECS {
            self.current_start_s += BUCKET_SECS;
            self.buckets.push_back(0);
            if self.buckets.len() > HISTORY_CAP {
                self.buckets.pop_front();
            }
        }
    }

    /// Add to the most recent bucket. Caller must have advanced first.
    fn add_count(&mut self, count: u64) {
        if let Some(last) = self.buckets.back_mut() {
            *last += count;
        }
    }

    /// Track the max value seen in the most recent bucket. Caller must
    /// have advanced first.
    fn add_max(&mut self, value: u64) {
        if let Some(last) = self.buckets.back_mut() {
            *last = (*last).max(value);
        }
    }

    fn reset(&mut self) {
        self.buckets.clear();
        self.current_start_s = 0;
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
// LatencyHisto — log-2 bucket histogram for end-to-end latency
// ============================================================================

/// Number of power-of-two buckets. Bucket `i` covers
/// `[2^i, 2^(i+1))` nanoseconds, so 40 buckets reach ~18 minutes
/// per sample — far beyond anything we'll ever see.
const LAT_BUCKETS: usize = 40;

/// Lock-free latency histogram. `record(ns)` is one atomic add per
/// scalar (count/sum/min/max) plus one bucket bump. `snapshot()`
/// computes percentiles from the buckets — reported as the geometric
/// mean of the matching log-2 bucket, so the worst-case error is ±√2
/// (about ±41%) instead of the 2× overestimate you'd get from the
/// upper edge.
pub struct LatencyHisto {
    count: AtomicU64,
    sum_ns: AtomicU64,
    min_ns: AtomicU64,
    max_ns: AtomicU64,
    buckets: [AtomicU64; LAT_BUCKETS],
}

impl LatencyHisto {
    pub fn new() -> Self {
        const Z: AtomicU64 = AtomicU64::new(0);
        Self {
            count: AtomicU64::new(0),
            sum_ns: AtomicU64::new(0),
            min_ns: AtomicU64::new(u64::MAX),
            max_ns: AtomicU64::new(0),
            buckets: [Z; LAT_BUCKETS],
        }
    }

    /// Record one latency sample. Caller is responsible for filtering
    /// clock-skew (negative) cases; this method does nothing if `ns == 0`.
    pub fn record(&self, ns: u64) {
        if ns == 0 {
            return;
        }
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum_ns.fetch_add(ns, Ordering::Relaxed);
        self.min_ns.fetch_min(ns, Ordering::Relaxed);
        self.max_ns.fetch_max(ns, Ordering::Relaxed);
        let bucket = (64u32 - ns.leading_zeros()).saturating_sub(1) as usize;
        self.buckets[bucket.min(LAT_BUCKETS - 1)].fetch_add(1, Ordering::Relaxed);
    }

    pub fn reset(&self) {
        self.count.store(0, Ordering::Relaxed);
        self.sum_ns.store(0, Ordering::Relaxed);
        self.min_ns.store(u64::MAX, Ordering::Relaxed);
        self.max_ns.store(0, Ordering::Relaxed);
        for b in &self.buckets {
            b.store(0, Ordering::Relaxed);
        }
    }

    pub fn snapshot(&self) -> LatencySnapshot {
        let count = self.count.load(Ordering::Relaxed);
        let sum = self.sum_ns.load(Ordering::Relaxed);
        let max = self.max_ns.load(Ordering::Relaxed);
        LatencySnapshot {
            count,
            min_ns: if count == 0 { 0 } else { self.min_ns.load(Ordering::Relaxed) },
            avg_ns: if count > 0 { sum / count } else { 0 },
            max_ns: max,
            p50_ns: self.percentile(0.50),
            p95_ns: self.percentile(0.95),
            p99_ns: self.percentile(0.99),
        }
    }

    fn percentile(&self, p: f64) -> u64 {
        let total = self.count.load(Ordering::Relaxed);
        if total == 0 {
            return 0;
        }
        let threshold = ((total as f64) * p).ceil() as u64;
        let mut acc = 0u64;
        for (i, b) in self.buckets.iter().enumerate() {
            acc += b.load(Ordering::Relaxed);
            if acc >= threshold {
                // Bucket i covers [2^i, 2^(i+1)). Report the geometric
                // mean (≈ lower * sqrt(2)) so the estimate sits in the
                // middle of the bucket on a log scale, instead of
                // systematically overshooting by 2× at the upper edge.
                let lower = 1u64 << i.min(62);
                return lower + (lower >> 1);
            }
        }
        self.max_ns.load(Ordering::Relaxed)
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct LatencySnapshot {
    pub count: u64,
    pub min_ns: u64,
    pub avg_ns: u64,
    pub max_ns: u64,
    pub p50_ns: u64,
    pub p95_ns: u64,
    pub p99_ns: u64,
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
    started_ns: AtomicU64,
    started_at: Mutex<Instant>,
    received: AtomicU64,
    /// Wire loss detected by counter gaps from the published stream.
    lost_wire: AtomicU64,
    reordered: AtomicU64,
    duplicates: AtomicU64,
    /// Loss reported by the local DDS stack via `DataReaderStatus::SampleLost`.
    lost_dds_local: AtomicU64,
    /// Samples where `recv_ns < stamp_ns` — clocks skewed, latency
    /// would be negative, sample skipped from the histogram.
    clock_skew_skipped: AtomicU64,
    streams: Mutex<HashMap<u32, StreamState>>,
    history: Mutex<SubHistory>,
    /// Cumulative latency since the last `reset()`. Used by the live
    /// stdout heartbeat and the final JSON dump at exit.
    latency: LatencyHisto,
    /// Per-tick latency window. Both `latency` and `latency_window`
    /// receive each sample; the bench heartbeat snapshots+resets this
    /// one every second so each CSV row reflects a real 1-second
    /// window instead of a cumulative average.
    latency_window: LatencyHisto,
}

struct SubHistory {
    recv: TimeBuckets,
    lost: TimeBuckets,
    /// Max latency observed per bucket, in microseconds.
    lat_us: TimeBuckets,
}

impl SubHistory {
    /// Advance all three rows to the same `now`, so the sparklines stay
    /// time-aligned even when one signal is quiet.
    fn advance_to(&mut self, now: u64) {
        self.recv.advance_to(now);
        self.lost.advance_to(now);
        self.lat_us.advance_to(now);
    }
}

impl SubMetrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            started_ns: AtomicU64::new(now_ns()),
            started_at: Mutex::new(Instant::now()),
            received: AtomicU64::new(0),
            lost_wire: AtomicU64::new(0),
            reordered: AtomicU64::new(0),
            duplicates: AtomicU64::new(0),
            lost_dds_local: AtomicU64::new(0),
            clock_skew_skipped: AtomicU64::new(0),
            streams: Mutex::new(HashMap::new()),
            history: Mutex::new(SubHistory {
                recv: TimeBuckets::new(),
                lost: TimeBuckets::new(),
                lat_us: TimeBuckets::new(),
            }),
            latency: LatencyHisto::new(),
            latency_window: LatencyHisto::new(),
        })
    }

    /// Zero every counter, drop per-publisher state, clear the
    /// sparkline buffers and the latency histograms, and re-stamp the
    /// start time. Used at the warmup boundary.
    ///
    /// Counters are zeroed under the `started_at` mutex so a
    /// concurrent `snapshot()` observes either the pre-reset state or
    /// the post-reset state — never a torn mix.
    pub fn reset(&self) {
        let mut g = self.started_at.lock().unwrap();
        self.received.store(0, Ordering::Relaxed);
        self.lost_wire.store(0, Ordering::Relaxed);
        self.reordered.store(0, Ordering::Relaxed);
        self.duplicates.store(0, Ordering::Relaxed);
        self.lost_dds_local.store(0, Ordering::Relaxed);
        self.clock_skew_skipped.store(0, Ordering::Relaxed);
        self.streams.lock().unwrap().clear();
        let mut h = self.history.lock().unwrap();
        h.recv.reset();
        h.lost.reset();
        h.lat_us.reset();
        drop(h);
        self.latency.reset();
        self.latency_window.reset();
        *g = Instant::now();
        self.started_ns.store(now_ns(), Ordering::Relaxed);
    }

    /// Snapshot the per-window latency histogram and zero it. Called
    /// once per CSV tick so each row reflects the latency distribution
    /// over the past ~1 second instead of since reset.
    pub fn take_latency_window(&self) -> LatencySnapshot {
        let snap = self.latency_window.snapshot();
        self.latency_window.reset();
        snap
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
            self.bump_history(1, 0);
            return;
        }

        if counter == state.last_counter + 1 {
            state.last_counter = counter;
            state.received += 1;
            self.received.fetch_add(1, Ordering::Relaxed);
            self.bump_history(1, 0);
        } else if counter > state.last_counter + 1 {
            let gap = counter - state.last_counter - 1;
            state.lost += gap;
            state.last_counter = counter;
            state.received += 1;
            self.lost_wire.fetch_add(gap, Ordering::Relaxed);
            self.received.fetch_add(1, Ordering::Relaxed);
            self.bump_history(1, gap);
        } else if counter == state.last_counter {
            self.duplicates.fetch_add(1, Ordering::Relaxed);
        } else if counter < state.first_counter_seen {
            // Publisher restarted with a reset counter — rebaseline.
            state.last_counter = counter;
            state.first_counter_seen = counter;
            state.received += 1;
            self.received.fetch_add(1, Ordering::Relaxed);
            self.bump_history(1, 0);
        } else {
            self.reordered.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Update the recv / lost rows, keeping all three history rows
    /// aligned to the same `now`.
    fn bump_history(&self, recv: u64, lost: u64) {
        let mut h = self.history.lock().unwrap();
        let now = now_s();
        h.advance_to(now);
        if recv > 0 {
            h.recv.add_count(recv);
        }
        if lost > 0 {
            h.lost.add_count(lost);
        }
    }

    pub fn record_sample_lost_dds(&self, n: u64) {
        self.lost_dds_local.fetch_add(n, Ordering::Relaxed);
    }

    /// Record one end-to-end latency observation. Pass `recv_ns` and
    /// the `stamp_ns` the publisher embedded in the message. If clocks
    /// are skewed (recv < stamp) the sample is counted in
    /// `clock_skew_skipped` and dropped from the histogram.
    pub fn record_latency(&self, stamp_ns: u64, recv_ns: u64) {
        if recv_ns <= stamp_ns {
            self.clock_skew_skipped.fetch_add(1, Ordering::Relaxed);
            return;
        }
        let ns = recv_ns - stamp_ns;
        self.latency.record(ns);
        self.latency_window.record(ns);
        let mut h = self.history.lock().unwrap();
        h.advance_to(now_s());
        h.lat_us.add_max(ns / 1_000);
    }

    pub fn snapshot(&self) -> SubMetricsSnapshot {
        let g = self.started_at.lock().unwrap();
        let elapsed_s = g.elapsed().as_secs_f64();
        let received = self.received.load(Ordering::Relaxed);
        let lost_wire = self.lost_wire.load(Ordering::Relaxed);
        drop(g);
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
            clock_skew_skipped: self.clock_skew_skipped.load(Ordering::Relaxed),
            loss_rate: if denom > 0 { lost_wire as f64 / denom as f64 } else { 0.0 },
            rate_per_s: if elapsed_s > 0.0 { received as f64 / elapsed_s } else { 0.0 },
            latency: self.latency.snapshot(),
            streams,
        }
    }

    /// Render the recv / lost / latency history as a three-line ASCII
    /// sparkline. Latency row shows max-per-bucket in microseconds.
    /// Advances all rows to "now" first so trailing quiet time shows
    /// up as empty buckets on the right of every row.
    pub fn render_history(&self) -> String {
        let mut h = self.history.lock().unwrap();
        h.advance_to(now_s());
        format!(
            "  recv \u{2502}{}\n  lost \u{2502}{}\n  lat  \u{2502}{}",
            h.recv.sparkline(),
            h.lost.sparkline(),
            h.lat_us.sparkline(),
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
    pub clock_skew_skipped: u64,
    pub loss_rate: f64,
    pub rate_per_s: f64,
    pub latency: LatencySnapshot,
    pub streams: Vec<StreamSnapshot>,
}

impl SubMetricsSnapshot {
    pub fn format_line(&self) -> String {
        let lat = &self.latency;
        let lat_str = if lat.count == 0 {
            "lat=n/a".to_string()
        } else {
            format!(
                "lat p50={}µs p95={}µs max={}µs",
                lat.p50_ns / 1_000,
                lat.p95_ns / 1_000,
                lat.max_ns / 1_000,
            )
        };
        format!(
            "recv={} lost_wire={} ({:.1}%) lost_dds={} reord={} dup={} {} rate={:.1}/s uptime={:.1}s",
            self.received,
            self.lost_wire,
            self.loss_rate * 100.0,
            self.lost_dds_local,
            self.reordered,
            self.duplicates,
            lat_str,
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
    fn latency_histogram_basic() {
        let h = LatencyHisto::new();
        // 100 fast samples (1µs each)
        for _ in 0..100 {
            h.record(1_000);
        }
        // 5 slow samples (1ms each)
        for _ in 0..5 {
            h.record(1_000_000);
        }
        let s = h.snapshot();
        assert_eq!(s.count, 105);
        assert!(s.min_ns <= 1_000);
        assert!(s.max_ns >= 1_000_000);
        // p50 lands in the 1µs bucket (around 1-2µs)
        assert!(s.p50_ns < 10_000, "p50 was {}ns", s.p50_ns);
        // p99 lands in the 1ms bucket
        assert!(s.p99_ns >= 524_288, "p99 was {}ns", s.p99_ns);
    }

    #[test]
    fn clock_skew_is_skipped() {
        let m = SubMetrics::new();
        // recv before stamp → clocks skewed
        m.record_latency(/* stamp */ 1000, /* recv */ 500);
        // normal sample
        m.record_latency(/* stamp */ 1000, /* recv */ 2000);
        let s = m.snapshot();
        assert_eq!(s.clock_skew_skipped, 1);
        assert_eq!(s.latency.count, 1);
        assert_eq!(s.latency.min_ns, 1000);
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
