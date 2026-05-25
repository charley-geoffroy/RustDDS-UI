//! Publisher demo / bench backed by the RustDDS implementation of
//! `dds-backend`. Flags let you sweep rate, payload size, QoS, etc.
//! without recompiling.

use std::io::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use clap::{Parser, ValueEnum};
use dds_backend::metrics::{MatchEvent, MatchTable, PubMetrics};
use dds_backend::rustdds::RustDdsBackend;
use dds_backend::DdsBackend;
use demo_msgs::{Chatter, DOMAIN_ID, TOPIC_NAME, TYPE_NAME};
use rustdds::policy::{Durability, History, Reliability};
use rustdds::{QosPolicies, QosPolicyBuilder};

#[derive(Parser, Debug)]
#[command(about = "DDS publisher bench (RustDDS backend)")]
struct Cli {
    /// Publish rate in Hz. Set to 0 to publish flat-out with no sleep.
    #[arg(long, default_value_t = 1.0)]
    rate: f64,

    /// Stop after this many samples (0 = unlimited).
    #[arg(long, default_value_t = 0)]
    count: u64,

    /// Stop after this many seconds (0 = run until Ctrl-C).
    #[arg(long, default_value_t = 0)]
    duration: u64,

    /// Block writes until at least N readers have matched. Eliminates
    /// pre-match buffering on the writer side (which otherwise shows
    /// up as a catch-up burst with stale stamps in subscriber metrics).
    /// Recommended for bench runs; 0 = start writing immediately.
    #[arg(long, default_value_t = 0)]
    await_readers: usize,

    /// Discard the first N seconds of metrics after writes begin —
    /// counters are zeroed at the boundary. Samples are still sent
    /// during this window.
    #[arg(long, default_value_t = 0)]
    warmup: u64,

    /// Extra opaque bytes appended to every message payload.
    #[arg(long, default_value_t = 0)]
    payload: usize,

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

    /// Append a per-second CSV row to this file. Header is written on
    /// open. Rows are skipped during `--await-readers` and `--warmup`
    /// so the file only contains steady-state samples.
    #[arg(long)]
    csv: Option<std::path::PathBuf>,
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

    // Discovery events aren't needed; spawn a drainer so the unbounded
    // channel doesn't leak.
    let (events_tx, events_rx) = mpsc::channel();
    thread::spawn(move || while events_rx.recv().is_ok() {});

    let backend = RustDdsBackend::start(cli.domain, events_tx)?;
    let qos = build_qos(&cli);
    let publisher =
        backend.create_typed_publisher_with_qos::<Chatter>(&cli.topic, TYPE_NAME, qos)?;
    let metrics = publisher.metrics().clone();
    let matches = publisher.matches().clone();

    let publisher_id = std::process::id();
    let mut counter: u64 = 0;
    let padding = vec![0u8; cli.payload];

    println!(
        "[{} {}] publisher {publisher_id:08x} writing to '{}' on domain {} \
         (rate={:.1}Hz payload={}B reliability={:?} await_readers={} duration={}s warmup={}s)",
        RustDdsBackend::name(),
        RustDdsBackend::version(),
        cli.topic,
        cli.domain,
        cli.rate,
        cli.payload,
        cli.reliability,
        cli.await_readers,
        cli.duration,
        cli.warmup,
    );

    // True once any await + warmup phases have passed. Used to gate
    // CSV writes so the file only contains steady-state samples.
    let measuring = Arc::new(AtomicBool::new(
        cli.await_readers == 0 && cli.warmup == 0,
    ));

