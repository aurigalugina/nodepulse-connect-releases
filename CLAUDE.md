# nodepulse-connect — Current State

## Role
Desktop app untuk personal devices (engineer Ussi / staff BPR) agar bisa join mesh network (Headscale/Tailscale) tanpa perlu akses dashboard penuh.

## Stack
- Tauri v2 + SvelteKit + Tailwind v4
- `@tauri-apps/api` v2, `@tauri-apps/plugin-fs`, `@tauri-apps/plugin-store`
- `@tauri-apps/plugin-updater` v2 — auto-update via MinIO manifest
- `@tauri-apps/plugin-process` v2 — `relaunch()` after update install
- Tailscale **bundled sebagai sidecar** (`src-tauri/binaries/`) — user tidak perlu install Tailscale. App spawn daemon-nya sendiri (`tailscaled --tun=userspace-networking --socket=<isolated>`) saat startup dan kill saat exit. System Tailscale tidak tahu koneksi ini sama sekali.

## Distribution & Auto-Update

### Build Pipeline
Cross-platform builds via **GitHub Actions** (`.github/workflows/build-connect.yml`).
Trigger: push tag `connect/v*` (e.g. `connect/v0.2.0`) atau `workflow_dispatch`.

Matrix builds (satu native runner per platform):
| Platform | Runner | Bundle |
|---|---|---|
| Windows x64 | windows-latest | `.msi` |
| macOS Intel | macos-13 | `.dmg` |
| macOS Apple Silicon | macos-latest | `.dmg` |
| Linux x64 | ubuntu-22.04 | `.AppImage` |

Setelah semua build selesai, `publish` job mengumpulkan artifacts, upload ke MinIO, dan generate `latest.json`.

### MinIO Structure
```
nodepulse/connect/
  latest.json                          ← update manifest (public read)
  releases/v{version}/
    windows/NodePulse Connect_*.msi
    windows/NodePulse Connect_*.msi.sig
    macos/NodePulse Connect_*.dmg
    macos/NodePulse Connect_*.dmg.sig
    linux/NodePulse Connect_*.AppImage
    linux/NodePulse Connect_*.AppImage.sig
```

### Signing Key Setup (WAJIB — satu kali sebelum first release)
```bash
# Generate keypair
npx @tauri-apps/cli signer generate

# Output:
# Public key  → tambah ke src-tauri/tauri.conf.json → plugins.updater.pubkey
# Private key → tambah sebagai GitHub secret TAURI_SIGNING_PRIVATE_KEY
```
**Jangan hilangkan private key** — tidak bisa di-recover. Simpan di password manager.

### GitHub Actions Secrets Required
| Secret | Value |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Private key dari `signer generate` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password keypair (kosong = `""`) |
| `MINIO_ENDPOINT` | e.g. `http://103.93.162.151:19000` |
| `MINIO_ACCESS_KEY` | MinIO access key |
| `MINIO_SECRET_KEY` | MinIO secret key |
| `MINIO_PUBLIC_URL` | e.g. `http://minio.ussireschndev.net` |

### Auto-Update Flow (client side)
`UpdateNotice.svelte` di-mount di `App.svelte` dengan delay 3 detik:
1. `check()` dari plugin-updater → fetch `latest.json` dari MinIO
2. Jika ada versi baru → tampilkan banner dengan versi + notes
3. User klik "Update Now" → `downloadAndInstall()` dengan progress bar 0-100%
4. Setelah `Finished` event → `relaunch()` dari plugin-process

### First Release Steps
1. Generate signing keypair (sekali saja): `make gen-signing-key`
2. Tambah public key ke `src-tauri/tauri.conf.json → plugins.updater.pubkey`
3. Tambah semua GitHub secrets di atas
4. Push tag: `git tag connect/v0.1.0 && git push --tags`
5. GitHub Actions build + publish otomatis (~15 menit)
6. Cek hasil di web panel: `/settings/connect`

### Manual Publish (fallback)
Jika tidak pakai GitHub Actions, gunakan:
```bash
VERSION=0.2.0 MINIO_PUBLIC_URL=http://minio.ussireschndev.net make publish-minio
```
Pastikan `mc` terinstall dan alias `minio` sudah dikonfigurasi.

## Window
- Size: 360×520 (compact, non-resizable)
- Single-page app, tidak ada routing — state machine via `connectionStore`

## Connection State Machine
```
IDLE → AUTHENTICATING → CLUSTER_SELECT → GENERATING_KEY → TAILSCALE_UP → CONNECTED
```

