//! Publisher demo backed by the RustDDS implementation of `dds-backend`.
//!
//! To switch backend, change the two imports below to the equivalent
//! HDDS / future-backend types — the rest of `main` stays identical.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use dds_backend::rustdds::RustDdsBackend;
use dds_backend::DdsBackend;
use demo_msgs::{Chatter, DOMAIN_ID, TOPIC_NAME, TYPE_NAME};

fn main() -> Result<()> {
    env_logger::init();

    let stop = Arc::new(AtomicBool::new(false));
    {
        let stop = stop.clone();
        ctrlc::set_handler(move || stop.store(true, Ordering::Relaxed))?;
    }

    // Discovery events go to a channel we don't drain (the demo doesn't
    // need them). They'll be silently dropped once it fills, which is OK
    // for a small example. The inspector UI drains them properly.
    let (events_tx, _events_rx) = mpsc::channel();

    let backend = RustDdsBackend::start(DOMAIN_ID, events_tx)?;
    let publisher = backend.create_typed_publisher::<Chatter>(TOPIC_NAME, TYPE_NAME)?;

    let publisher_id = std::process::id();
    let mut counter: u64 = 0;

    println!(
        "[{} {}] publisher {publisher_id:08x} writing to '{TOPIC_NAME}' on domain {DOMAIN_ID} (Ctrl-C to stop)",
        RustDdsBackend::name(),
        RustDdsBackend::version(),
    );

    while !stop.load(Ordering::Relaxed) {
        let stamp_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let msg = Chatter {
            publisher_id,
            counter,
            stamp_ns,
            text: format!("hello #{counter}"),
        };
        publisher.write(msg)?;
        println!("sent #{counter}");
        counter += 1;
        thread::sleep(Duration::from_secs(1));
    }
    Ok(())
}
