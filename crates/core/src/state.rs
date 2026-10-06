//! Registro local del último perfil aplicado y del historial de intentos.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::time::SystemTime;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub timestamp: String,
    pub profile: String,
    pub platform: String,
    /// `manual`, `connect` o `startup`.
    pub trigger: String,
    pub ok: bool,
    pub message: String,
}

impl Record {
    pub fn now(profile: &str, platform: &str, trigger: &str, result: &Result<()>) -> Record {
        Record {
            timestamp: humantime::format_rfc3339_seconds(SystemTime::now()).to_string(),
            profile: profile.to_string(),
            platform: platform.to_string(),
            trigger: trigger.to_string(),
            ok: result.is_ok(),
            message: match result {
                Ok(()) => "perfil aplicado".into(),
                Err(e) => format!("{e:#}"),
            },
        }
    }
}

/// Guarda el intento en el historial y, si tuvo éxito, como último perfil aplicado.
pub fn save(record: &Record) -> Result<()> {
    fs::create_dir_all(paths::config_dir())?;
    let line = serde_json::to_string(record)?;
    let mut history = OpenOptions::new().create(true).append(true).open(paths::history_file())?;
    writeln!(history, "{line}")?;
    if record.ok {
        fs::write(paths::state_file(), serde_json::to_string_pretty(record)?)?;
    }
    Ok(())
}

/// Último perfil aplicado con éxito.
pub fn last_applied() -> Option<Record> {
    let text = fs::read_to_string(paths::state_file()).ok()?;
    serde_json::from_str(&text).ok()
}

/// Últimos `n` intentos, del más reciente al más antiguo.
pub fn history(n: usize) -> Vec<Record> {
    let Ok(text) = fs::read_to_string(paths::history_file()) else { return Vec::new() };
    text.lines()
        .rev()
        .filter_map(|l| serde_json::from_str(l).ok())
        .take(n)
        .collect()
}

/// Perfil que tiene el teclado ahora: el último aplicado con éxito o, si
/// nunca se aplicó ninguno, el automático de este sistema.
pub fn active_profile() -> String {
    last_applied().map(|r| r.profile).unwrap_or_else(|| {
        crate::settings::Settings::load()
            .map(|s| s.auto_profile_for(crate::Platform::current()).to_string())
            .unwrap_or_else(|_| crate::settings::AutoProfile::default().linux)
    })
}
