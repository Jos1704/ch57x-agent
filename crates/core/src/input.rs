//! Escucha de las teclas del pad.
//!
//! Linux: lee los dispositivos `/dev/input/event*` cuyo USB es `1189:8890`,
//! así que el resto de los teclados no se ve afectado. Requiere la regla udev
//! con `uaccess` para el subsistema `input`.
//!
//! macOS: pendiente (IOHIDManager + permiso de Monitorización de entrada);
//! ver `docs/macos-agente.md`.

use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Access {
    /// Se pueden leer las teclas del pad.
    Ok,
    /// El pad está conectado pero falta permiso.
    Denied,
    /// No hay dispositivos de entrada del pad (desconectado).
    NoDevice,
    /// Este sistema aún no escucha teclas.
    Unsupported,
}

/// Regla udev completa: programar el pad (usb) y leer sus teclas (input).
pub const UDEV_RULE: &str = include_str!("../../../installers/linux/70-macropad-agent.rules");
pub const UDEV_RULE_PATH: &str = "/etc/udev/rules.d/70-macropad-agent.rules";

#[cfg(target_os = "linux")]
mod imp {
    use std::fs::{self, File};
    use std::io::Read;
    use std::path::{Path, PathBuf};
    use std::sync::mpsc::Sender;
    use std::thread;

    use super::Access;
    use crate::automation;
    use crate::device::{PRODUCT_ID, VENDOR_ID};