## Role-Aware Flow
Dua jenis user (dari JWT `role` field):
- **admin / operator**: tampilkan ClusterSelect → user pilih cluster → join `<cluster-slug>` namespace di Headscale
- **client** (BPR staff): skip cluster picker → auto-join `<cluster-slug>-connect` namespace berdasarkan `cluster_id` dari JWT

## Key Files

| File | Fungsi |
|---|---|
| `src/App.svelte` | Root — state machine routing + authStore.load on mount + UpdateNotice overlay |
| `src/lib/stores/authStore.svelte.js` | Config persist (url, username, token, role, cluster_id, device_name) |
| `src/lib/stores/connectionStore.svelte.js` | State machine, tailscale status polling, tray sync |
| `src/lib/api/nodepulse.js` | NodePulse REST client: login, getClusters, getNetworkConfig, generateNetworkKey, registerDevice, changePassword |
| `src/lib/components/Setup.svelte` | Login form — handle must_change_password (inline form), save role + cluster_id |
| `src/lib/components/TailscaleSetup.svelte` | First-run download screen — progress bar, retry on error |
| `src/lib/components/ClusterSelect.svelte` | Cluster picker for admin/operator; auto-join for client role |
| `src/lib/components/Connecting.svelte` | Progress steps display (GENERATING_KEY + TAILSCALE_UP states) |
| `src/lib/components/Connected.svelte` | Connected state — mesh IP, node list, disconnect button |
| `src/lib/components/UpdateNotice.svelte` | Auto-update banner: check latest.json → download with progress → relaunch |
| `src-tauri/tauri.conf.json` | App config: window size, updater endpoint + pubkey, bundle targets |
| `src-tauri/src/lib.rs` | Tauri plugins registered: updater + process |
| `src/app.css` | Design tokens via Tailwind v4 `@theme`: np-bg, np-surface, np-indigo, np-green, etc. |
| `.github/workflows/build-connect.yml` | Cross-platform build + MinIO publish pipeline |

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
- `must_change_password: true` → `Setup.svelte` tampilkan form ganti password sebelum lanjut ke CLUSTER_SELECT; JWT di-hold di `pendingToken` sampai berhasil
- Change password: `POST /api/v1/auth/change-password` via `changePassword()` di `nodepulse.js`

## Tauri Commands Used
- `read_config` / `write_config` / `clear_auth_token` — config persistence
- `tailscale_is_ready` — cek apakah binary sudah ada di app data dir (bool)
- `ensure_tailscale` — download + extract Tailscale ke app data dir; emit `tailscale-setup` events
- `tailscale_up` / `tailscale_down` / `tailscale_status` — mesh join/leave/status (semua via isolated socket)
- `set_tray_connected` — tray icon state
- `get_device_identity` — returns `{ machine_id, mac_address }` cross-platform; fail-safe; fire-and-forget setelah CONNECTED

## Isolated Tailscale Daemon (v0.3.0)

`tailscaled` dan `tailscale` **didownload otomatis saat pertama kali launch** ke `<AppData>/tailscale-bin/`. App spawn daemon saat startup (noop jika belum download), kill saat exit.

```
tailscaled --tun=userspace-networking --socket=<isolated> --statedir=<AppData>/tailscale-state
tailscale   --socket=<isolated> up/status/logout ...
```

**Binary path:**
- Windows: `%APPDATA%\id.ussi.nodepulse-connect\tailscale-bin\tailscaled.exe`
- macOS: `~/Library/Application Support/id.ussi.nodepulse-connect/tailscale-bin/tailscaled`
- Linux: `~/.local/share/id.ussi.nodepulse-connect/tailscale-bin/tailscaled`

**Socket path:**
- Windows: `\\.\pipe\NodePulseConnect\tailscaled`
- macOS/Linux: `<AppData>/tailscale-state/tailscaled.sock`

**First-run flow:**
1. `App.svelte` call `tailscale_is_ready` → false → tampilkan `TailscaleSetup.svelte`
2. `TailscaleSetup` call `ensure_tailscale` → download ~25MB tarball/zip dari `pkgs.tailscale.com`
3. Extract ke `<AppData>/tailscale-bin/`, set executable bit, clear macOS quarantine
4. `start_daemon()` dipanggil dari `ensure_tailscale` → daemon running
5. `onReady()` callback → App.svelte tampilkan login form normal

**Local dev setup:** langsung `npm run tauri dev` — app akan download binary saat pertama kali jalan.

**CI:** `build.yml` tidak perlu download binary — tidak ada sidecar, binary didownload oleh app.

## DO NOT
- Jangan pakai httpOnly cookie — desktop app pakai Bearer token
- Jangan tambah page routing (tidak ada SvelteKit pages, semua satu halaman)
- Jangan hardcode Headscale URL — selalu ambil dari `/api/v1/network/config`
