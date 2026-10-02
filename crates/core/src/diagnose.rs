//! Diagnóstico compartido por la CLI y la ventana de configuración.

use serde::Serialize;

use crate::device::{self, Access};
use crate::platform::Platform;
use crate::profile::Profile;
use crate::settings::Settings;
use crate::tool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Ok,
    Info,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub section: &'static str,
    pub level: Level,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub checks: Vec<Check>,
    pub connected: bool,
}

impl Report {
    pub fn problems(&self) -> usize {
        self.checks.iter().filter(|c| c.level == Level::Error).count()
    }

    /// Texto plano, apto para copiar en un reporte.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        let mut section = "";
        for c in &self.checks {
            if c.section != section {
                if !section.is_empty() {
                    out.push('\n');
                }
                out.push_str(&format!("== {}\n", c.section));
                section = c.section;
            }
            let mark = match c.level {
                Level::Ok => "✓ ",
                Level::Error => "✗ ",
                Level::Info => "  ",
            };
            out.push_str(&format!("{mark}{}\n", c.message));
        }
        out
    }
}

pub fn run() -> Report {
    let mut checks = Vec::new();
    let mut push = |section, level, message: String| checks.push(Check { section, level, message });

    const TOOL: &str = "Herramienta de programación";
    match tool::locate() {
        Some(t) => push(TOOL, Level::Ok, format!("{} ({})", t.display(), tool::version(&t).unwrap_or_default())),
        None => push(TOOL, Level::Error, format!("{0} no encontrado; instálalo con `cargo install {0}`", tool::BINARY)),
    }

    const DEV: &str = "Dispositivo 1189:8890";
    let found = device::find().unwrap_or_else(|e| {
        push(DEV, Level::Error, format!("no se pudo listar el USB: {e:#}"));
        Vec::new()
    });
    let connected = !found.is_empty();
    match found.first() {
        None => push(DEV, Level::Error, "no conectado".into()),
        Some(dev) => {
            if found.len() > 1 {
                push(DEV, Level::Info, format!("hay {} dispositivos iguales; se usa el primero", found.len()));
            }
            push(DEV, Level::Ok, format!("conectado en {}", dev.usb_path()));
            push(DEV, Level::Info, format!(
                "disposición: {} teclas ({}×{}) + {} perilla",
                device::ROWS * device::COLUMNS, device::ROWS, device::COLUMNS, device::KNOBS
            ));
            match device::describe(dev) {
                Ok(report) => {
                    for iface in &report.interfaces {
                        let eps: Vec<String> = iface.endpoints.iter()
                            .map(|e| format!("{:#04x} {} {}", e.address, e.direction, e.transfer))
                            .collect();
                        let mark = if iface.number == device::PROGRAMMING_INTERFACE { " ← programación" } else { "" };
                        push(DEV, Level::Info, format!("interfaz {}: {} [{}]{mark}", iface.number, iface.protocol, eps.join(", ")));
                    }
                    push(DEV, Level::Info, "el perfil guardado no se puede leer (solo escritura); se reaplica en cada conexión".into());
                    const PERM: &str = "Permisos";
                    match report.access {
                        Access::Ok => push(PERM, Level::Ok, "el usuario puede abrir el dispositivo".into()),
                        Access::Denied => push(PERM, Level::Error, format!(
                            "acceso denegado a {}{}",
                            dev.usb_path(),
                            if cfg!(target_os = "linux") { "; instala la regla udev" } else { "" }
                        )),
                        Access::Error(e) => push(PERM, Level::Error, format!("no se pudo abrir: {e}")),
                    }
                }
                Err(e) => push(DEV, Level::Error, format!("no se pudo inspeccionar: {e:#}")),
            }
        }
    }
    if cfg!(target_os = "linux") {
        let installed = UDEV_RULES.iter().find(|p| std::path::Path::new(p).exists());
        match installed {
            Some(p) => push("Permisos", Level::Ok, format!("regla udev {p}")),
            None => push("Permisos", Level::Info, "regla udev no instalada".into()),
        }
    }

    const PROF: &str = "Perfiles";
    let platform = Platform::current();
    match Settings::load() {
        Ok(settings) => {
            let auto = settings.auto_profile_for(platform);
            match Profile::load(auto).and_then(|(p, _)| p.validate(platform).map(|_| p)) {
                Ok(p) => push(PROF, Level::Ok, format!("perfil automático «{auto}» ({}) válido", p.name)),
                Err(e) => push(PROF, Level::Error, format!("perfil automático «{auto}»: {e:#}")),
            }
        }
        Err(e) => push(PROF, Level::Error, format!("config.yaml: {e:#}")),
    }

    Report { checks, connected }
}

/// Ubicaciones de la regla udev: instalación manual y paquete .deb.
pub const UDEV_RULES: &[&str] = &[
    "/etc/udev/rules.d/70-macropad-agent.rules",
    "/usr/lib/udev/rules.d/70-macropad-agent.rules",
];
