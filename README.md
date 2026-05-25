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

A clean bench invocation. Start sub first so it's listening; pub will
wait for it to match before writing:

```bash
# Terminal 1
cargo run --release -p sub-rustdds -- --duration 30 --warmup 5

# Terminal 2
cargo run --release -p pub-rustdds -- \
    --rate 1000 --payload 1024 --duration 30 --warmup 5 --await-readers 1
```

Pub and sub must agree on `--reliability` and `--domain` for matching.
Without `--await-readers`, the pub's writer queue holds up to
`--history-depth` samples for any future reader; when one matches,
those buffered samples are delivered as a burst with stale `stamp_ns`,
which inflates the latency histogram. `--await-readers` avoids that
at the source.

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
