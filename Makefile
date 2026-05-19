.PHONY: install dev build build-web clean lint check docker-up docker-down docker-logs publish-minio gen-signing-key release

# ── Deps ──────────────────────────────────────────────────────────────────────
# Install all npm + Cargo dependencies (run once after clone)
install:
	npm install

# ── Dev (local) ───────────────────────────────────────────────────────────────
# Start Tauri dev window (hot-reload frontend + Rust backend)
# Must be run from a Windows machine with Rust toolchain installed
dev:
	npm run tauri dev

# ── Dev (Docker) ──────────────────────────────────────────────────────────────
# Run Vite frontend dev server in Docker on port 1420.
# Tauri on Windows connects to http://<this-host>:1420 as devUrl.
docker-up:
	docker compose up -d --build

docker-down:
	docker compose down

docker-logs:
	docker compose logs -f frontend

# ── Build ─────────────────────────────────────────────────────────────────────
# Build release installer (.msi) for Windows
# Output: src-tauri/target/release/bundle/msi/NodePulse Connect_*.msi
build:
	npm run tauri build

# Build frontend only (for CI / lint checks without Rust)
build-web:
	npm run build

# ── Lint / Check ──────────────────────────────────────────────────────────────
# Lint Rust code
lint:
	cd src-tauri && cargo clippy -- -D warnings

# Type-check Svelte components via svelte-check
check:
	npx svelte-check --tsconfig ./jsconfig.json 2>&1

# ── Clean ─────────────────────────────────────────────────────────────────────
clean:
	rm -rf dist node_modules src-tauri/target

# ── Release — tag + push (triggers GitHub Actions build) ─────────────────────
# Reads version from src-tauri/tauri.conf.json automatically.
# Usage: make release
# To release a specific version: edit tauri.conf.json version first, then make release
release:
	$(eval VERSION := $(shell python3 -c "import json; print(json.load(open('src-tauri/tauri.conf.json'))['version'])"))
	$(eval TAG := connect/v$(VERSION))
	@if git rev-parse "$(TAG)" >/dev/null 2>&1; then \
		echo "Tag $(TAG) already exists. Bump version in src-tauri/tauri.conf.json first."; \
		exit 1; \
	fi
	@echo "Releasing NodePulse Connect $(VERSION)..."
	git tag $(TAG)
	git push origin $(TAG)
	@echo "Tag $(TAG) pushed. GitHub Actions build started."
	@echo "Monitor: https://github.com/$(shell git remote get-url origin | sed 's/.*github.com[:/]//' | sed 's/\.git//')/actions"

# ── Distribution (manual fallback — CI is the primary path) ───────────────────
# Generate a new Tauri signing keypair (run ONCE, then store in GitHub secrets).
# Prints the public key — add it to src-tauri/tauri.conf.json → plugins.updater.pubkey
# Prints the private key — add as GitHub secret TAURI_SIGNING_PRIVATE_KEY
gen-signing-key:
	npx @tauri-apps/cli signer generate

# Upload a pre-built bundle to MinIO manually (when not using GitHub Actions).
# Prerequisites:
#   1. mc (MinIO client) installed and configured: mc alias set minio <url> <key> <secret>
#   2. MINIO_PUBLIC_URL env var set (e.g. http://minio.ussireschndev.net)
#   3. VERSION env var set (e.g. make publish-minio VERSION=0.2.0)
#   4. Bundles built locally with TAURI_SIGNING_PRIVATE_KEY set
#
# Usage:
#   VERSION=0.2.0 MINIO_PUBLIC_URL=http://... make publish-minio
publish-minio:
ifndef VERSION
	$(error VERSION is required, e.g. make publish-minio VERSION=0.2.0)
endif
ifndef MINIO_PUBLIC_URL
	$(error MINIO_PUBLIC_URL is required, e.g. MINIO_PUBLIC_URL=http://minio.ussireschndev.net make publish-minio)
endif
	@echo "Publishing NodePulse Connect v$(VERSION) to MinIO..."
	@BASE="minio/nodepulse/connect/releases/v$(VERSION)"; \
	find src-tauri/target/release/bundle -name "*.msi"      -exec mc cp {} $$BASE/windows/ \; ; \
	find src-tauri/target/release/bundle -name "*.msi.sig"  -exec mc cp {} $$BASE/windows/ \; ; \
	find src-tauri/target/release/bundle -name "*.dmg"      -exec mc cp {} $$BASE/macos/   \; ; \
	find src-tauri/target/release/bundle -name "*.dmg.sig"  -exec mc cp {} $$BASE/macos/   \; ; \
	find src-tauri/target/release/bundle -name "*.AppImage" -exec mc cp {} $$BASE/linux/   \; ; \
	find src-tauri/target/release/bundle -name "*.AppImage.sig" -exec mc cp {} $$BASE/linux/ \;
	@echo "Generating latest.json..."
	@python3 scripts/gen_connect_manifest.py
	@mc cp latest.json minio/nodepulse/connect/latest.json
	@mc anonymous set download minio/nodepulse/connect/
	@echo "Done. Update manifest: $(MINIO_PUBLIC_URL)/nodepulse/connect/latest.json"
	@rm -f latest.json
