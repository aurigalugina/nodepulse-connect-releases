#!/usr/bin/env python3
"""Generate connect/latest.json from locally-built bundles.

Usage:
    VERSION=0.2.0 MINIO_PUBLIC_URL=http://minio.ussireschndev.net python3 scripts/gen_connect_manifest.py
"""
import json, os, glob
from datetime import datetime, timezone

version   = os.environ['VERSION']
pub_url   = os.environ['MINIO_PUBLIC_URL']
base_url  = f"{pub_url}/nodepulse/connect/releases/v{version}"
bundle    = "src-tauri/target/release/bundle"

def find1(pat):
    g = glob.glob(f"{bundle}/**/{pat}", recursive=True)
    return g[0] if g else None

def sig(p):
    sig_path = p + ".sig"
    if p and os.path.exists(sig_path):
        return open(sig_path).read().strip()
    return None

entries = [
    ("windows-x86_64", "windows", find1("*.msi")),
    ("darwin-x86_64",  "macos",   find1("*.dmg")),
    ("darwin-aarch64", "macos",   find1("*.dmg")),
    ("linux-x86_64",   "linux",   find1("*.AppImage")),
]

manifest = {
    "version":   version,
    "notes":     f"NodePulse Connect v{version}",
    "pub_date":  datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "platforms": {}
}

for target, plat, fpath in entries:
    s = sig(fpath)
    if fpath and s:
        manifest["platforms"][target] = {
            "url":       f"{base_url}/{plat}/{os.path.basename(fpath)}",
            "signature": s
        }

with open("latest.json", "w") as f:
    json.dump(manifest, f, indent=2)

print(json.dumps(manifest, indent=2))
