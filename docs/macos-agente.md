# Guía para terminar MacroPad Agent en macOS

Este documento es para quien continúe el trabajo en un Mac, ya sea una persona o un agente como Claude Code. Explica qué ya funciona, qué falta, qué archivos tocar y cómo comprobar cada paso. Léelo junto con `plan-macropad-agent.md`.

## Estado actual

| Pieza | Linux | macOS |
|---|---|---|
| Núcleo (`crates/core`): perfiles, acciones, USB, historial | ✅ probado con el teclado real | compila con `cfg!`, **sin probar** |
| CLI `macropad-agent` (`status`, `apply`, `diagnose`, …) | ✅ probado | **sin probar** |
| Agente `macropad-agent run` (hotplug con libusb) | ✅ probado: arranque y reconexión | **sin probar** |
| Inicio automático | ✅ servicio de usuario `systemd` | `installers/macos/install.sh` + LaunchAgent, **sin probar** |
| Ventana Tauri (`crates/desktop`) | ✅ probada | **sin probar**; falta empaquetar `.app` y `.dmg` |
| Perfil `desarrollo-macos` (`cmd-…`) | ✅ validado con `ch57x-keyboard-tool validate` | falta probarlo en el teclado |

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

### 2. Diagnóstico del hardware

Conecta el macro pad por cable y ejecuta:

```bash
cargo run -p macropad-agent -- diagnose
```

**Comprobar:** se detecta el dispositivo `1189:8890` con las mismas 4 interfaces de arriba.

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

Si `cmd` no funciona como Command, revisa `actions.rs`. Las acciones de macOS usan `cmd-…`, que en `ch57x-keyboard-tool` es la tecla GUI izquierda.

### 5. Agente en primer plano

```bash
cargo run -p macropad-agent -- run
```

**Comprobar:**

- Al arrancar dice `escuchando eventos USB (hotplug)`. Si dice `hotplug no disponible`, revisa cada 2 s; también es válido, pero anótalo.
- Desconecta y conecta el teclado: debe aparecer `conectado: … (connect)` y `perfil «Desarrollo macOS» aplicado`, además de una notificación.
- `cargo run -p macropad-agent -- history` muestra los intentos.

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

2. `beforeBundleCommand` ejecuta `installers/linux/deb/prepare.sh`. Es POSIX y sirve también en macOS (compila el agente y copia la herramienta a `target/release`). Conviene moverlo a `installers/common/prepare.sh` y actualizar la ruta en `tauri.conf.json`. Recuerda que el comando se ejecuta desde `crates/`.

3. Compila con:

   ```bash
   cd crates/desktop && cargo tauri build --bundles app,dmg
   ```

4. Haz que el LaunchAgent apunte al binario dentro del `.app`: `/Applications/MacroPad Agent.app/Contents/MacOS/macropad-agent`. Lo ideal es que la app instale el LaunchAgent en su primera ejecución, como pide la fase 4 del plan («solicitará los permisos… durante la primera ejecución»). Puede ser un comando Tauri `install_agent` que escriba el plist y ejecute `launchctl bootstrap gui/<uid>`.

5. Sin firma de Apple, Gatekeeper bloqueará la app. Para uso personal basta con `xattr -dr com.apple.quarantine "/Applications/MacroPad Agent.app"`, o con abrirla una vez desde *Abrir* en el menú contextual.

**Comprobar:** instala el `.dmg` y arrastra la app a Aplicaciones. Ábrela: el agente queda instalado y el teclado se programa al reconectarlo.

## Cómo terminar

Considera macOS terminado cuando:

- [ ] Los pasos 1 a 8 funcionan y quedan anotadas las sorpresas (sobre todo permisos y `F5`).
- [ ] `cargo test` y `cargo clippy --workspace` siguen limpios en macOS.
- [ ] En Linux sigue todo igual. Todo lo nuevo de macOS va con `cfg!(target_os = "macos")`; no se cambia el comportamiento de Linux.
- [ ] `docs/instalacion-local.md` y la tabla de estado de este documento están al día.
