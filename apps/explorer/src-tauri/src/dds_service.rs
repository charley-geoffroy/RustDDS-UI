//! High-level service driven by Tauri commands. Generic over the DDS
//! backend so we can swap RustDDS for HDDS (or an RTPS-sniff mode) without
//! changing the command surface or the frontend.
//!
//! Threading model:
//! ```
//! [Backend's discovery thread] ──BackendEvent──▶ [event_drain thread]
//!                                                     │
//!                                                     ▼
//!                                              registry + Tauri emit
//!
//! [Backend's per-topic sub thread] ──BackendSample──▶ [sample_drain thread]
//!                                                          │
//!                                                          ▼
//!                                                  batch + throttle (~30 Hz)
//!                                                          │
//!                                                          ▼
//!                                                     Tauri emit
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use tauri::{AppHandle, Emitter};

use dds_backend::{BackendEvent, BackendSample, DdsBackend};

use crate::dto::{RegistrySnapshot, SampleBatchDto, SampleDto};
use crate::registry::{new_shared, SharedRegistry};

/// Public API consumed by Tauri commands. Substituting this with a
/// `RemoteDdsService` later is the path to a daemon mode.
pub trait DdsService: Send + Sync + 'static {
    fn snapshot(&self) -> RegistrySnapshot;
    fn subscribe(&self, topic_name: &str, type_name: &str) -> Result<()>;
    fn unsubscribe(&self, topic_name: &str) -> Result<()>;
}

struct SubscriptionHandle {
    stop: Arc<AtomicBool>,
}

pub struct EmbeddedDdsService<B: DdsBackend> {
    backend: Arc<B>,
    registry: SharedRegistry,
    app: AppHandle,
    subscriptions: Mutex<HashMap<String, SubscriptionHandle>>,
}

impl<B: DdsBackend> EmbeddedDdsService<B> {
    pub fn start(domain_id: u16, app: AppHandle) -> Result<Self> {
        let (events_tx, events_rx) = channel();
        let backend = Arc::new(
            B::start(domain_id, events_tx).context("backend start failed")?,
        );
        let registry = new_shared();
        spawn_event_drain(events_rx, registry.clone(), app.clone());
        Ok(Self {
            backend,
            registry,
            app,
            subscriptions: Mutex::new(HashMap::new()),
        })
    }
}

impl<B: DdsBackend> DdsService for EmbeddedDdsService<B> {
    fn snapshot(&self) -> RegistrySnapshot {
        self.registry.lock().unwrap().snapshot()
    }

    fn subscribe(&self, topic_name: &str, type_name: &str) -> Result<()> {
        let mut subs = self.subscriptions.lock().unwrap();
        if subs.contains_key(topic_name) {
            return Ok(()); // idempotent
        }
        let stop = Arc::new(AtomicBool::new(false));
        let (samples_tx, samples_rx) = channel();
        self.backend.spawn_raw_subscription(
            topic_name.to_string(),
            type_name.to_string(),
            samples_tx,
            stop.clone(),
        )?;
        spawn_sample_drain(
            samples_rx,
            topic_name.to_string(),
            self.app.clone(),
            stop.clone(),
        );
        subs.insert(topic_name.to_string(), SubscriptionHandle { stop });
        Ok(())
    }

    fn unsubscribe(&self, topic_name: &str) -> Result<()> {
        if let Some(handle) = self.subscriptions.lock().unwrap().remove(topic_name) {
            handle.stop.store(true, Ordering::Relaxed);
        }
        Ok(())
    }
}

// ============================================================================
// Drain threads
// ============================================================================

fn spawn_event_drain(rx: Receiver<BackendEvent>, registry: SharedRegistry, app: AppHandle) {
    thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            apply_event(event, &registry, &app);
        }
    });
}

fn apply_event(event: BackendEvent, registry: &SharedRegistry, app: &AppHandle) {
    use BackendEvent as E;
    match event {
        E::ParticipantAdded(p) => {
            registry.lock().unwrap().add_participant(p.clone());
            let _ = app.emit("dds:participant_added", &p);
        }
        E::ParticipantRemoved(g) => {
            registry.lock().unwrap().remove_participant(&g);
            let _ = app.emit("dds:participant_removed", &g);
        }
        E::TopicAdded(t) => {
            registry.lock().unwrap().add_topic(t.clone());
            let _ = app.emit("dds:topic_added", &t);
        }
        E::TopicRemoved(n) => {
            registry.lock().unwrap().remove_topic(&n);
            let _ = app.emit("dds:topic_removed", &n);
        }
        E::WriterAdded(e) => {
            registry.lock().unwrap().add_writer(e.clone());
            let _ = app.emit("dds:writer_added", &e);
        }
        E::WriterRemoved(g) => {
            registry.lock().unwrap().remove_writer(&g);
            let _ = app.emit("dds:writer_removed", &g);
        }
        E::ReaderAdded(e) => {
            registry.lock().unwrap().add_reader(e.clone());
            let _ = app.emit("dds:reader_added", &e);
        }
        E::ReaderRemoved(g) => {
            registry.lock().unwrap().remove_reader(&g);
            let _ = app.emit("dds:reader_removed", &g);
        }
    }
}

const THROTTLE_MS: u64 = 33; // ~30 Hz max emission to the UI

fn spawn_sample_drain(
    rx: Receiver<BackendSample>,
    topic_name: String,
    app: AppHandle,
    stop: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        let throttle = Duration::from_millis(THROTTLE_MS);
        let mut batch: Vec<SampleDto> = Vec::with_capacity(64);
        let mut received_since_last: u32 = 0;
        let mut last_emit = Instant::now();

        loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            // Sleep up to one throttle tick waiting for the next sample.
            let timeout = throttle.saturating_sub(last_emit.elapsed());
            match rx.recv_timeout(timeout) {
                Ok(sample) => {
                    received_since_last = received_since_last.saturating_add(1);
                    batch.push(SampleDto {
                        topic: sample.topic,
                        recv_ns: sample.recv_ns,
                        size: sample.bytes.len(),
                        bytes_hex: hex_str(&sample.bytes),
                    });
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }

            if last_emit.elapsed() >= throttle
                && (!batch.is_empty() || received_since_last > 0)
            {
                let payload = SampleBatchDto {
                    topic: topic_name.clone(),
                    samples: std::mem::take(&mut batch),
                    received_since_last: std::mem::take(&mut received_since_last),
                };
                let _ = app.emit("dds:samples", &payload);
                last_emit = Instant::now();
            }
        }
    });
}

fn hex_str(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}
