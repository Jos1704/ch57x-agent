#!/usr/bin/env bash
# Compila e instala MacroPad Agent como LaunchAgent del usuario en macOS.
# Probado en macOS 27 (Apple Silicon).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
bin="$HOME/.local/bin/macropad-agent"
support="$HOME/Library/Application Support/MacroPad Agent"
plist="$HOME/Library/LaunchAgents/com.macropad-agent.plist"

cargo build --release --manifest-path "$root/Cargo.toml" -p macropad-agent
install -d "$(dirname "$bin")" "$support" "$(dirname "$plist")"
install -m 755 "$root/target/release/macropad-agent" "$bin"
sed -e "s|__BIN__|$bin|" -e "s|__LOG__|$support/agent.log|" "$here/com.macropad-agent.plist" > "$plist"
launchctl bootout "gui/$(id -u)" "$plist" 2>/dev/null || true
launchctl bootstrap "gui/$(id -u)" "$plist"
echo "✓ LaunchAgent instalado. Registro: $support/agent.log"