    /// Dispositivos `/dev/input/eventN` del pad.
    pub fn devices() -> Vec<PathBuf> {
        let Ok(dir) = fs::read_dir("/sys/class/input") else { return Vec::new() };
        let read_hex = |p: PathBuf| {
            fs::read_to_string(p).ok().and_then(|s| u16::from_str_radix(s.trim(), 16).ok())
        };
        let mut found: Vec<PathBuf> = dir
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with("event"))
            .filter(|e| {
                let id = e.path().join("device/id");
                read_hex(id.join("vendor")) == Some(VENDOR_ID) && read_hex(id.join("product")) == Some(PRODUCT_ID)
            })
            .map(|e| Path::new("/dev/input").join(e.file_name()))
            .collect();
        found.sort();
        found
    }

    pub fn access() -> Access {
        let devices = devices();
        if devices.is_empty() {
            return Access::NoDevice;
        }
        if devices.iter().all(|d| File::open(d).is_ok()) { Access::Ok } else { Access::Denied }
    }

    /// Lee un dispositivo hasta que se desconecta y envía el control de cada
    /// tecla reservada pulsada. Bloquea: no consume CPU mientras no hay teclas.
    pub fn listen(path: PathBuf, tx: Sender<&'static str>) -> std::io::Result<thread::JoinHandle<()>> {
        let mut file = File::open(&path)?;
        Ok(thread::spawn(move || {
            let mut buf = [0u8; EVENT_SIZE * 16];
            let mut decoder = Decoder::default();
            loop {
                let n = match file.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                for control in decoder.feed(&buf[..n]) {
                    if tx.send(control).is_err() {
                        return;
                    }
                }
            }
        }))
    }

    /// `struct input_event` en 64 bits: timeval (16) + type (2) + code (2) + value (4).
    pub const EVENT_SIZE: usize = 24;

    /// Convierte eventos en controles. Recuerda si Shift está pulsado, porque
    /// algunos controles envían `Shift+F13`/`Shift+F14`.
    #[derive(Default)]
    pub struct Decoder {
        shift: [bool; 2],
    }

    impl Decoder {
        /// Controles cuya tecla reservada se pulsó en este bloque de eventos.
        pub fn feed(&mut self, bytes: &[u8]) -> Vec<&'static str> {
            const EV_KEY: u16 = 1;
            const KEY_LEFTSHIFT: u16 = 42;
            const KEY_RIGHTSHIFT: u16 = 54;
            let mut out = Vec::new();
            for ev in bytes.as_chunks::<EVENT_SIZE>().0 {
                let kind = u16::from_ne_bytes([ev[16], ev[17]]);
                let code = u16::from_ne_bytes([ev[18], ev[19]]);
                let value = i32::from_ne_bytes([ev[20], ev[21], ev[22], ev[23]]);
                if kind != EV_KEY {
                    continue;
                }
                match code {
                    KEY_LEFTSHIFT => self.shift[0] = value != 0,
                    KEY_RIGHTSHIFT => self.shift[1] = value != 0,
                    // value 1 = pulsación (2 = repetición, 0 = soltar).
                    _ if value == 1 => {
                        let shift = self.shift[0] || self.shift[1];
                        out.extend(automation::control_for_keycode(code, shift));
                    }
                    _ => {}
                }
            }
            out
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn event(kind: u16, code: u16, value: i32) -> Vec<u8> {
            let mut ev = vec![0u8; 16];
            ev.extend(kind.to_ne_bytes());
            ev.extend(code.to_ne_bytes());
            ev.extend(value.to_ne_bytes());
            ev
        }

        #[test]
        fn decodes_reserved_key_presses_only() {
            let mut bytes = Vec::new();
            bytes.extend(event(1, 188, 1)); // F18 pulsada → key_6
            bytes.extend(event(1, 188, 2)); // repetición
            bytes.extend(event(1, 188, 0)); // soltar
            bytes.extend(event(1, 31, 1)); // tecla «s» normal
            bytes.extend(event(4, 4, 458_771)); // MSC_SCAN
            bytes.extend(event(1, 42, 1)); // Shift
            bytes.extend(event(1, 183, 1)); // Shift+F13 → knob_press
            bytes.extend(event(1, 42, 0));
            bytes.extend(event(1, 183, 1)); // F13 → key_1
            bytes.extend(event(1, 190, 1)); // F20 no es de ningún control
            assert_eq!(Decoder::default().feed(&bytes), ["key_6", "knob_press", "key_1"]);
        }

        #[test]
        fn shift_state_survives_between_reads() {
            let mut d = Decoder::default();
            assert!(d.feed(&event(1, 54, 1)).is_empty()); // Shift derecho
            assert_eq!(d.feed(&event(1, 184, 1)), ["knob_right"]);
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod imp {
    use std::path::PathBuf;
    use std::sync::mpsc::Sender;
    use std::thread;

    use super::Access;

    pub fn devices() -> Vec<PathBuf> {
        Vec::new()
    }

    pub fn access() -> Access {
        Access::Unsupported
    }

    pub fn listen(_path: PathBuf, _tx: Sender<&'static str>) -> std::io::Result<thread::JoinHandle<()>> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "escucha de teclas no implementada en este sistema"))
    }
}

pub use imp::{access, listen};

pub fn devices() -> Vec<PathBuf> {
    imp::devices()
}

/// ¿La regla instalada ya incluye el permiso para leer las teclas?
pub fn udev_rule_current() -> bool {
    ["/etc/udev/rules.d/70-macropad-agent.rules", "/usr/lib/udev/rules.d/70-macropad-agent.rules"]
        .iter()
        .any(|p| std::fs::read_to_string(p).is_ok_and(|t| t.contains("SUBSYSTEM==\"input\"")))
}

/// Linux: instala la regla udev con `pkexec` (pide la contraseña en una
/// ventana) y la aplica a los dispositivos conectados.
pub fn grant_permission() -> anyhow::Result<()> {
    use anyhow::{bail, Context};
    if !cfg!(target_os = "linux") {
        bail!("en macOS el permiso se da en Ajustes › Privacidad y seguridad › Monitorización de entrada");
    }
    let script = format!(
        "cat > {UDEV_RULE_PATH} <<'EOF'\n{UDEV_RULE}EOF\nudevadm control --reload && udevadm trigger --subsystem-match=usb --attr-match=idVendor=1189 --attr-match=idProduct=8890 && udevadm trigger --subsystem-match=input"
    );
    let out = std::process::Command::new("pkexec")
        .args(["sh", "-c", &script])
        .output()
        .context("ejecutar pkexec")?;
    if !out.status.success() {
        bail!("no se instaló el permiso: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(())
}
