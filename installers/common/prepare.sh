#!/bin/sh
# Lo ejecuta `cargo tauri build` antes de empaquetar: compila el agente y copia
# ch57x-keyboard-tool a target/release para incluirlos en el .deb o en el .app.
set -e
root="$(cd "$(dirname "$0")/../.." && pwd)"
cargo build --release --manifest-path "$root/Cargo.toml" -p macropad-agent
tool="$(command -v ch57x-keyboard-tool || true)"
[ -n "$tool" ] || tool="${CARGO_HOME:-$HOME/.cargo}/bin/ch57x-keyboard-tool"
if [ ! -x "$tool" ]; then
  echo "falta ch57x-keyboard-tool: cargo install ch57x-keyboard-tool" >&2
  exit 1
fi
if [ "$(uname -s)" = Darwin ]; then
  # En macOS se recompila la misma versión con libusb estática (rusb/vendored),
  # para que la .app funcione sin Homebrew.
  version="$("$tool" --version | awk '{print $2}')"
  cargo install ch57x-keyboard-tool --version "$version" --locked \
    --features rusb/vendored --root "$root/target/ch57x-vendored"
  tool="$root/target/ch57x-vendored/bin/ch57x-keyboard-tool"
fi
install -m 755 "$tool" "$root/target/release/ch57x-keyboard-tool"
