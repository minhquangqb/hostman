# Hostman

> Manage local virtual hosts from a desktop app — map friendly domains like `myapp.test` to any local port, with HTTPS and Git-based sync across machines.

Hostman is a cross-platform (macOS & Windows) desktop tool that turns `https://myapp.test` into a reverse proxy for `localhost:2222`. Add as many hosts as you want, toggle them on/off, and keep the same setup on every machine through a Git-synced config.

## Features

- **Unlimited virtual hosts** — map any domain to any `host:port` target.
- **Automatic local HTTPS** — Caddy issues trusted certificates for internal TLDs (`.test`, `.localhost`).
- **Configurable TLD** — default `.test`, or set a full custom domain per host.
- **One-click apply** — updates the system `hosts` file and reloads the proxy with no downtime.
- **Git sync** — share one config across machines (init, remote, commit, pull, push from the UI).
- **Open in browser** — launch any host directly from the app.
- **Cross-platform** — macOS and Windows.

## Download

Grab the latest installer from the [Releases](https://github.com/minhquangqb/hostman/releases) page:

| OS | File |
|---|---|
| macOS (Apple Silicon) | `Hostman_*_aarch64.dmg` |
| macOS (Intel) | `Hostman_*_x64.dmg` |
| Windows | `Hostman_*_x64-setup.exe` / `.msi` |

> Builds are currently **unsigned**. On first launch:
> - **macOS** — *System Settings → Privacy & Security → Open Anyway*, or run
>   `xattr -dr com.apple.quarantine /Applications/Hostman.app`.
> - **Windows** — SmartScreen → *More info → Run anyway*.

## How it works

```
Browser requests myapp.test
        │
        ▼
   hosts file  ──►  127.0.0.1
        │
        ▼
   Caddy (:80/:443)  ──► reads Host header ──►  reverse_proxy localhost:2222
```

A `hosts` file only maps a hostname to an IP — it cannot encode a port. Hostman therefore uses two layers:

1. The **hosts file** resolves each domain to `127.0.0.1`.
2. **Caddy** listens on `:80`/`:443`, reads the `Host` header, and reverse-proxies the request to the configured port. Caddy also provisions a local CA for HTTPS.

The single source of truth is one JSON file (`~/.hostman/config/hosts.json`), synced via Git.

## Tech stack

| Layer | Technology |
|---|---|
| GUI | Tauri 2 + Vue 3 + TypeScript |
| Backend | Rust (config, hosts file, Caddy control, Git) |
| Proxy | Caddy (bundled as a sidecar binary) |
| Sync | Git |

## Project structure

```
src/                      # Vue 3 + TS frontend
  api.ts                  # typed invoke() wrappers
  types.ts                # types mirroring the Rust models
  App.vue                 # main UI
  components/             # HostModal, PreviewModal
src-tauri/src/
  models.rs               # Host, Config, status structs
  config.rs               # read/write ~/.hostman/config/hosts.json
  hosts_file.rs           # managed block + privileged write (cross-platform)
  caddy.rs                # generate Caddyfile + start/stop/reload
  git_sync.rs             # init/commit/pull/push
  lib.rs                  # Tauri commands
scripts/fetch-caddy.mjs   # download the Caddy sidecar binary
```

## Getting started

### Prerequisites

- [Node.js](https://nodejs.org/) 18+ and [pnpm](https://pnpm.io/)
- [Rust](https://www.rust-lang.org/tools/install) toolchain (for Tauri)
- Platform dependencies for Tauri — see the [Tauri prerequisites](https://tauri.app/start/prerequisites/)

### Development

```bash
pnpm install
node scripts/fetch-caddy.mjs    # download Caddy for the current platform
export HOSTMAN_CADDY="$PWD/src-tauri/binaries/caddy-$(rustc -Vv | sed -n 's/host: //p')"
pnpm tauri dev
```

`HOSTMAN_CADDY` points the app at the Caddy binary during development; in a packaged build the bundled sidecar is used automatically.

### Build

```bash
node scripts/fetch-caddy.mjs    # ensure binaries/caddy-<target-triple> exists
pnpm tauri build
```

## Configuration & sync

Hostman stores its config under `~/.hostman/config/`:

- `hosts.json` — the list of virtual hosts (commit this to Git).
- `Caddyfile` — generated automatically on apply.

To sync across machines: in the app, **Init repo → Set remote → Push**. On another machine, clone the repository into `~/.hostman/config` and **Pull**.

## Permissions & notes

- **Ports 80/443** require elevation on macOS/Linux (binding ports below 1024 needs root). Hostman prompts for administrator access when starting the proxy.
- **`.local` is reserved by mDNS on macOS** and can be slow or unreliable — prefer `.test`. Hostman lets you set any domain per host.
- **HTTPS trust** — click **Trust HTTPS** in the header once to install the local CA into the system trust store.
- **Background service (macOS)** — under **Chạy nền (Service)**, install Caddy as a `launchd` LaunchDaemon (`/Library/LaunchDaemons/com.hostman.caddy.plist`). It runs as root, auto-starts with the machine, and binds 80/443 without re-prompting for admin each time. Use **Gỡ service** to remove it. When the service is installed, prefer reloading config over the manual Start/Stop buttons.
- **Tray icon** — closing the window hides Hostman to the system tray; the app keeps running. Reopen from the tray icon or quit via the tray menu.

## Releasing

Releases are built automatically by GitHub Actions ([`.github/workflows/release.yml`](.github/workflows/release.yml)). To cut a release, push a version tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The workflow builds installers for macOS (Apple Silicon + Intel) and Windows in parallel, then publishes a **draft** GitHub Release with the artifacts attached. Review the draft and click *Publish* when ready.

## Roadmap

- [x] `caddy trust` button for one-click HTTPS trust
- [x] Run Caddy as a background service (launchd) to avoid repeated elevation and enable auto-start — *macOS; Windows service planned*
- [x] System tray icon (closing the window hides to tray; quit from the tray menu)
- [ ] Per-host custom TLD presets

## License

MIT
