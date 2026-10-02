#!/usr/bin/env bash
# Instala la regla udev limitada al dispositivo 1189:8890. Requiere sudo.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
if [[ $EUID -ne 0 ]]; then
  exec sudo "$0" "$@"
fi
install -m 644 "$here/70-macropad-agent.rules" /etc/udev/rules.d/70-macropad-agent.rules
udevadm control --reload
udevadm trigger --subsystem-match=usb --attr-match=idVendor=1189 --attr-match=idProduct=8890
echo "Regla udev instalada. Si el acceso sigue denegado, desconecta y vuelve a conectar el macro pad."
