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
