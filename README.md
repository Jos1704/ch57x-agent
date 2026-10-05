# MacroPad Agent

Agente local para Linux y macOS que detecta el macro pad USB `1189:8890` y le programa el perfil del sistema operativo. No usa cuentas, red ni telemetría.

- `crates/core`: perfiles, catálogo de acciones, detección USB y programación (usa `ch57x-keyboard-tool`).
- `crates/daemon`: binario `macropad-agent` (CLI y agente en segundo plano).
- `crates/desktop`: ventana de configuración Tauri (`macropad-agent-desktop`).
- `profiles/`: perfiles incluidos.
- `installers/`: regla `udev`, servicio `systemd`, paquete `.deb` y LaunchAgent.

## macOS

Probado en macOS 27 con Apple Silicon. Para programar el teclado no hace falta `sudo` ni dar permisos en Ajustes del Sistema.

### Instalar la app

1. Abre `MacroPad Agent_<versión>_aarch64.dmg` y arrastra **MacroPad Agent** a Aplicaciones.
2. Ábrela desde Aplicaciones. Como no está notarizada, la primera vez usa clic derecho → *Abrir*, o ejecuta `xattr -dr com.apple.quarantine "/Applications/MacroPad Agent.app"`.
3. Al abrirse instala el LaunchAgent `com.macropad-agent`. Desde ese momento, el agente aplica el perfil `desarrollo-macos` al iniciar sesión y cada vez que conectas el macro pad.

La app incluye `ch57x-keyboard-tool` y `libusb`. No necesita Homebrew ni Rust. Funciona solo en Apple Silicon con macOS 11 o posterior.

### Correr desde el código

```bash
xcode-select --install                         # si faltan las herramientas de línea de comandos
brew install rustup pkg-config
export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH"   # agrégalo también a ~/.zshrc
rustup default stable
cargo install ch57x-keyboard-tool
cargo install tauri-cli --version "^2" --locked

cargo test
cargo run -p macropad-agent -- diagnose        # detecta el macro pad y revisa permisos
cargo run -p macropad-agent -- apply desarrollo-macos
cargo run -p macropad-agent -- run             # agente en primer plano
cargo run -p macropad-agent-desktop            # ventana de configuración
```

Homebrew instala `rustup` como keg-only, sin `rustup-init`; por eso hace falta la línea del `PATH`.

Para que el agente arranque solo con el binario compilado (sin la app): `installers/macos/install.sh`.

### Generar el `.dmg`

```bash
cd crates/desktop && cargo tauri build --bundles app,dmg
```

Sale en `target/release/bundle/dmg/`. Si falla en `bundle_dmg.sh`, desmonta el volumen `rw.*.dmg` que quedó (`hdiutil info`, `hdiutil detach`) y repite el comando.

### Archivos

- Perfiles y configuración: `~/Library/Application Support/MacroPad Agent/`
- Registro del agente: `~/Library/Application Support/MacroPad Agent/agent.log`
- Desinstalar el agente: `launchctl bootout gui/$(id -u) ~/Library/LaunchAgents/com.macropad-agent.plist`, luego borra ese plist y la app.

## Documentación

Instalación y uso: [docs/instalacion-local.md](docs/instalacion-local.md). Cómo terminar macOS: [docs/macos-agente.md](docs/macos-agente.md). Plan: [plan-macropad-agent.md](plan-macropad-agent.md).
