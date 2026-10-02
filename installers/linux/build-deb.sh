#!/usr/bin/env bash
# Genera el paquete .deb (agente, ventana, ch57x-keyboard-tool, regla udev y servicio).
# Requiere: cargo install tauri-cli --version "^2" --locked
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root/crates/desktop"
cargo tauri build --bundles deb
ls -1 "$root"/target/release/bundle/deb/*.deb