    // 1 Hz heartbeat — stats + match events queued since the last tick.
    {
        let metrics = metrics.clone();
        let matches = matches.clone();
        let stop = stop.clone();
        let measuring = measuring.clone();
        let csv_path = cli.csv.clone();
        // Captured into the thread for the CSV header (self-describing run config).
        let csv_rate = cli.rate;
        let csv_payload = cli.payload;
        let csv_duration = cli.duration;
        let csv_warmup = cli.warmup;
        let csv_await = cli.await_readers;
        let csv_reliab = cli.reliability;
        let csv_hist = cli.history_depth;
        let csv_topic = cli.topic.clone();
        let csv_domain = cli.domain;
        thread::spawn(move || {
            let mut csv = csv_path.as_ref().map(|p| {
                let f = std::fs::File::create(p).expect("failed to open --csv file");
                let mut w = std::io::BufWriter::new(f);
                // Self-describing header: a "# config: k=v k=v" line the
                // explorer's bench viewer parses to know the run params
                // (target rate, payload, QoS) so it can compare observed
                // vs intended.
                writeln!(
                    w,
                    "# config: kind=pub rate={} payload={} duration={} \
                     warmup={} await_readers={} reliability={:?} \
                     history_depth={} topic={} domain={}",
                    csv_rate, csv_payload, csv_duration, csv_warmup,
                    csv_await, csv_reliab, csv_hist, csv_topic, csv_domain,
                )
                .unwrap();
                writeln!(w, "t_s,sent,errors,rate_per_s,write_avg_us,write_max_us").unwrap();
                w
            });
            while !stop.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_secs(1));
                print_match_events(&matches, &metrics);
                let snap = metrics.snapshot();
                println!("[stats] {}", snap.format_line());
                if measuring.load(Ordering::Relaxed) {
                    if let Some(w) = csv.as_mut() {
                        let _ = writeln!(
                            w,
                            "{:.3},{},{},{:.3},{},{}",
                            snap.elapsed_s,
                            snap.sent,
                            snap.errors,
                            snap.rate_per_s,
                            snap.write_ns_avg / 1_000,
                            snap.write_ns_max / 1_000,
                        );
                        let _ = w.flush();
                    }
                }
            }
        });
    }

    // Optionally block writes until enough readers have matched, so
    // there's no pre-match buffer on the writer to flush as a burst.
    if cli.await_readers > 0 {
        eprintln!(
            "[await] waiting for {} reader(s) to match before writing...",
            cli.await_readers,
        );
        while !stop.load(Ordering::Relaxed) {
            publisher.drain_status();
            let open = matches.snapshot().open.len();
            if open >= cli.await_readers {
                eprintln!("[await] {} reader(s) matched — starting writes", open);
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        // Reset so the very first write is t=0 in the metrics.
        metrics.reset();
        matches.rebase_sent_counter();
        if cli.warmup == 0 {
            measuring.store(true, Ordering::Relaxed);
        }
    }

    let period = (cli.rate > 0.0).then(|| Duration::from_secs_f64(1.0 / cli.rate));
    let mut next_deadline = Instant::now();
    let stop_at = (cli.duration > 0).then(|| Instant::now() + Duration::from_secs(cli.duration));
    let warmup_at = (cli.warmup > 0).then(|| Instant::now() + Duration::from_secs(cli.warmup));
    let mut warmed_up = warmup_at.is_none();

    while !stop.load(Ordering::Relaxed) {
        if matches!(stop_at, Some(d) if Instant::now() >= d) {
            break;
        }
        if cli.count > 0 && counter >= cli.count {
            break;
        }

        let stamp_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let msg = Chatter {
            publisher_id,
            counter,
            stamp_ns,
            text: format!("hello #{counter}"),
            padding: padding.clone(),
        };
        publisher.write(msg)?;
        counter += 1;

        if !warmed_up {
            if let Some(deadline) = warmup_at {
                if Instant::now() >= deadline {
                    metrics.reset();
                    matches.rebase_sent_counter();
                    warmed_up = true;
                    measuring.store(true, Ordering::Relaxed);
                    eprintln!("[warmup] {} s elapsed — counters reset", cli.warmup);
                }
            }
        }

        if let Some(p) = period {
            next_deadline += p;
            let now = Instant::now();
            if now < next_deadline {
                thread::sleep(next_deadline - now);
            } else {
                // Fell behind — resync so the next sleep doesn't try to
                // burn off accumulated debt all at once.
                next_deadline = now;
            }
        }
    }

    publisher.drain_status();
    print_match_events(&matches, &metrics);

    println!("\n[final] {}", publisher.report().to_json_pretty());
    Ok(())
}

fn print_match_events(matches: &MatchTable, metrics: &PubMetrics) {
    let current_sent = metrics.sent();
    for event in matches.drain_events() {
        match event {
            MatchEvent::Matched(s) => println!(
                "[+match]   reader={} sent_at_match={}",
                s.reader_guid, s.sent_at_match
            ),
            MatchEvent::Unmatched(s) => println!(
                "[-unmatch] reader={} samples_during_match={}",
                s.reader_guid,
                s.samples_during(current_sent),
            ),
        }
    }
}
