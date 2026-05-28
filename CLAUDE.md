# nodepulse-connect — Current State

## Role
Desktop app untuk personal devices (engineer Ussi / staff BPR) agar bisa join mesh network (Headscale/Tailscale) tanpa perlu akses dashboard penuh.

## Stack
- Tauri v2 + SvelteKit + Tailwind v4
- `@tauri-apps/api` v2, `@tauri-apps/plugin-fs`, `@tauri-apps/plugin-store`
- `@tauri-apps/plugin-updater` v2 — auto-update via GitHub Releases manifest
- `@tauri-apps/plugin-process` v2 — `relaunch()` after update install
- Tailscale **bundled (Win/macOS) atau didownload (Linux)** — user tidak perlu install Tailscale. App spawn daemon-nya sendiri (`tailscaled --tun=userspace-networking --socket=<isolated>`) saat startup dan kill saat exit. System Tailscale tidak tahu koneksi ini.

## Distribution & Auto-Update

### Build Pipeline
Cross-platform builds via **GitHub Actions** (`.github/workflows/build.yml`).
Trigger: push tag `v*` (e.g. `v0.3.7`) atau `workflow_dispatch`.

Matrix builds:
| Platform | Runner | Bundle |
|---|---|---|
| Windows x64 | windows-latest | `.exe` (NSIS) |
| macOS Apple Silicon | macos-latest | `.dmg` |
| Linux x64 | ubuntu-22.04 | `.AppImage` |

Setelah build selesai, `publish` job assembles `latest.json` dan creates GitHub Release di repo `nodepulse-connect-releases`.

### Update Manifest Endpoint
`https://github.com/aurigalugina/nodepulse-connect-releases/releases/latest/download/latest.json`

### GitHub Actions Secrets Required
| Secret | Value |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Private key dari `npx @tauri-apps/cli signer generate` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password keypair (kosong = `""`) |

### Tailscale Binary Delivery per Platform
| Platform | Metode | Sumber |
|---|---|---|
| Linux | Runtime download oleh app (first launch) | `pkgs.tailscale.com/stable/tailscale_1.98.2_amd64.tgz` |
| Windows | CI bundle via `externalBin`, di-copy dari NSIS installer | `tailscale-setup-full-1.98.2.exe` → Program Files\Tailscale |
| macOS | CI bundle via `externalBin`, di-copy dari Homebrew | `brew install tailscale` → `/opt/homebrew/bin/` |

Build Win/macOS menggunakan override config: `npm run tauri build -- -c src-tauri/tauri-sidecar-extra.json`
`tauri-sidecar-extra.json` menambahkan `bundle.externalBin` ke config base tanpa mengubah `tauri.conf.json`.

Alasan split: `pkgs.tailscale.com` hanya punya tarball untuk Linux. Windows dan macOS tidak tersedia sebagai standalone binary di URL publik — harus ekstrak dari installer/brew.

## Window
- Size: 360×520 (compact, non-resizable)
- Single-page app, tidak ada routing — state machine via `connectionStore`

## Connection State Machine
```
IDLE → AUTHENTICATING → CLUSTER_SELECT → GENERATING_KEY → TAILSCALE_UP → CONNECTED
```

## Startup Flow (v0.3.6+)
```
App open → StartupCheck (cek update) → TailscaleSetup? (Linux first-run only) → Login form
```
1. `StartupCheck.svelte` tampil dulu — check update via plugin-updater
2. Jika update tersedia: tampilkan versi + tombol "Update Now" (download+relaunch) dan "Later"
3. Jika up-to-date / offline: lanjut otomatis setelah 1-1.5 detik
4. Setelah `startupDone = true`, cek `tailscale_is_ready`
5. Jika binary belum ada (Linux first-run): `TailscaleSetup.svelte` download binary
6. Jika binary sudah ada: langsung ke login/reconnect

## Role-Aware Flow
- **admin / operator**: ClusterSelect → pilih cluster → join `<cluster-slug>` namespace
- **client** (BPR staff): skip cluster picker → auto-join `<cluster-slug>-connect` namespace berdasarkan `cluster_id` dari JWT

## Key Files

