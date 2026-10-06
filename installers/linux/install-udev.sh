#!/usr/bin/env bash
# Instala la regla udev limitada al dispositivo 1189:8890. Requiere permisos de administrador:
# usa sudo en una terminal y, si no la hay, pkexec (ventana de contraseña del sistema).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
if [[ $EUID -ne 0 ]]; then
  if [[ -t 0 ]]; then
    exec sudo "$here/install-udev.sh" "$@"
  elif command -v pkexec >/dev/null; then
    exec pkexec "$here/install-udev.sh" "$@"
  else
    echo "Ejecuta en una terminal: sudo $here/install-udev.sh" >&2
    exit 1
  fi
fi
install -m 644 "$here/70-macropad-agent.rules" /etc/udev/rules.d/70-macropad-agent.rules
udevadm control --reload
udevadm trigger --subsystem-match=usb --attr-match=idVendor=1189 --attr-match=idProduct=8890
udevadm trigger --subsystem-match=input
echo "Regla udev instalada. Si el acceso sigue denegado, desconecta y vuelve a conectar el macro pad."
