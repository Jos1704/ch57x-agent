#!/bin/sh
# Lo ejecuta `cargo tauri build` antes de empaquetar: compila el agente y copia
# ch57x-keyboard-tool a target/release para incluirlos en el .deb.
set -e
root="$(cd "$(dirname "$0")/../../.." && pwd)"
cargo build --release --manifest-path "$root/Cargo.toml" -p macropad-agent
tool="$(command -v ch57x-keyboard-tool || true)"
[ -n "$tool" ] || tool="${CARGO_HOME:-$HOME/.cargo}/bin/ch57x-keyboard-tool"
if [ ! -x "$tool" ]; then
  echo "falta ch57x-keyboard-tool: cargo install ch57x-keyboard-tool" >&2
  exit 1
fi
install -m 755 "$tool" "$root/target/release/ch57x-keyboard-tool"
