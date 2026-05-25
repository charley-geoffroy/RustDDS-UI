//! Subscriber demo backed by the HDDS implementation of `dds-backend`.
//!
//! Structural twin of `sub-rustdds` — only the two
//! `dds_backend::...` imports differ. Today the HDDS backend is a stub
//! that returns no samples; the binary still runs to demonstrate the
//! parity with the RustDDS demo.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

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
    let mut subscriber =
        backend.create_typed_subscriber::<Chatter>(TOPIC_NAME, TYPE_NAME)?;

    println!(
        "[{} {}] subscriber listening on '{TOPIC_NAME}' on domain {DOMAIN_ID} (Ctrl-C to stop)",
        HddsBackend::name(),
        HddsBackend::version(),
    );

    while !stop.load(Ordering::Relaxed) {
        match subscriber.take_next(Duration::from_millis(500))? {
            Some(c) => println!(
                "recv pub={:08x} #{:<5} stamp={} text=\"{}\"",
                c.publisher_id, c.counter, c.stamp_ns, c.text
            ),
            None => continue,
        }
    }
    Ok(())
}
