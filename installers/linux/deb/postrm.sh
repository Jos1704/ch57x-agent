#!/bin/sh
set -e
if [ "$1" = "remove" ] || [ "$1" = "purge" ]; then
  if command -v systemctl >/dev/null 2>&1; then
    systemctl --global disable macropad-agent.service || true
  fi
  if command -v udevadm >/dev/null 2>&1; then
    udevadm control --reload || true
  fi
fi