| File | Fungsi |
|---|---|
| `src/App.svelte` | Root — startup gate + state machine routing + UpdateNotice overlay |
| `src/lib/stores/authStore.svelte.js` | Config persist (url, username, token, role, cluster_id, device_name) |
| `src/lib/stores/connectionStore.svelte.js` | State machine, tailscale status polling, tray sync |
| `src/lib/api/nodepulse.js` | NodePulse REST client: login, getClusters, getNetworkConfig, generateNetworkKey, registerDevice, changePassword |
| `src/lib/components/StartupCheck.svelte` | **v0.3.6+** — Startup gate: cek update → up-to-date/download banner → onDone() |
| `src/lib/components/Setup.svelte` | Login form — handle must_change_password (inline form), save role + cluster_id |
| `src/lib/components/TailscaleSetup.svelte` | Linux first-run: download+extract Tailscale, progress bar, retry on error |
| `src/lib/components/ClusterSelect.svelte` | Cluster picker (admin/operator); auto-join (client role) |
| `src/lib/components/Connecting.svelte` | Progress steps + error display dengan "Try again" / "Sign out" buttons |
| `src/lib/components/Connected.svelte` | Connected state — mesh IP, node list, disconnect button |
| `src/lib/components/UpdateNotice.svelte` | Background update banner (setelah startup, saat app running) |
| `src-tauri/tauri.conf.json` | App config: window size, updater endpoint + pubkey, bundle targets |
| `src-tauri/tauri-sidecar-extra.json` | Extra config (Win/macOS builds): tambah `bundle.externalBin` |
| `src-tauri/src/lib.rs` | Tauri plugins + DaemonHandle + start_daemon di setup |
| `src-tauri/src/commands/tailscale.rs` | Seluruh logika daemon + CLI commands |
| `src/app.css` | Design tokens via Tailwind v4 `@theme` |
| `.github/workflows/build.yml` | CI: bundle Tailscale binaries (Win/macOS), build, publish GitHub Release |

## Design Tokens (app.css)
```
--color-np-bg:           #0b0b18  (darkest background)
--color-np-surface:      #12121f  (card background)
--color-np-elevated:     #1a1a2d  (elevated elements)
--color-np-border:       #24243c
--color-np-indigo:       #6366f1  (primary accent)
--color-np-indigo-light: #818cf8
--color-np-green:        #34d399
--color-np-red:          #f87171
--color-np-amber:        #fbbf24
--color-np-text:         #eef0f6
--color-np-muted:        #7c859e
--color-np-subtle:       #363850
```
Utility classes: `.np-input`, `.np-btn-primary`, `.np-btn-ghost`, `.np-btn-danger`

## Headscale Namespace Convention
- `nodepulse-core` — core backend container (admin join mesh)
- `<cluster-slug>` — BPR server runners (provisioned via dashboard)
- `<cluster-slug>-connect` — personal devices via NodePulse Connect (client role)

## Auth API
- Login: `POST /api/v1/auth/login/token` — Bearer token (bukan cookie)
- Response: `{ token, expires_at, role, cluster_id, must_change_password }`
- `cluster_id` non-empty hanya untuk role `client`
- `must_change_password: true` → Setup.svelte tampilkan form ganti password inline
- Change password: `POST /api/v1/auth/change-password`

## Tauri Commands Used
| Command | Tipe | Fungsi |
|---|---|---|
| `read_config` / `write_config` / `clear_auth_token` | sync | Config persistence |
| `tailscale_is_ready` | sync | Cek binary ada di path yang benar (bool) |
| `ensure_tailscale` | async | Linux: download+extract jika belum ada. Win/macOS: verifikasi bundle |
| `tailscale_up` | async | Join mesh — cek daemon alive dulu, return error+log jika daemon fail |
| `tailscale_down` | async | Logout dari mesh |
| `tailscale_status` | async | Status + mesh IP via `tailscale status --json` |
| `get_daemon_log` | sync | Baca `<statedir>/tailscaled.log` untuk diagnostics |
| `set_tray_connected` | async | Update tray icon state |
| `get_device_identity` | async | Return `{ machine_id, mac_address }` — fire-and-forget setelah CONNECTED |

## Isolated Tailscale Daemon (v0.3.0+, revised v0.3.3+)

Tailscale berjalan sebagai daemon terpisah dari system Tailscale:

```
tailscaled --tun=userspace-networking --socket=<isolated> --statedir=<AppData>/tailscale-state
tailscale   --socket=<isolated> up/status/logout ...
```

### Binary Paths

