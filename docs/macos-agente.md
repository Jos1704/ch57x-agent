# Guía para terminar MacroPad Agent en macOS

Este documento es para quien continúe el trabajo en un Mac, ya sea una persona o un agente como Claude Code. Explica qué ya funciona, qué falta, qué archivos tocar y cómo comprobar cada paso. Léelo junto con `plan-macropad-agent.md`.

## Estado actual

| Pieza | Linux | macOS |
|---|---|---|
| Núcleo (`crates/core`): perfiles, acciones, USB, historial | ✅ probado con el teclado real | ✅ probado con el teclado real (macOS 27, Apple Silicon) |
| CLI `macropad-agent` (`status`, `apply`, `diagnose`, …) | ✅ probado | ✅ probado, sin `sudo` |
| Agente `macropad-agent run` (hotplug con libusb) | ✅ probado: arranque y reconexión | ✅ probado: arranque y reconexión |
| Inicio automático | ✅ servicio de usuario `systemd` | ✅ LaunchAgent (`install.sh` o la `.app`); falta probar cerrar e iniciar sesión |
| Ventana Tauri (`crates/desktop`) | ✅ probada | ✅ editar, guardar y aplicar; faltan algunos botones (paso 7) |
| Paquete | ✅ `.deb` | ✅ `.app` y `.dmg` (arm64), sin depender de Homebrew |
| Perfil `desarrollo-macos` (`cmd-…`) | ✅ validado con `ch57x-keyboard-tool validate` | ✅ probado en el teclado con VS Code |

### Datos confirmados del hardware (en Linux)

- USB `1189:8890`, modelo `ch57x-2` en `ch57x-keyboard-tool` 1.8.0.
- 4 interfaces HID: 0 teclado (`0x81` IN), **1 programación (`0x02` OUT)**, 2 HID genérico (`0x83` IN), 3 ratón (`0x82` IN).
- Disposición: 2 filas × 3 teclas, perilla a la derecha. El perfil `prueba` escribe 1–6 en orden. Esto ya se comprobó físicamente.
- **El teclado solo permite escribir.** No hay comando para leer el perfil guardado, así que el agente reaplica el perfil en cada conexión.
- Solo se programa la capa 1.

## Código que depende del sistema operativo

Todo lo que cambia entre Linux y macOS está en estos lugares; el resto es común.

| Archivo | Qué hace en macOS |
|---|---|
| `crates/core/src/platform.rs` (`Platform::current`) | Devuelve `macos`, y así el perfil automático es `desarrollo-macos`. |
| `crates/core/src/paths.rs` (`config_dir`) | Usa `~/Library/Application Support/MacroPad Agent/`. |
| `crates/core/src/service.rs` | `launchctl list com.macropad-agent` para saber el estado; `launchctl kickstart -k gui/<uid>/com.macropad-agent` para reiniciar. |
| `crates/core/src/tool.rs` (`locate`) | Busca `ch57x-keyboard-tool` junto al ejecutable (dentro del `.app`), en `PATH`, `~/.cargo/bin`, `/opt/homebrew/bin` y `/usr/local/bin`. |
| `crates/core/src/diagnose.rs` | Los mensajes de la regla `udev` solo aparecen en Linux. **Hay que agregar los consejos de permisos de macOS** (ver paso 3). |
| `crates/daemon/src/notify.rs` | Muestra notificaciones con `osascript -e 'display notification …'`. |
| `installers/macos/` | `com.macropad-agent.plist` (plantilla) e `install.sh`. |

## Pasos

Ejecuta cada paso desde la raíz del repositorio. No pases al siguiente hasta que el anterior funcione.

### 1. Preparar el entorno

```bash
xcode-select --install                 # si no están las herramientas de línea de comandos
brew install rustup-init libusb pkg-config && rustup-init -y
cargo install ch57x-keyboard-tool
cargo install tauri-cli --version "^2" --locked
```

**Comprobar:** `cargo test` pasa (9 tests en `macropad-core`) y `cargo build` termina sin advertencias.

> **Resultado (2026-10-02, Apple Silicon):** ✅. Homebrew ya no incluye `rustup-init`: la fórmula instala `rustup` como keg-only en `/opt/homebrew/opt/rustup/bin`. Ejecuta `rustup default stable` y agrega al `PATH` `/opt/homebrew/opt/rustup/bin` y `~/.cargo/bin`.

### 2. Diagnóstico del hardware

Conecta el macro pad por cable y ejecuta:

```bash
cargo run -p macropad-agent -- diagnose
```

**Comprobar:** se detecta el dispositivo `1189:8890` con las mismas 4 interfaces de arriba.

> **Resultado (2026-10-02):** ✅. Se detectan las mismas 4 interfaces, también a través de un hub USB 2.0. El dispositivo no expone nombre de producto (en `ioreg` solo aparece como `IOUSBHostDevice`). `Found::usb_path()` mostraba la ruta de Linux `/dev/bus/usb/…`; en macOS ahora muestra `USB bus N, dirección N`.

