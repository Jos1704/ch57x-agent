#!/bin/sh
# Activa la regla udev y el servicio de usuario para todas las sesiones.
set -e
if command -v udevadm >/dev/null 2>&1; then
  udevadm control --reload || true
  udevadm trigger --subsystem-match=usb --attr-match=idVendor=1189 --attr-match=idProduct=8890 || true
fi
if command -v systemctl >/dev/null 2>&1; then
  systemctl --global enable macropad-agent.service || true
fi
echo "MacroPad Agent: el agente inicia con tu próxima sesión."
echo "Para iniciarlo ya: systemctl --user daemon-reload && systemctl --user start macropad-agent"
