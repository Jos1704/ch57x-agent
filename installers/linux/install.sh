#!/usr/bin/env bash
# Compila e instala MacroPad Agent (agente + ventana) para el usuario actual en Linux.
# Para distribuciones basadas en Debian también está el paquete: installers/linux/build-deb.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"

cargo build --release --manifest-path "$root/Cargo.toml" -p macropad-agent -p macropad-agent-desktop
install -Dm 755 "$root/target/release/macropad-agent" "$HOME/.local/bin/macropad-agent"
install -Dm 755 "$root/target/release/macropad-agent-desktop" "$HOME/.local/bin/macropad-agent-desktop"
install -Dm 644 "$here/macropad-agent.desktop" "$HOME/.local/share/applications/macropad-agent.desktop"
install -Dm 644 "$root/crates/desktop/icons/128x128@2x.png" "$HOME/.local/share/icons/hicolor/256x256/apps/macropad-agent.png"
install -Dm 644 "$root/crates/desktop/icons/128x128.png" "$HOME/.local/share/icons/hicolor/128x128/apps/macropad-agent.png"
command -v update-desktop-database >/dev/null && update-desktop-database "$HOME/.local/share/applications" || true
echo "✓ binarios en ~/.local/bin y «MacroPad Agent» en el menú de aplicaciones"
if command -v omarchy >/dev/null; then
  echo "  Si no aparece en el lanzador de Omarchy, ejecuta: omarchy restart shell"
fi

if ! grep -qs "SUBSYSTEM==\"input\"" /etc/udev/rules.d/70-macropad-agent.rules /usr/lib/udev/rules.d/70-macropad-agent.rules; then
  echo "Instalando regla udev (pedirá tu contraseña)…"
  "$here/install-udev.sh"
fi

install -Dm 644 "$here/macropad-agent.service" "$HOME/.config/systemd/user/macropad-agent.service"
systemctl --user daemon-reload
systemctl --user enable macropad-agent.service
systemctl --user restart macropad-agent.service
echo "✓ servicio de usuario activo (journalctl --user -u macropad-agent -f para ver el registro)"
