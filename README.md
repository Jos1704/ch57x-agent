# MacroPad Agent

Agente local para Linux y macOS que detecta el macro pad USB con chip **CH57x** (`1189:8890`) y le programa el perfil del sistema operativo. Cada tecla puede ser un atajo, una tecla multimedia, un comando o un flujo de varios pasos (abrir terminales y el editor, desplegar, etc.), con una biblioteca de preconfigurados. No usa cuentas, red ni telemetría.

<!-- Foto o GIF del macro pad: guárdala en docs/img/ y descomenta la línea.
![Macro pad CH57x con MacroPad Agent](docs/img/macropad.jpg)
-->

### ¿Es tu macro pad?

Es el macro pad genérico que se vende sin marca en AliExpress, Amazon y similares, y que de fábrica se programa con un software solo para Windows. MacroPad Agent soporta el modelo de **6 teclas y 1 perilla**. Para saber si el tuyo usa el mismo chip, conéctalo y busca el ID `1189:8890`:

```bash
lsusb | grep 1189:8890                          # Linux
system_profiler SPUSBDataType | grep -A2 0x8890  # macOS
```

Si aparece y tiene 6 teclas y una perilla, MacroPad Agent puede programarlo. Si tienes otra disposición, abre un issue.

### Estructura

- `crates/core`: perfiles, catálogo de acciones, comandos y flujos, biblioteca, detección USB y escucha de teclas, programación (usa `ch57x-keyboard-tool`).
- `crates/daemon`: binario `macropad-agent` (CLI y agente en segundo plano).
- `crates/desktop`: ventana de configuración Tauri (`macropad-agent-desktop`).
- `profiles/`: perfiles incluidos.
- `installers/`: regla `udev`, servicio `systemd`, paquete `.deb` y LaunchAgent.

## Linux

Probado en Arch Linux (Omarchy, Hyprland). Debe funcionar en cualquier distribución con `systemd` y una sesión gráfica (Wayland o X11).

Se instala desde el código, para tu usuario: no toca nada del sistema salvo una regla `udev` limitada al macro pad.

### 1. Instalar lo necesario

**Dependencias del sistema.** Las primeras sirven para compilar la ventana y hablar con el USB; `libnotify` y `xdg-utils` dan las notificaciones y abren los enlaces de los flujos:

```bash
# Debian / Ubuntu (22.04 o posterior)
sudo apt install build-essential pkg-config git curl libwebkit2gtk-4.1-dev libusb-1.0-0-dev libssl-dev libnotify-bin xdg-utils

# Fedora
sudo dnf install gcc pkgconf-pkg-config git curl webkit2gtk4.1-devel libusb1-devel openssl-devel libnotify xdg-utils

# Arch / Omarchy
sudo pacman -S --needed base-devel git webkit2gtk-4.1 libusb libnotify xdg-utils
```

Para los pasos «Terminal» de los flujos sirve la terminal que ya tengas: GNOME Terminal, Ptyxis, Konsole, Kitty, Alacritty, Foot, WezTerm, xfce4-terminal o xterm.

