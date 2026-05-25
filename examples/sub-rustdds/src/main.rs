//! Subscriber demo / bench backed by the RustDDS implementation of
//! `dds-backend`. Mirror of `pub-rustdds` — flags for QoS, warmup,
//! duration cap.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use clap::{Parser, ValueEnum};
use dds_backend::rustdds::RustDdsBackend;
use dds_backend::DdsBackend;
use demo_msgs::{Chatter, DOMAIN_ID, TOPIC_NAME, TYPE_NAME};
use rustdds::policy::{Durability, History, Reliability};
use rustdds::{QosPolicies, QosPolicyBuilder};

#[derive(Parser, Debug)]
#[command(about = "DDS subscriber bench (RustDDS backend)")]
struct Cli {
    /// Stop after this many seconds (0 = run until Ctrl-C).
    #[arg(long, default_value_t = 0)]
    duration: u64,

    /// Discard the first N seconds of metrics. Samples are still
    /// received during this window — only the counters are zeroed at
    /// the end. Useful to skip discovery and cold-cache effects.
    #[arg(long, default_value_t = 0)]
    warmup: u64,

    /// DDS domain ID.
    #[arg(long, default_value_t = DOMAIN_ID)]
    domain: u16,

    /// Topic name.
    #[arg(long, default_value_t = TOPIC_NAME.to_string())]
    topic: String,

    /// Reliability QoS.
    #[arg(long, value_enum, default_value_t = ReliabilityArg::Reliable)]
    reliability: ReliabilityArg,

    /// History KeepLast depth.
    #[arg(long, default_value_t = 100)]
    history_depth: i32,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ReliabilityArg {
    Reliable,
    BestEffort,
}

fn build_qos(cli: &Cli) -> QosPolicies {
    let reliability = match cli.reliability {
        ReliabilityArg::Reliable => Reliability::Reliable {
            max_blocking_time: rustdds::Duration::from_millis(100),
        },
        ReliabilityArg::BestEffort => Reliability::BestEffort,
    };
    QosPolicyBuilder::new()
        .reliability(reliability)
        .durability(Durability::Volatile)
        .history(History::KeepLast {
            depth: cli.history_depth,
        })
        .build()
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    let stop = Arc::new(AtomicBool::new(false));
    {
        let stop = stop.clone();
        ctrlc::set_handler(move || stop.store(true, Ordering::Relaxed))?;
    }

    let (events_tx, events_rx) = mpsc::channel();
    thread::spawn(move || while events_rx.recv().is_ok() {});

    let backend = RustDdsBackend::start(cli.domain, events_tx)?;
    let qos = build_qos(&cli);
    let mut subscriber =
        backend.create_typed_subscriber_with_qos::<Chatter>(&cli.topic, TYPE_NAME, qos)?;
    let metrics = subscriber.metrics().clone();

    println!(
        "[{} {}] subscriber listening on '{}' on domain {} \
         (reliability={:?} duration={}s warmup={}s)",
        RustDdsBackend::name(),
        RustDdsBackend::version(),
        cli.topic,
        cli.domain,
        cli.reliability,
        cli.duration,
        cli.warmup,
    );

    // 1 Hz heartbeat — stats + sparkline.
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

    let stop_at = (cli.duration > 0).then(|| Instant::now() + Duration::from_secs(cli.duration));
    let warmup_at = (cli.warmup > 0).then(|| Instant::now() + Duration::from_secs(cli.warmup));
    let mut warmed_up = warmup_at.is_none();

    while !stop.load(Ordering::Relaxed) {
        if matches!(stop_at, Some(d) if Instant::now() >= d) {
            break;
        }
        if !warmed_up {
            if let Some(deadline) = warmup_at {
                if Instant::now() >= deadline {
                    metrics.reset();
                    warmed_up = true;
                    eprintln!("[warmup] {} s elapsed — counters reset", cli.warmup);
                }
            }
        }
        match subscriber.take_available(Duration::from_millis(500)) {
            Ok(batch) => {
                for (c, recv_ns) in batch {
                    metrics.on_sample(c.publisher_id, c.counter);
                    metrics.record_latency(c.stamp_ns, recv_ns);
                }
            }
            // EINTR (Ctrl-C interrupting the poll) or any other reader
            // error — log and break so the final report still prints.
            Err(e) => {
                eprintln!("[take_available ended] {e:?}");
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
