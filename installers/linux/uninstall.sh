#!/usr/bin/env bash
# Desinstala MacroPad Agent instalado con install.sh. Conserva ~/.config/macropad-agent (perfiles e historial).
set -euo pipefail
systemctl --user disable --now macropad-agent.service 2>/dev/null || true
rm -f "$HOME/.config/systemd/user/macropad-agent.service" \
      "$HOME/.local/bin/macropad-agent" \
      "$HOME/.local/bin/macropad-agent-desktop" \
      "$HOME/.local/share/applications/macropad-agent.desktop" \
      "$HOME/.local/share/icons/hicolor/256x256/apps/macropad-agent.png" \
      "$HOME/.local/share/icons/hicolor/128x128/apps/macropad-agent.png"
systemctl --user daemon-reload
if [[ -f /etc/udev/rules.d/70-macropad-agent.rules ]]; then
  sudo rm -f /etc/udev/rules.d/70-macropad-agent.rules
  sudo udevadm control --reload
fi
echo "MacroPad Agent desinstalado."
