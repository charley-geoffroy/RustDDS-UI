//! UI-facing DTOs. The DDS-shaped ones come from `dds-backend` (so they
//! are produced by any backend impl); the UI-specific aggregates and the
//! sample wire-format live here.

pub use dds_backend::{EndpointDto, ParticipantDto, TopicDto};
use serde::Serialize;

#[derive(Serialize, Clone, Debug, Default)]
pub struct RegistrySnapshot {
    pub participants: Vec<ParticipantDto>,
    pub topics: Vec<TopicDto>,
    pub writers: Vec<EndpointDto>,
    pub readers: Vec<EndpointDto>,
}

/// One raw CDR sample emitted to the frontend (bytes are pre-hex-encoded
/// here for the JSON wire to the webview).
#[derive(Serialize, Clone, Debug)]
pub struct SampleDto {
    pub topic: String,
    pub recv_ns: u64,
    pub size: usize,
    pub bytes_hex: String,
}

/// Batch of samples emitted to the frontend on each throttle tick.
#[derive(Serialize, Clone, Debug)]
pub struct SampleBatchDto {
    pub topic: String,
    pub samples: Vec<SampleDto>,
    pub received_since_last: u32,
}
