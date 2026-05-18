//! HDDS backend — **stub**.
//!
//! HDDS ([git.hdds.io/hdds/hdds][1]) is another pure-Rust DDS impl with
//! XTypes 1.3, DDS-Security 1.1, and multi-language SDKs. At the time of
//! writing it isn't on crates.io yet (pre-release), so this module is a
//! structural placeholder that:
//!
//! - lets the project compile with the `hdds-backend` feature on,
//! - lets the example binaries demonstrate "swap backend = swap one
//!   import" without needing the real HDDS dependency,
//! - prints clearly that no real DDS traffic happens here.
//!
//! When HDDS lands publicly, the body of each method below gets replaced
//! by a real implementation against its API. The trait surface and the
//! call sites in the demos / UI don't change.
//!
//! [1]: https://git.hdds.io

use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};

use super::{BackendEvent, BackendSample, DdsBackend};

pub struct HddsBackend {
    domain_id: u16,
}

impl HddsBackend {
    /// Returns the domain this stub thinks it joined.
    pub fn domain_id(&self) -> u16 {
        self.domain_id
    }

    /// Stub typed publisher (no traffic emitted).
    pub fn create_typed_publisher<T: Serialize + 'static>(
        &self,
        topic_name: &str,
        type_name: &str,
    ) -> Result<StubPublisher<T>> {
        eprintln!(
            "[hdds stub] would create typed publisher on '{topic_name}' (type='{type_name}')"
        );
        Ok(StubPublisher {
            topic: topic_name.to_string(),
            _phantom: std::marker::PhantomData,
        })
    }

    /// Stub typed subscriber (no traffic received).
    pub fn create_typed_subscriber<T: DeserializeOwned + 'static>(
        &self,
        topic_name: &str,
        type_name: &str,
    ) -> Result<StubSubscriber<T>> {
        eprintln!(
            "[hdds stub] would create typed subscriber on '{topic_name}' (type='{type_name}')"
        );
        Ok(StubSubscriber {
            topic: topic_name.to_string(),
            _phantom: std::marker::PhantomData,
        })
    }
}

impl DdsBackend for HddsBackend {
    fn name() -> &'static str {
        "hdds (stub)"
    }

    fn version() -> &'static str {
        "0.0.0-stub"
    }

    fn start(domain_id: u16, _events_tx: Sender<BackendEvent>) -> Result<Self> {
        eprintln!(
            "[hdds stub] start(domain_id={domain_id}) — no participant created, no traffic"
        );
        eprintln!("[hdds stub] swap this module for a real HDDS impl when the crate ships");
        Ok(Self { domain_id })
    }

    fn spawn_raw_subscription(
        &self,
        topic_name: String,
        type_name: String,
        _samples_tx: Sender<BackendSample>,
        _stop: Arc<AtomicBool>,
    ) -> Result<()> {
        eprintln!(
            "[hdds stub] spawn_raw_subscription('{topic_name}', type='{type_name}') — no-op"
        );
        Ok(())
    }
}

pub struct StubPublisher<T> {
    topic: String,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Serialize> StubPublisher<T> {
    pub fn write(&self, _value: T) -> Result<()> {
        eprintln!(
            "[hdds stub] publisher on '{}' would write a sample (no-op)",
            self.topic
        );
        Ok(())
    }
}

pub struct StubSubscriber<T> {
    topic: String,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: DeserializeOwned> StubSubscriber<T> {
    pub fn take_next(&mut self, _timeout: std::time::Duration) -> Result<Option<T>> {
        // No traffic in the stub; block for a moment so the demo loop
        // doesn't spin.
        std::thread::sleep(std::time::Duration::from_millis(500));
        Ok(None)
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }
}
