//! Publisher demo backed by the HDDS implementation of `dds-backend`.
//!
//! **Structural twin of `pub-rustdds`** — only the two
//! `dds_backend::...` imports differ. This file is the proof that the
//! demo code is backend-agnostic at the call-site level.
//!
//! Today the HDDS backend is a stub that prints "would publish" instead
//! of emitting traffic. When HDDS lands on crates.io and `dds-backend`
//! gets its real impl, this binary starts publishing for real with no
//! source change here.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use dds_backend::hdds::HddsBackend;
use dds_backend::DdsBackend;
use demo_msgs::{Chatter, DOMAIN_ID, TOPIC_NAME, TYPE_NAME};

fn main() -> Result<()> {
    env_logger::init();

    let stop = Arc::new(AtomicBool::new(false));
    {
        let stop = stop.clone();
        ctrlc::set_handler(move || stop.store(true, Ordering::Relaxed))?;
    }

    let (events_tx, events_rx) = mpsc::channel();
    thread::spawn(move || while events_rx.recv().is_ok() {});
    let backend = HddsBackend::start(DOMAIN_ID, events_tx)?;
    let publisher = backend.create_typed_publisher::<Chatter>(TOPIC_NAME, TYPE_NAME)?;

    let publisher_id = std::process::id();
    let mut counter: u64 = 0;

    println!(
        "[{} {}] publisher {publisher_id:08x} writing to '{TOPIC_NAME}' on domain {DOMAIN_ID} (Ctrl-C to stop)",
        HddsBackend::name(),
        HddsBackend::version(),
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
            padding: Vec::new(),
        };
        publisher.write(msg)?;
        println!("sent #{counter}");
        counter += 1;
        thread::sleep(Duration::from_secs(1));
    }
    Ok(())
}