**Rust 1.88 o posterior.** Los paquetes de Rust de algunas distribuciones son más viejos; lo más seguro es `rustup`:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustc --version        # debe decir 1.88 o más
```

**La herramienta que programa el teclado:**

```bash
cargo install ch57x-keyboard-tool
```

### 2. Instalar MacroPad Agent

Conecta el macro pad por cable y ejecuta:

```bash
git clone https://github.com/Jos1704/MacroPad-Agent.git
cd MacroPad-Agent
installers/linux/install.sh
```

El script:

1. Compila el agente y la ventana (la primera vez tarda unos minutos).
2. Los instala en `~/.local/bin` y añade **MacroPad Agent** al menú de aplicaciones.
3. Instala la regla `udev` `/etc/udev/rules.d/70-macropad-agent.rules`, que deja a tu usuario programar el pad y leer sus teclas. **Aquí pide tu contraseña**, en la terminal o en una ventana del sistema.
4. Activa el servicio de usuario `macropad-agent`, que arranca con tu sesión.

Al terminar, **desconecta y vuelve a conectar el macro pad**. El agente lo programa con el perfil `desarrollo-linux` y muestra una notificación.

### 3. Comprobar que funciona

```bash
macropad-agent diagnose
```

Debe terminar con «Todo listo». Para comprobar también el hardware:

```bash
macropad-agent diagnose --apply-test   # cada tecla escribe su número: 1–6
macropad-agent apply desarrollo-linux  # vuelve al perfil de desarrollo
```

### 4. Usarlo

- **Ventana de configuración:** ábrela desde el menú de aplicaciones («MacroPad Agent») o con `macropad-agent-desktop`. Haz clic en una tecla para elegir qué hace: un atajo, multimedia, un comando o un flujo de varios pasos. En **Biblioteca** hay flujos listos (empezar a trabajar, desplegar, ver logs, pruebas…). Guarda el perfil y listo: el teclado se reprograma solo si hace falta.
- **El agente** corre en segundo plano aunque la ventana esté cerrada: programa el pad al conectarlo y ejecuta los comandos y flujos de sus teclas.
- **Línea de comandos:**

  ```bash
  macropad-agent status              # conexión, perfil aplicado y agente
  macropad-agent profiles            # perfiles disponibles
  macropad-agent apply <perfil>      # programar un perfil ahora
  macropad-agent ejecutar key_6      # correr el comando de una tecla sin pulsarla
  macropad-agent comandos            # historial de comandos ejecutados
  journalctl --user -u macropad-agent -f   # registro del agente
  ```

### Actualizar

```bash
cd MacroPad-Agent
git pull
installers/linux/install.sh
```

Tus perfiles y tu biblioteca están en `~/.config/macropad-agent/` y no se tocan al actualizar.

### Problemas comunes

| Síntoma | Solución |
|---|---|
| `macropad-agent: command not found` | Agrega `~/.local/bin` a tu `PATH` (por ejemplo, en `~/.bashrc`: `export PATH="$HOME/.local/bin:$PATH"`). |
| `Access denied` o «SIN permisos» en `diagnose` | Desconecta y vuelve a conectar el pad. Si sigue, ejecuta `installers/linux/install-udev.sh`. |
| Los comandos no se ejecutan al pulsar las teclas | En la ventana, usa «Dar permiso» si aparece el aviso; luego `systemctl --user restart macropad-agent`. Revisa `macropad-agent diagnose`, sección «Comandos y flujos». |
| No aparece en el menú de aplicaciones | Cierra sesión y vuelve a entrar. En Omarchy: `omarchy restart shell`. |
| `no se encontró ch57x-keyboard-tool` | `cargo install ch57x-keyboard-tool` (queda en `~/.cargo/bin`, que el agente ya revisa). |
| Error al compilar por `webkit2gtk-4.1` o `libusb` | Faltan las dependencias del paso 1. |
| La ventana se abre en blanco (pasa con algunas tarjetas NVIDIA) | Ábrela con `WEBKIT_DISABLE_DMABUF_RENDERER=1 macropad-agent-desktop`. |

### Desinstalar

```bash
installers/linux/uninstall.sh
```

Quita el agente, la ventana, el servicio y la regla `udev`. Conserva `~/.config/macropad-agent/`; bórrala a mano si no la quieres.

### Paquete `.deb` (opcional)

Para instalar en Debian o Ubuntu sin compilar en cada equipo, genera un paquete en un equipo con todo lo anterior:

```bash
cargo install tauri-cli --version "^2" --locked
installers/linux/build-deb.sh
```

Sale en `target/release/bundle/deb/` e incluye `ch57x-keyboard-tool`, así que en el otro equipo basta con `sudo apt install ./MacroPad*.deb`.

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

## Contribuir

Los issues y pull requests son bienvenidos, sobre todo:

- **Otros modelos del macro pad CH57x** (otra cantidad de teclas o perillas): abre un issue con la salida de `lsusb` y una foto.
- **Flujos y perfiles para la biblioteca** que te sirvan en tu día a día.
- **Pruebas en otras distribuciones**: cuéntanos en qué distribución y entorno gráfico funcionó, o qué falló.

Antes de abrir un pull request, ejecuta `cargo test`.

## Créditos

La programación del teclado la hace [`ch57x-keyboard-tool`](https://github.com/kriomant/ch57x-keyboard-tool) (MIT o Apache-2.0).

## Licencia

MacroPad Agent se distribuye bajo la licencia MIT; consulta [LICENSE](LICENSE).

Los paquetes `.deb` y `.app` incluyen `ch57x-keyboard-tool`. La `.app` de macOS lleva además [libusb](https://libusb.info) enlazada de forma estática, que se distribuye bajo la licencia [LGPL-2.1](https://github.com/libusb/libusb/blob/master/COPYING). Su código fuente está en <https://github.com/libusb/libusb>. Puedes volver a enlazar la app con otra versión de libusb compilándola desde este repositorio, como se explica en «Correr desde el código».
