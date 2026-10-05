# Instalación local

## Requisitos

- Rust (`cargo`) y `libusb` 1.0.
- `ch57x-keyboard-tool`: `cargo install ch57x-keyboard-tool`.
  El agente lo busca en el `PATH`, en `~/.cargo/bin`, en Homebrew o en `MACROPAD_CH57X_TOOL`.

## Linux

### Instalación desde el código (cualquier distribución, incluida Arch)

```bash
installers/linux/install.sh
```

El script:

1. Compila e instala `~/.local/bin/macropad-agent` (el agente) y `~/.local/bin/macropad-agent-desktop` (la ventana), y añade «MacroPad Agent» al menú de aplicaciones.
2. Si falta, instala la regla `udev` `/etc/udev/rules.d/70-macropad-agent.rules`, limitada al dispositivo `1189:8890`. Este paso pide `sudo`.
3. Instala, activa y reinicia el servicio de usuario `macropad-agent.service`.

Vuelve a ejecutarlo cada vez que actualices el código.

### Paquete `.deb` (Debian y Ubuntu)

```bash
cargo install tauri-cli --version "^2" --locked   # una sola vez
installers/linux/build-deb.sh
```

Genera `target/release/bundle/deb/MacroPad Agent_<versión>_amd64.deb`. El paquete se llama `macro-pad-agent` e incluye:

- `/usr/bin/macropad-agent` y `/usr/bin/macropad-agent-desktop`.
- `ch57x-keyboard-tool` en `/usr/lib/macropad-agent/`, así que no hay que instalarlo aparte.
- La regla `udev` y el servicio de usuario. El servicio se activa para todas las sesiones.

Para instalar solo la regla `udev`: `installers/linux/install-udev.sh`.

Para ver el registro del agente: `journalctl --user -u macropad-agent -f`.

Para desinstalar: `installers/linux/uninstall.sh`. Tus perfiles y el historial se conservan.

## macOS

Probado en macOS 27 (Apple Silicon). No hace falta `sudo` ni conceder permisos para programar el teclado.

### App (`.dmg`)

```bash
cargo install tauri-cli --version "^2" --locked   # una sola vez
cd crates/desktop && cargo tauri build --bundles app,dmg
```

Genera `target/release/bundle/dmg/MacroPad Agent_<versión>_aarch64.dmg`. Arrastra la app a Aplicaciones y ábrela. Al abrirse instala el LaunchAgent, que apunta al agente de dentro de la `.app`. `ch57x-keyboard-tool` y libusb van incluidos, así que no hace falta Homebrew.

La app no está notarizada: la primera vez ábrela con clic derecho → *Abrir*, o ejecuta `xattr -dr com.apple.quarantine "/Applications/MacroPad Agent.app"`.

### Desde el código

```bash
installers/macos/install.sh
```

Instala `~/.local/bin/macropad-agent` y el LaunchAgent `com.macropad-agent`. Si `rustup` viene de Homebrew, agrega `/opt/homebrew/opt/rustup/bin` y `~/.cargo/bin` al `PATH`.

Registro del agente: `~/Library/Application Support/MacroPad Agent/agent.log`.

Detalles y pendientes: [macos-agente.md](macos-agente.md).

## Ventana de configuración

Ábrela desde el menú de aplicaciones o con `macropad-agent-desktop`. El agente sigue funcionando aunque la ventana esté cerrada.

- **Estado:** conexión del macro pad, permisos, agente activo y último perfil aplicado (o el último fallo).
- **Perfiles:** lista con insignias (*automático*, *incluido*, *modificado*). «+ Nuevo» crea una copia del perfil abierto.
- **Editor:** las 6 teclas y la perilla (izquierda, pulsar, derecha). Al hacer clic en una se abre el panel con acciones comunes por categoría, un constructor de combinaciones, teclas multimedia y valor directo. La validación con `ch57x-keyboard-tool` es inmediata.
- **Botones:**
  - Guardar (`Ctrl+S`).
  - Aplicar ahora.
  - Usar como automático: el agente lo aplica en la siguiente conexión.
  - Restaurar perfil de desarrollo.
  - Eliminar o descartar modificaciones.
- **Diagnóstico:** las mismas comprobaciones que `macropad-agent diagnose`, el historial, «Copiar» y «Reiniciar agente».

Al modificar un perfil incluido se guarda una copia en tu carpeta de perfiles. «Descartar modificaciones» vuelve a la versión original.

## Uso

```bash
macropad-agent diagnose                # revisa el hardware, los permisos y la herramienta
macropad-agent diagnose --apply-test   # además programa el perfil de prueba (las teclas escriben 1–6)
macropad-agent status
macropad-agent apply desarrollo-linux
macropad-agent validate mi-perfil      # muestra cómo queda cada control, sin programar
macropad-agent profiles
macropad-agent actions                 # acciones con nombre: guardar, deshacer, paleta-comandos…
macropad-agent history
macropad-agent run                     # el agente (lo ejecuta el servicio)
```

## Perfiles y configuración

| Sistema | Ubicación |
|---|---|
| Linux | `~/.config/macropad-agent/` |
| macOS | `~/Library/Application Support/MacroPad Agent/` |

- `profiles/*.yaml`: perfiles propios. Un perfil con el mismo nombre que uno incluido lo reemplaza.
- `config.yaml`: opcional.

  ```yaml
  auto_profile:
    linux: desarrollo-linux
    macos: desarrollo-macos
  settle_delay_ms: 1500
  notifications: true
  ```

- `state.json`: el último perfil aplicado con éxito.
- `history.jsonl`: todos los intentos.

Cada control de un perfil acepta dos tipos de valor:

- Una **acción con nombre** (`guardar`, `rehacer`, `volume_up`…), que se traduce a `ctrl` o `cmd` según la plataforma del perfil.
- Cualquier valor de `ch57x-keyboard-tool show-keys`, por ejemplo `ctrl-alt-t` o `f5`.

Las teclas multimedia no se pueden combinar con modificadores.

## Notas del hardware

- Disposición: 2 filas × 3 teclas y una perilla a la derecha. La fila superior es `key_1`–`key_3`.
- El protocolo solo permite **escribir**: no hay forma de leer el perfil guardado. Por eso el agente lo vuelve a aplicar en cada conexión y al iniciar sesión.
- Solo se programa la capa 1 de las tres que tiene el teclado.
