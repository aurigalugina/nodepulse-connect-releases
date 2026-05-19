# NodePulse Connect — Deploy & Dev Guide

Desktop app Windows untuk join Headscale mesh NodePulse dari personal device (laptop, PC) tanpa terminal.

---

## Arsitektur

```
┌─────────────────────────────────┐     ┌──────────────────────────────────┐
│  Server Linux / Laptop Dev      │     │  Windows Machine                 │
│                                 │     │                                  │
│  Docker Container               │     │  Tauri App                       │
│  ┌─────────────────────────┐    │     │  ┌──────────────────────────┐   │
│  │  Vite dev server        │◄───┼─────┤  │  WebView2 (frontend)     │   │
│  │  port 1420              │    │     │  │  + Rust native layer     │   │
│  └─────────────────────────┘    │     │  │  + Tailscale CLI         │   │
│                                 │     │  └──────────────────────────┘   │
└─────────────────────────────────┘     └──────────────────────────────────┘
```

- **Docker** hanya menjalankan Vite frontend dev server (hot-reload).
- **Tauri** di Windows menjalankan WebView2 yang load dari `devUrl`, plus native Rust layer untuk spawn Tailscale dan baca/tulis config.
- Untuk **production build** (distribusi .msi), seluruh build dilakukan dari Windows — tidak butuh Docker.

---

## Prasyarat

### Server Linux (Docker host)
- Docker + Docker Compose
- Port `1420` terbuka ke Windows dev machine

### Windows machine (dev atau build)
| Tool | Cara install |
|---|---|
| Node.js 20+ | https://nodejs.org |
| Rust toolchain | https://rustup.rs → download `rustup-init.exe` → jalankan → pilih default install |
| Tauri v2 CLI | Setelah Rust terinstall: `cargo install tauri-cli --version "^2"` |
| WebView2 Runtime | Sudah built-in di Windows 11; Windows 10: https://developer.microsoft.com/en-us/microsoft-edge/webview2/ |
| Tailscale for Windows | https://pkgs.tailscale.com/stable/tailscale-setup-latest.exe |

> **Setelah install Rust:** Tutup terminal lama, buka terminal baru agar `cargo` masuk ke PATH. Verifikasi: `cargo --version`

---

## Development

### 1. Jalankan Vite dev server (Docker — di Linux)

```bash
cd nodepulse-connect
make docker-up      # build image + start container
make docker-logs    # lihat output, tunggu "VITE v... ready"
```

Vite server akan berjalan di `http://<ip-server>:1420`.

### 2. Update devUrl di tauri.conf.json (di Windows)

Edit `src-tauri/tauri.conf.json`, ganti `devUrl` ke IP server:

```json
"devUrl": "http://192.168.1.100:1420"
```

> Ganti `192.168.1.100` dengan IP laptop/server yang menjalankan Docker.

### 3. Install dependencies (di Windows, sekali saja)

```bash
cd nodepulse-connect
make install
# atau: npm install
```

### 4. Jalankan Tauri dev window (di Windows)

```bash
make dev
# atau: npm run tauri dev
```

Tauri akan membuka window yang load frontend dari Docker container. Edit file di `src/` → browser hot-reload otomatis.

---

## Production Build (.msi)

Build **harus dilakukan dari Windows machine** (Tauri tidak support cross-compile ke Windows dari Linux).

```bash
# Di Windows machine, dari folder nodepulse-connect/
make install    # jika belum
make build
```

Output: `src-tauri/target/release/bundle/msi/NodePulse Connect_0.1.0_x64_en-US.msi`

Installer ini bisa langsung didistribusikan ke user internal — double-click untuk install.

> **Sebelum build:** Pastikan `devUrl` di `tauri.conf.json` sudah dikembalikan ke `http://localhost:1420` (bukan IP Docker). Tauri build production tidak pakai devUrl, tapi lebih baik konsisten.

---

## Makefile targets

| Target | Deskripsi |
|---|---|
| `make install` | `npm install` — jalankan sekali setelah clone |
| `make docker-up` | Build Docker image + start Vite dev server di port 1420 |
| `make docker-down` | Stop container |
| `make docker-logs` | Follow log container |
| `make dev` | Jalankan Tauri dev window (dari Windows) |
| `make build` | Build release `.msi` installer (dari Windows) |
| `make build-web` | Build frontend saja tanpa Rust |
| `make lint` | `cargo clippy` untuk Rust code |
| `make check` | `svelte-check` untuk Svelte components |
| `make clean` | Hapus `dist/`, `node_modules/`, `src-tauri/target/` |

---

## Konfigurasi persisten

App menyimpan config di `%APPDATA%\NodePulse Connect\config.json`:

```json
{
  "nodepulse_url": "http://192.168.1.100:18080",
  "username": "admin",
  "auth_token": "<jwt>",
  "last_cluster_id": "cluster_xxx",
  "device_name": "my-home-pc"
}
```

> Password **tidak disimpan**. Token expired → app minta re-login otomatis.

---

## Troubleshooting

| Masalah | Solusi |
|---|---|
| `Tailscale not found` | Install Tailscale for Windows dari link di atas, restart app |
| `Cannot reach NodePulse at [URL]` | Pastikan NodePulse backend jalan di `18080`, cek firewall |
| `Invalid credentials` | Cek username/password di NodePulse web panel |
| `tailscale up` error | Lihat pesan error di screen — biasanya auth key expired (coba lagi) atau Tailscale service tidak jalan |
| `cargo` / Rust tidak ditemukan (`No such file or directory`) | Install Rust via https://rustup.rs → restart terminal → verifikasi `cargo --version` |
| Mesh IP tidak muncul setelah connect | Tunggu 5–10 detik, app polling otomatis tiap 5 detik |
