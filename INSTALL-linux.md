# Running on Linux

The pub/sub demos are pure Rust and need only a Rust toolchain. The
**explorer** (Tauri 2) additionally needs the GTK + WebKit stack, which
macOS ships with the OS but Linux does not.

## Demos only (no GUI)

```bash
cargo run -p pub-rustdds
cargo run -p sub-rustdds
```

Or via Docker (no Rust toolchain needed on the host):

```bash
docker compose -f docker/docker-compose.yml up --build
```

See [docker/docker-compose.yml](docker/docker-compose.yml) — both
containers run on `network_mode: host` so RTPS multicast discovery
works.

## Explorer (Tauri app)

### Debian / Ubuntu 22.04+

```bash
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libsoup-3.0-dev \
  build-essential curl wget file pkg-config
```

Ubuntu 22.04 may need `libwebkit2gtk-4.0-dev` instead of `-4.1-dev`.

### Fedora / RHEL

```bash
sudo dnf install -y \
  webkit2gtk4.1-devel \
  gtk3-devel \
  libappindicator-gtk3-devel \
  librsvg2-devel \
  libsoup3-devel \
  @development-tools curl wget file
```

### Arch

```bash
sudo pacman -S --needed \
  webkit2gtk-4.1 gtk3 libappindicator-gtk3 librsvg libsoup3 \
  base-devel curl wget file
```

Then:

```bash
cd apps/explorer
pnpm install
pnpm tauri dev
```

The Tauri docs list the canonical, per-distro package set:
<https://tauri.app/start/prerequisites/>.

## Troubleshooting RTPS discovery on Linux

If pub and sub don't see each other on the same machine:

- Check the firewall (`sudo ufw status` / `sudo firewall-cmd --state`)
  isn't dropping UDP. RTPS uses ports in the 7400+ range plus multicast
  to `239.255.0.x`.
- Verify multicast is enabled on the chosen interface:
  `ip link show | grep MULTICAST`.
- In Docker, always use `--network=host`. Bridge networks break
  RTPS discovery silently.