### 3. Permisos de USB en macOS (lo más incierto)

En Linux hizo falta una regla `udev`. En macOS no existe ese mecanismo. El sistema reclama las interfaces HID para sí, y libusb no puede «desconectar el driver del kernel» como en Linux.

```bash
cargo run -p macropad-agent -- diagnose --apply-test
```

- **Funciona sin `sudo`** (las teclas escriben 1–6): no hace falta más. Anótalo en este documento.
- **Falla con `Access denied`, `claim interface` o `open USB device`:**
  1. Prueba `sudo ~/.cargo/bin/ch57x-keyboard-tool upload` con la salida de `macropad-agent validate prueba` para aislar el problema. Si con `sudo` funciona, es un tema de permisos y no de protocolo.
  2. Revisa *Ajustes del Sistema → Privacidad y seguridad → Monitorización de entrada* y *Accesorios*. Autoriza la terminal o la app si aparece.
  3. Revisa los issues de <https://github.com/kriomant/ch57x-keyboard-tool> sobre macOS y el modelo 8890.
  4. Documenta la solución aquí y agrega en `crates/core/src/diagnose.rs` un consejo para `Access::Denied` con `cfg!(target_os = "macos")`, igual que el de `udev`.

No cambies el protocolo USB: la programación la hace `ch57x-keyboard-tool`.

> **Resultado (2026-10-02, macOS 27):** ✅ **funciona sin `sudo`** y sin conceder permisos en Ajustes del Sistema. `diagnose --apply-test` programó el perfil `prueba` y las teclas escriben 1–6 en orden. La perilla sube y baja el volumen, y al pulsarla silencia. No hizo falta el consejo para `Access::Denied` en `diagnose.rs`.

### 4. Perfil de desarrollo de macOS

```bash
cargo run -p macropad-agent -- apply desarrollo-macos
```

**Comprobar en VS Code:**

- Tecla 1 guarda (`Cmd+S`).
- Tecla 2 deshace y tecla 3 rehace (`Cmd+Shift+Z`).
- Tecla 4 abre archivo (`Cmd+P`) y tecla 5 abre la paleta (`Cmd+Shift+P`).
- Tecla 6 envía `F5`. En macOS `F5` puede ser una tecla multimedia del sistema; si pasa, documéntalo.
- La perilla sube y baja el volumen, y al pulsarla silencia.

> **Resultado (2026-10-02):** ✅ las 6 teclas y la perilla funcionan como se espera en VS Code. `cmd` funciona como Command, y `F5` llega a VS Code sin activar ninguna función del sistema.

Si `cmd` no funciona como Command, revisa `actions.rs`. Las acciones de macOS usan `cmd-…`, que en `ch57x-keyboard-tool` es la tecla GUI izquierda.

### 5. Agente en primer plano

```bash
cargo run -p macropad-agent -- run
```

**Comprobar:**

- Al arrancar dice `escuchando eventos USB (hotplug)`. Si dice `hotplug no disponible`, revisa cada 2 s; también es válido, pero anótalo.
- Desconecta y conecta el teclado: debe aparecer `conectado: … (connect)` y `perfil «Desarrollo macOS» aplicado`, además de una notificación.
- `cargo run -p macropad-agent -- history` muestra los intentos.

> **Resultado (2026-10-02):** ✅. El hotplug de libusb funciona en macOS (`escuchando eventos USB (hotplug)`). Al arrancar aplica el perfil (`startup`) y al reconectar también (`connect`). Las notificaciones de `osascript` llegan, y `history` registra todos los intentos.

### 6. Inicio automático (LaunchAgent)

```bash
installers/macos/install.sh
```

Instala `~/.local/bin/macropad-agent` y `~/Library/LaunchAgents/com.macropad-agent.plist`.

**Comprobar:**

- `launchctl list com.macropad-agent` existe y `macropad-agent status` dice `Agente: activo`.
- El registro está en `~/Library/Application Support/MacroPad Agent/agent.log`.
- Cierra sesión y vuelve a entrar: el perfil se aplica solo (trigger `startup`).
- Las notificaciones funcionan desde el LaunchAgent.

> **Resultado (2026-10-02):** ✅ `launchctl list`, `status` (`Agente: activo`), `agent.log`, la reconexión y `launchctl kickstart -k` funcionan. launchd encuentra `ch57x-keyboard-tool` en `~/.cargo/bin` sin `MACROPAD_CH57X_TOOL`. **Pendiente:** cerrar sesión y volver a entrar.

Si launchd no encuentra `ch57x-keyboard-tool`, define `MACROPAD_CH57X_TOOL` en el plist con `EnvironmentVariables`.

### 7. Ventana de configuración

```bash
cargo run -p macropad-agent-desktop
```

**Comprobar:**

