# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Proyecto

Agente local (Linux y macOS) que detecta el macro pad USB CH57x `1189:8890` (6 teclas + 1 perilla) y lo programa con el perfil del sistema operativo. Sin red, cuentas ni telemetría. Todo el código, los mensajes, la documentación y los identificadores de dominio están en **español**; mantén ese idioma.

## Comandos

Requisitos: Rust ≥ 1.88 (edición 2024), `cargo install ch57x-keyboard-tool` y, para la ventana, `cargo install tauri-cli --version "^2" --locked`. En Linux también `webkit2gtk-4.1` y `libusb` (ver README).

```bash
cargo test                                   # obligatorio antes de un PR; las pruebas viven en crates/core
cargo test -p macropad-core <nombre>         # una sola prueba (filtro por nombre)
cargo build

cargo run -p macropad-agent -- diagnose      # detecta el pad y revisa permisos/herramienta
cargo run -p macropad-agent -- validate desarrollo-linux   # resuelve y valida sin programar
cargo run -p macropad-agent -- apply desarrollo-linux      # programa el pad (debe estar conectado)
cargo run -p macropad-agent -- ejecutar key_1              # ejecuta el comando/flujo de un control
cargo run -p macropad-agent -- run           # agente en primer plano
cargo run -p macropad-agent-desktop          # ventana Tauri

installers/linux/install.sh                  # instala en ~/.local/bin + udev + servicio systemd de usuario
installers/linux/build-deb.sh                # paquete .deb en target/release/bundle/deb/
cd crates/desktop && cargo tauri build --bundles app,dmg   # macOS
```

`MACROPAD_AGENT_HOME` redirige el directorio de configuración (útil para probar sin tocar `~/.config/macropad-agent`). `MACROPAD_CH57X_TOOL` fuerza la ruta de `ch57x-keyboard-tool`.

## Arquitectura

Workspace de tres crates:

- **`crates/core` (`macropad-core`)**: toda la lógica. Los otros dos crates son capas finas sobre él.
- **`crates/daemon` (binario `macropad-agent`)**: CLI con `clap` (`main.rs`) y el agente en segundo plano (`agent.rs`: hotplug de libusb con respaldo por sondeo, deduplicación de llegadas, reintentos, ejecución de automatizaciones en `exec.rs`, notificaciones en `notify.rs`).
- **`crates/desktop` (binario `macropad-agent-desktop`)**: Tauri 2. `src/main.rs` expone comandos `#[tauri::command]` que llaman a `macropad-core`; la UI es HTML/CSS/JS plano en `ui/` sin paso de build (`withGlobalTauri`, `invoke(...)` en `app.js`). Al añadir un comando hay que registrarlo en `generate_handler!`.

### Flujo de un perfil

1. `Profile::load(id)` (`profile.rs`): busca primero en el directorio de perfiles del usuario y luego en los incluidos, que se embeben con `include_str!` desde `profiles/*.yaml` (añadir un perfil incluido = añadir el YAML y la entrada en `BUILTIN`). Un argumento con `/` o `.yaml` se trata como ruta.
2. Cada control (`key_1`…`key_6`, `knob_left`, `knob_right`, `knob_press`) es un `Binding` (`automation.rs`, serde `untagged`): una cadena de teclas, un `command:` o un `flow:`.
3. Las cadenas se resuelven con `actions.rs`: acciones con nombre (`guardar`, `paleta-comandos`…) se traducen por plataforma (`ctrl-s` / `cmd-s`); cualquier otra cosa pasa tal cual a `ch57x-keyboard-tool`.
4. `to_ch57x_yaml` genera la configuración y `tool.rs` la valida y sube invocando `ch57x-keyboard-tool` como proceso externo (el protocolo USB no está implementado aquí).
5. `apply::apply_profile` registra el resultado en `state.rs` (`state.json` + `history.jsonl`) y guarda la config subida en `applied.yaml`; `needs_reapply` compara contra ese archivo.

**El teclado solo admite escritura** (no se puede leer lo programado), por eso el agente reaplica el perfil en cada conexión/arranque.

### Comandos y flujos

El pad solo almacena pulsaciones. Un control con `command:`/`flow:` se programa para enviar una tecla reservada (`CONTROL_KEYS` en `automation.rs`: `F13`–`F19`, `Shift+F13/F14`). `input.rs` escucha **solo** los `/dev/input/event*` del pad (evdev, filtrando por vendor/product en sysfs) y el agente ejecuta lo asignado; el resto de teclados no se ve afectado. Cambiar solo el texto de un comando no requiere reprogramar; cambiar de atajo a comando sí.

`library.rs` + `templates/biblioteca.yaml`: plantillas preconfiguradas (con `needs` que piden `cwd` o variables de entorno) más las guardadas por el usuario en `biblioteca.yaml` del directorio de configuración. `runs.rs` guarda el historial de ejecuciones.

### Código dependiente del sistema

`platform.rs`, `paths.rs` (`~/.config/macropad-agent` vs `~/Library/Application Support/MacroPad Agent`), `service.rs` (systemd de usuario vs LaunchAgent `com.macropad-agent`), `tool.rs` (`locate`), `diagnose.rs`, `notify.rs`, `automation.rs` (shell/terminal/abrir enlaces) e `input.rs`. **En macOS falta implementar la escucha de teclas** (`input.rs` devuelve `Unsupported`); el estado y los pasos pendientes están en `docs/macos-agente.md`.

### Empaquetado

`tauri.conf.json` ejecuta `installers/common/prepare.sh` antes de empaquetar: compila `macropad-agent` en release y copia `ch57x-keyboard-tool` a `target/release/` para incluirlo en el `.deb`/`.app` (en macOS lo recompila con libusb estática). La regla udev `installers/linux/70-macropad-agent.rules` se embebe también en `input.rs`.

## Convenciones

- Los structs de perfil/plantilla usan `#[serde(deny_unknown_fields)]`: un campo nuevo en YAML exige añadirlo al struct.
- Errores con `anyhow` y `.context(...)` en español; el CLI imprime `error: {e:#}`.
