# MacroPad Agent

Agente local para Linux y macOS que detecta el macro pad USB `1189:8890` y le programa el perfil del sistema operativo. No usa cuentas, red ni telemetría.

- `crates/core`: perfiles, catálogo de acciones, detección USB y programación (usa `ch57x-keyboard-tool`).
- `crates/daemon`: binario `macropad-agent` (CLI y agente en segundo plano).
- `crates/desktop`: ventana de configuración Tauri (`macropad-agent-desktop`).
- `profiles/`: perfiles incluidos.
- `installers/`: regla `udev`, servicio `systemd`, paquete `.deb` y LaunchAgent.

Instalación y uso: [docs/instalacion-local.md](docs/instalacion-local.md). Cómo terminar macOS: [docs/macos-agente.md](docs/macos-agente.md). Plan: [plan-macropad-agent.md](plan-macropad-agent.md).
