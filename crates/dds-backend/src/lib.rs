//! `dds-backend` — pluggable DDS abstraction shared by the inspector UI
//! and the example pub/sub binaries.
//!
//! The crate is split in two layers:
//!
//! 1. **Neutral types & inspection trait** (this module) — used by code
//!    that wants to be backend-agnostic, like the inspector UI.
//! 2. **Per-backend modules** ([`rustdds`], [`hdds`]) — concrete impls
//!    behind Cargo features. They expose the `DdsBackend` trait *and*
//!    backend-specific typed pub/sub helpers used by the example
//!    binaries.
//!
//! Adding a new backend = adding a new module here that implements
//! `DdsBackend` and (optionally) typed helpers.

use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use serde::Serialize;

#[cfg(feature = "rustdds-backend")]
pub mod rustdds;

#[cfg(feature = "hdds-backend")]
pub mod hdds;

pub mod metrics;

// ============================================================================
// Neutral DTOs — used by every backend, by the UI, and by the inspection
// trait below. These map 1:1 onto JSON shipped to the webview.
// ============================================================================

#[derive(Serialize, Clone, Debug)]
pub struct ParticipantDto {
    pub guid: String,
    pub entity_name: Option<String>,
    pub vendor_id: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct EndpointDto {
    pub guid: String,
    pub participant_guid: String,
    pub topic_name: String,
    pub type_name: String,
    /// Per-backend QoS payload. Backends serialize their own QoS struct
    /// here; the UI displays the JSON as-is until Phase 3 lands a typed
    /// matcher.
    pub qos: serde_json::Value,
}

#[derive(Serialize, Clone, Debug)]
pub struct TopicDto {
    pub name: String,
    pub type_name: String,
}

// ============================================================================
// Inspection trait
// ============================================================================

#[derive(Debug, Clone)]
pub enum BackendEvent {
    ParticipantAdded(ParticipantDto),
    ParticipantRemoved(String),
    TopicAdded(TopicDto),
    TopicRemoved(String),
    WriterAdded(EndpointDto),
    WriterRemoved(String),
    ReaderAdded(EndpointDto),
    ReaderRemoved(String),
}

#[derive(Debug)]
pub struct BackendSample {
    pub topic: String,
    pub bytes: Vec<u8>,
    pub recv_ns: u64,
}

/// Pluggable DDS implementation, used by inspector-style consumers.
///
/// Concrete backends additionally expose typed pub/sub helpers for the
/// example binaries — those are backend-specific (different libs have
/// different constraints on T) and live on the impl, not on this trait.
pub trait DdsBackend: Send + Sync + 'static {
    /// Human-readable backend identifier shown in the UI.
    fn name() -> &'static str
    where
        Self: Sized;

    /// Pinned version string of the underlying DDS library.
    fn version() -> &'static str
    where
        Self: Sized;

    /// Join `domain_id`, spawn the discovery thread internally, and start
    /// forwarding events through `events_tx`. The backend stops cleanly
    /// when `events_tx` is dropped.
    fn start(domain_id: u16, events_tx: Sender<BackendEvent>) -> anyhow::Result<Self>
    where
        Self: Sized;

    /// Spawn a subscription thread that pushes raw bytes through
    /// `samples_tx` until `stop` flips to `true` (or the channel is
    /// dropped). Used by the inspector — for typed subscribe, use the
    /// per-backend helpers.
    fn spawn_raw_subscription(
        &self,
        topic_name: String,
        type_name: String,
        samples_tx: Sender<BackendSample>,
        stop: Arc<AtomicBool>,
    ) -> anyhow::Result<()>;
}
