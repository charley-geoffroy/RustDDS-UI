//! Lightweight publisher-side counters: how many samples were sent,
//! how long the local `write()` call took, and what the effective rate
//! is over the run.
//!
//! Designed to live next to a `TypedPublisher` as an `Arc<PubMetrics>`,
//! so a heartbeat thread can read it concurrently with publishing.

use std::collections::HashMap;
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