| Platform | Path binary |
|---|---|
| Windows | Next to main exe: `C:\Program Files\NodePulse Connect\tailscaled.exe` |
| macOS | Next to main exe: `/Applications/NodePulse Connect.app/Contents/MacOS/tailscaled` |
| Linux | `~/.local/share/id.ussi.nodepulse-connect/tailscale-bin/tailscaled` |

### Socket Paths
| Platform | Socket |
|---|---|
| Windows | `<AppData>\id.ussi.nodepulse-connect\tailscale-state\tailscaled.sock` (AF_UNIX — not named pipe) |
| macOS/Linux | `<AppData>/id.ussi.nodepulse-connect/tailscale-state/tailscaled.sock` |

> **Why not named pipe on Windows**: `\\.\pipe\...` paths cause `ERROR_INVALID_OWNER` (1307) in
> Tailscale's `safesocket.Listen` — it sets an explicit owner SID that restricted user tokens
> cannot claim. AF_UNIX file sockets (Windows 10 1803+) avoid this entirely.

### Daemon Lifecycle
1. App startup → `start_daemon()` di `lib.rs` setup (noop jika binary belum ada)
2. `start_daemon` log output ke `<statedir>/tailscaled.log`
3. Setelah spawn, `wait_for_socket()` poll 25× @ 200ms (max 5 detik) sampai socket ready
4. `tailscale_up` cek `socket_is_ready` sebelum jalan — jika tidak ready, coba restart daemon
5. Jika masih tidak ready, return error + tail dari `tailscaled.log`
6. App exit → `DaemonHandle::kill()` membersihkan proses

### Error UX (v0.3.4+)
- Jika koneksi gagal: **tetap di Connecting screen** (tidak balik ke ClusterSelect)
- Error ditampilkan lengkap dengan scrollable log
- Tombol "Try again" → kembali ke ClusterSelect, retry dari awal
- Tombol "Sign out" → kembali ke IDLE

### Linux First-Run Flow
1. `tailscale_is_ready` → false → tampilkan `TailscaleSetup.svelte`
2. `ensure_tailscale` → download `tailscale_1.98.2_amd64.tgz` dari pkgs.tailscale.com (~25MB)
3. Extract ke `<AppData>/tailscale-bin/`, chmod 755
4. `start_daemon()` dipanggil → daemon running
5. `onReady()` callback → App.svelte lanjut ke login

### Cargo Dependencies (Linux-only conditional)
```toml
[target.'cfg(target_os = "linux")'.dependencies]
reqwest = { version = "0.12", features = ["rustls-tls"], default-features = false }
flate2 = "1"
tar = "0.4"
```

## Known Issues / Under Investigation
- **Windows joining mesh slow**: Inherent ke Windows userspace networking init. Max wait ~87s (15+10 iter × 3.5s) sebelum `tailscale up` 60s timeout. Biasanya selesai dalam 20-40 detik.

## Version History (recent)
- **v0.3.41** — `tailscale set --operator=<username>` before `tailscale up`; sets `serverMode=true` so `b.Start()` not called on CLI disconnect; doLogin completes after CLI exits
- **v0.3.40** — Wipe statedir on connect + remove `--force-reauth`; clean daemon start prevents startup `blockEngineUpdates`
- **v0.3.39** — `tailscale_down`: `logout` → `down`; preserves `profiles/<id>/` dir so `profileDirFor` succeeds on next reconnect
- **v0.3.38** — Remove `--operator` flag (not defined in Tailscale 1.98.2); restore `--timeout=60s --force-reauth`; Tokio wrapper 75s
- **v0.3.37** — `--timeout=60s` restored + `NO_PROXY=*` (correct combo for profileDirFor fix)
- **v0.3.36** — Keep statedir across attempts; `NO_PROXY=*`
- **v0.3.35** — Remove `--timeout=60s`; 75s post-up polling loop; reusable pre-auth keys

## DO NOT
- Jangan pakai httpOnly cookie — desktop app pakai Bearer token
- Jangan tambah page routing (tidak ada SvelteKit pages, semua satu halaman)
- Jangan hardcode Headscale URL — selalu ambil dari `/api/v1/network/config`
- Jangan remove `tauri-sidecar-extra.json` — diperlukan untuk Win/macOS CI builds
- Jangan tambah `externalBin` ke `tauri.conf.json` base — Linux tidak butuh dan akan gagal build
