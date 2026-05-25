//! Subscriber demo backed by the RustDDS implementation of `dds-backend`.
//!
//! Mirror of `pub-rustdds`: same pattern, swap backend by
//! changing the two `dds_backend::...` imports.

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

    let (events_tx, _events_rx) = mpsc::channel();
    let backend = RustDdsBackend::start(DOMAIN_ID, events_tx)?;
    let mut subscriber = backend.create_typed_subscriber::<Chatter>(TOPIC_NAME, TYPE_NAME)?;
    let metrics = subscriber.metrics().clone();

    println!(
        "[{} {}] subscriber listening on '{TOPIC_NAME}' on domain {DOMAIN_ID} (Ctrl-C to stop)",
        RustDdsBackend::name(),
        RustDdsBackend::version(),
    );

    // 1Hz heartbeat: stats line + a fresh ASCII sparkline (60s window
    // at 1s resolution). Sub-second jitter doesn't matter — the
    // sparkline buckets are coarse.
    {
        let metrics = metrics.clone();
        let stop = stop.clone();
        thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_secs(1));
                let snap = metrics.snapshot();
                println!("[stats] {}", snap.format_line());
                println!("{}", metrics.render_history());
            }
        });
    }

    while !stop.load(Ordering::Relaxed) {
        match subscriber.take_next(Duration::from_millis(500)) {
            Ok(Some(c)) => {
                let recv_ns = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(0);
                metrics.on_sample(c.publisher_id, c.counter);
                metrics.record_latency(c.stamp_ns, recv_ns);
            }
            Ok(None) => continue,
            // EINTR (Ctrl-C interrupting the poll) or any other reader
            // error — log and break so the final report still prints.
            Err(e) => {
                eprintln!("[take_next ended] {e:?}");
                break;
            }
        }
    }

    subscriber.drain_status();
    let snap = metrics.snapshot();
    println!("\n[final] {}", snap.to_json_pretty());
    println!("\n[viz]\n{}", metrics.render_history());
    Ok(())
}