- El estado arriba muestra la conexión, el agente y el último perfil aplicado.
- Puedes editar una tecla, guardar, aplicar y usar el perfil como automático.
- Funcionan «Restaurar perfil de desarrollo» y el diagnóstico con «Copiar».
- «Reiniciar agente» funciona; usa `launchctl kickstart`.

Las combinaciones del editor muestran `super` en Linux; en macOS deben decir `cmd`. Revisa `MOD_LABEL` en `crates/desktop/ui/app.js`.

> **Resultado (2026-10-02):** ✅ la ventana abre, edita una tecla, guarda (la copia queda en `profiles/`) y aplica. «Reiniciar agente» usa `launchctl kickstart -k`, que se probó desde la terminal. `MOD_LABEL` ya muestra `cmd` y `opt` en perfiles de macOS. **Pendiente:** probar en la ventana «Usar como automático», «Restaurar perfil de desarrollo», «Copiar» del diagnóstico y «Reiniciar agente».

### 8. Empaquetar `.app` y `.dmg`

`crates/desktop/tauri.conf.json` hoy solo genera `deb`. Para macOS:

1. Agrega en `bundle` una sección `macOS` con `files`, para meter el agente y la herramienta dentro del `.app`:

   ```json
   "macOS": {
     "files": {
       "MacOS/macropad-agent": "../../target/release/macropad-agent",
       "MacOS/ch57x-keyboard-tool": "../../target/release/ch57x-keyboard-tool"
     },
     "minimumSystemVersion": "11.0"
   }
   ```

   `tool::locate()` ya busca `ch57x-keyboard-tool` junto al ejecutable.

2. `beforeBundleCommand` ejecuta `installers/common/prepare.sh`, que compila el agente y copia la herramienta a `target/release`. El comando se ejecuta desde `crates/`.

3. Compila con:

   ```bash
   cd crates/desktop && cargo tauri build --bundles app,dmg
   ```

4. Haz que el LaunchAgent apunte al binario dentro del `.app`: `/Applications/MacroPad Agent.app/Contents/MacOS/macropad-agent`. Lo ideal es que la app instale el LaunchAgent en su primera ejecución, como pide la fase 4 del plan («solicitará los permisos… durante la primera ejecución»). Puede ser un comando Tauri `install_agent` que escriba el plist y ejecute `launchctl bootstrap gui/<uid>`.

5. Sin firma de Apple, Gatekeeper bloqueará la app. Para uso personal basta con `xattr -dr com.apple.quarantine "/Applications/MacroPad Agent.app"`, o con abrirla una vez desde *Abrir* en el menú contextual.

**Comprobar:** instala el `.dmg` y arrastra la app a Aplicaciones. Ábrela: el agente queda instalado y el teclado se programa al reconectarlo.

> **Resultado (2026-10-02):** ✅ se generan `MacroPad Agent.app` y `MacroPad Agent_0.1.0_aarch64.dmg`. Cómo quedó cada punto:
>
> - **libusb:** la de Homebrew exige la versión de macOS del equipo (`minos 26.0`) y no viene en un Mac sin Homebrew. En macOS se activa `rusb/vendored` (en `crates/core/Cargo.toml`, solo para `target_os = "macos"`), que compila libusb 1.0.27 estática. `prepare.sh` recompila `ch57x-keyboard-tool` igual (`--features rusb/vendored`). Los tres binarios de la `.app` usan solo librerías del sistema y piden macOS 11.0 o posterior.
> - **Firma:** `"signingIdentity": "-"` firma la `.app` completa ad-hoc. Sin esto, un Mac con Apple Silicon la marca como «dañada». Sigue sin notarizar, así que la primera vez hay que abrirla con *Abrir* o usar `xattr`.
> - **LaunchAgent:** cada vez que abre, la ventana llama a `service::install_launch_agent`, solo si corre desde `/Applications` o `~/Applications`. Escribe el plist con la plantilla de `installers/macos/` apuntando al agente de la `.app`, y lo recarga solo si cambió. Reemplaza el LaunchAgent de `install.sh`. ✅ probado: el agente arranca desde la `.app` y aplica el perfil al arrancar y al reconectar.
> - **`.dmg`:** `bundle_dmg.sh` falló una vez sin motivo claro y dejó montado un volumen `rw.*.dmg`. Se arregla con `hdiutil detach` y repitiendo la compilación.
> - **Solo arm64:** para Mac con Intel haría falta `--target universal-apple-darwin`.

## Cómo terminar

Considera macOS terminado cuando:

- [ ] Los pasos 1 a 8 funcionan y quedan anotadas las sorpresas (sobre todo permisos y `F5`).
- [ ] `cargo test` y `cargo clippy --workspace` siguen limpios en macOS.
- [ ] En Linux sigue todo igual. Todo lo nuevo de macOS va con `cfg!(target_os = "macos")`; no se cambia el comportamiento de Linux.
- [ ] `docs/instalacion-local.md` y la tabla de estado de este documento están al día.
