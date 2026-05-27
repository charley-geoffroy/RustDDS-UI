# Rust DDS UI

A small desktop app to **poke at DDS networks** — built while exploring
[DDS](https://www.dds-foundation.org/) and [RustDDS](https://github.com/Atostek/RustDDS).

Sits at the intersection of:

- a **learning tool** — bilingual (EN/FR) docs with interactive diagrams
  explaining DDS, QoS matching, discovery, CDR serialization,
- a **DDS explorer** — live topic list, sample inspector with hex view,
  QoS matrix, endpoint browser.

> ✨ **Vibe-coded.** This is a small tool I built mostly to **explore the
> concepts** behind DDS — pub/sub, QoS matching, discovery, CDR. Not a
> production tool, just a friendly playground to poke at and learn from.

## Stack

- **Tauri 2** desktop shell (Rust backend, embedded webview)
- **Svelte 5** + TypeScript frontend
- **RustDDS 0.11** as the default DDS backend (pluggable via a
  `DdsBackend` trait)

## Prerequisites

- Rust (stable) — `rustup`
- Node.js + `pnpm`
- Tauri 2 system deps — see
  [tauri.app/start/prerequisites](https://tauri.app/start/prerequisites/)
  (Linux users: [INSTALL-linux.md](INSTALL-linux.md) has the exact
  apt/dnf/pacman commands)

## Run

```bash
# From the repo root
cd apps/explorer
pnpm install
pnpm tauri dev
```

The app opens. Builtin DDS topics show up immediately. To see user data,
in another terminal:

```bash
# Pub/sub demo on a `Chatter` topic
cargo run -p pub-rustdds
cargo run -p sub-rustdds
```

Or any ROS 2 talker / DDS publisher on the same domain.

### Bench flags

Both binaries take CLI flags so you can sweep parameters without
recompiling. `--help` on either lists everything; the most useful ones:

| flag | applies to | what it does |
|---|---|---|
| `--rate <Hz>` | pub | publish rate (default `1`, `0` = flat-out) |
| `--payload <bytes>` | pub | extra opaque bytes per message |
| `--count <N>` | pub | stop after N samples |
| `--await-readers <N>` | pub | block writes until N readers have matched (recommended for benches; avoids the catch-up burst of pre-match buffered samples) |
| `--duration <s>` | both | stop after N seconds |
| `--warmup <s>` | both | discard the first N seconds of measurement. Pub counts from first write; sub counts from the **first received sample** so the warmup naturally covers late discovery and any catch-up burst |
| `--reliability <kind>` | both | `reliable` (default) or `best-effort` |
| `--history-depth <N>` | both | KeepLast depth (default `100`) |
| `--domain <id>` | both | DDS domain (default `0`) |
| `--topic <name>` | both | topic name (default `Chatter`) |
| `--csv <path>` | both | append a per-second row. Counters (`sent`, `recv`, `lost_*`) are cumulative; rate and latency percentiles are computed over the last ~1 s window. Rows are skipped during `--await-readers`/`--warmup` so the file is steady-state only |

Pub and sub must agree on `--reliability` and `--domain` for matching.
The `elapsed_s` field in `[stats]` / CSV / final JSON uses a monotonic
`Instant`, so an NTP / DST jump to the wall clock during the run
won't poison the rate calculation. The wire stamp (`stamp_ns` in the
message) and `last_send_ns` in the JSON are still `SystemTime` nanos
— those need to be comparable across processes.

### Scenarios

Each scenario is a self-contained bench setup that highlights one
specific behavior. Always start the sub first (so the pub's
`--await-readers` has someone to wait for), and load the resulting
CSVs side-by-side in the explorer's **Bench** tab to read the
verdict.

#### 1. Healthy baseline

Reference run. Establishes what a sane verdict looks like on your
machine so the other scenarios have something to deviate from.

```bash
# Terminal 1
cargo run --release -p sub-rustdds -- \
    --warmup 5 --duration 30 --csv /tmp/sub.base.csv

# Terminal 2
cargo run --release -p pub-rustdds -- \
    --rate 1000 --payload 128 --warmup 5 --duration 30 \
    --await-readers 1 --csv /tmp/pub.base.csv
```

What to expect: pair verdict says **Run sain**, `loss_rate` is 0,
`p99 < 2 ms`, and `delta` is in the low hundreds (tail samples sent
after the sub stopped). If `p99` blows past 2 ms on a fresh boot,
your machine just has background noise — make a note of the baseline
and use *that* as the reference threshold.

#### 2. Reliable vs Best-Effort under load

Same workload twice, only `--reliability` changes. Compares where
each policy costs you: retransmit-blocking on Reliable vs silent
drops on Best-Effort.

```bash
# Run A — Reliable
cargo run --release -p sub-rustdds -- \
    --reliability reliable --warmup 5 --duration 30 \
    --csv /tmp/sub.reliable.csv
cargo run --release -p pub-rustdds -- \
    --reliability reliable --rate 5000 --payload 4096 \
    --await-readers 1 --warmup 5 --duration 30 \
    --csv /tmp/pub.reliable.csv

# Run B — Best-Effort (same numbers, just swap reliability)
cargo run --release -p sub-rustdds -- \
    --reliability best-effort --warmup 5 --duration 30 \
    --csv /tmp/sub.be.csv
cargo run --release -p pub-rustdds -- \
    --reliability best-effort --rate 5000 --payload 4096 \
    --await-readers 1 --warmup 5 --duration 30 \
    --csv /tmp/pub.be.csv
```

What to expect:
- **Run A** stays at `lost_wire ≈ 0` but `write_max_us` spikes (the
  writer parks waiting for acks under back-pressure — its
  `max_blocking_time` is 100 ms).
- **Run B** shows `lost_wire > 0` whenever the sub stalls, but
  `write_avg_us` stays flat — the writer never blocks.

Open both pair reports back-to-back to see the trade-off in one
glance.

#### 3. Why `--await-readers` matters

Demonstrates the pre-match catch-up burst that contaminates latency
when the pub starts before any sub is listening. The fix is in the
flag name.

```bash
# Terminal 1 — pub goes first, no await, fat history
cargo run --release -p pub-rustdds -- \
    --rate 1000 --duration 30 --history-depth 5000 \
    --csv /tmp/pub.burst.csv

# Wait ~5 s, THEN terminal 2 (warmup=0 so we keep the burst)
cargo run --release -p sub-rustdds -- \
    --warmup 0 --duration 25 --csv /tmp/sub.burst.csv
```

What to expect on the sub side: the first 1–2 CSV rows show
`rate_per_s` far above 1000 (catch-up flood of buffered samples)
and `lat_max_us` in the hundreds of milliseconds — those samples
carry a `stamp_ns` from before the sub existed.

Now rerun with `--await-readers 1` on the pub *and* `--warmup 5` on
the sub: the burst is gone and the verdict goes back to Ok.

#### 4. Multi-publisher fan-in

Four pubs at 250 Hz each, one sub. Verifies that `Chatter.publisher_id`
correctly partitions per-stream state on the sub.

```bash
# Terminal 1
cargo run --release -p sub-rustdds -- \
    --warmup 5 --duration 30 --csv /tmp/sub.fanin.csv

# Terminal 2
for _ in 1 2 3 4; do
    cargo run --release -p pub-rustdds -- \
        --rate 250 --duration 30 --warmup 5 --await-readers 1 &
done
wait
```

What to expect:
- Sub aggregate `rate_per_s ≈ 1000`.
- Final JSON's `streams[]` lists 4 entries with ~6250 received each
  and distinct `publisher_id` values.
- If one of the pubs restarts mid-run, the sub's rebaseline logic
  (`counter < first_counter_seen` → reset) keeps `lost_wire` from
  spuriously jumping by a million.

Pair analysis isn't meaningful here (it expects a single pub CSV);
read each side's report independently.

#### 5. Heavy payload

Pushes the payload toward the UDP single-datagram limit (~64 KiB) to
show where serialization and fragmentation start dominating write
latency.

```bash
# Terminal 1
cargo run --release -p sub-rustdds -- \
    --warmup 5 --duration 30 --csv /tmp/sub.fat.csv

# Terminal 2
cargo run --release -p pub-rustdds -- \
    --rate 500 --payload 65000 --warmup 5 --duration 30 \
    --await-readers 1 --csv /tmp/pub.fat.csv
```

What to expect: `write_avg_us` an order of magnitude higher than
Scenario 1's 128-byte baseline, per-window `lat_p99_us` noisier, and
on Reliable, occasional `write_max_us` spikes when the writer waits
for fragment acks. Push to 1 MiB and you'll see Reliable trigger
visible retransmit pauses; switch to Best-Effort at that size and
loss appears instead.

### Reading the bench output

Both demos run as plain CLIs, print a 1 Hz heartbeat while alive, and
dump a JSON report on Ctrl-C / `--duration` expiry. The JSON is meant
to be diff-friendly (`jq` works great).

**Publisher** — `pub-rustdds`:

```
[+match]   reader=0112f8db…00000007 sent_at_match=3
[stats] sent=42 (8.4/s) errs=0 write_avg=24µs write_max=47µs uptime=5.0s
[-unmatch] reader=0112f8db…00000007 samples_during_match=39
```

- `[+match]/[-unmatch]` — fires the moment a remote reader joins or
  leaves the writer's match list. `sent_at_match` / `samples_during_match`
  let you tell "discovered me but received nothing" from "discovered
  me and got N samples".
- `[stats]` — sent count, effective rate, and the per-call
  `writer.write()` duration (local serialize + queue, not E2E).
- Final JSON includes `metrics` (the heartbeat fields) and `matches`
  with full `open`/`closed` session history (per-reader GUID,
  `first_matched_ns`, `sent_at_match`, etc.).

**Subscriber** — `sub-rustdds`:

```
[stats] recv=42 lost_wire=0 (0.0%) lost_dds=0 reord=0 dup=0 lat p50=180µs p95=620µs max=4058µs rate=8.4/s uptime=5.0s
  recv │█████████▇▇████████
  lost │
  lat  │█▄▂▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁
```

- `lost_wire` — samples missing as detected by counter gaps on the
  `Chatter.counter` field. Late-join window (everything before the
  first received counter) is **not** counted as loss.
- `lost_dds` — samples the local DDS stack reported dropping via
  `DataReaderStatus::SampleLost`. When `lost_wire ≠ lost_dds`, the
  delta tells you *where* the loss happened (wire vs. local).
- `reord` / `dup` — counter went backwards or repeated.
- `lat p50/p95/max` — end-to-end latency from the `stamp_ns` field the
  publisher sets at write time. Percentiles come from a log-2 bucket
  histogram and are reported as the geometric mean of the matching
  bucket (worst-case ±√2). Negative latencies (clock skew) are dropped
  and counted in `clock_skew_skipped` in the final JSON.
- The three sparkline rows are a rolling 60s history at 1s
  resolution: `recv` count, `lost` count, `lat` max-per-bucket in µs.

Final JSON adds `latency: { count, min/avg/max, p50/p95/p99 }` and a
per-publisher `streams` array (`received`, `lost`, `last_counter`,
`first_counter_seen`).

### Forcing loss for demos

On a single host with Reliable QoS you won't naturally see loss. Two
easy ways to simulate it:

```bash
# 1. Kill the sub mid-stream — when it restarts it won't see the
#    gap (Volatile durability means the pub already dropped those
#    samples), so this only shows up if you pre-populate state.

# 2. Throttle the loopback briefly (Linux only). pf on macOS has
#    its own syntax — netem is the canonical tool.
sudo tc qdisc add dev lo root netem loss 5%
# run pub + sub, then:
sudo tc qdisc del dev lo root
```

## Containers

`pub-rustdds` and `sub-rustdds` ship as a single Dockerfile. To run a
pub + sub pair without installing Rust on the host:

```bash
docker compose -f docker/docker-compose.yml up --build
```

### Running pub and sub separately

The compose file declares two services (`pub` and `sub`); name the one
you want:

```bash
# pub only
docker compose -f docker/docker-compose.yml up --build pub

# sub only (in another terminal, on the same host or any host
# reachable by RTPS multicast)
docker compose -f docker/docker-compose.yml up --build sub
```

Without Compose, build and run each image directly:

```bash
# build
docker build -t ddsui-pub -f docker/Dockerfile --build-arg BIN_NAME=pub-rustdds .
docker build -t ddsui-sub -f docker/Dockerfile --build-arg BIN_NAME=sub-rustdds .

# run (host networking is required for RTPS multicast discovery)
docker run --rm --network=host ddsui-pub
docker run --rm --network=host ddsui-sub
```

Both containers use `network_mode: host` so RTPS multicast discovery
works. The explorer is **not** containerized — see
[INSTALL-linux.md](INSTALL-linux.md) for native install on Linux.

## Layout

```
apps/
  explorer/                # Tauri + Svelte app (the inspector)
crates/
  dds-backend/             # backend trait + RustDDS impl + HDDS stub
  demo-msgs/               # shared Chatter type
examples/
  {pub,sub}-{rustdds,hdds}/  # demo binaries
```
