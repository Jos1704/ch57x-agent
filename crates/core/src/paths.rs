//! Ubicaciones locales. Los datos del usuario viven fuera de la aplicación
//! para sobrevivir a las actualizaciones.
//!
//! - Linux: `$XDG_CONFIG_HOME/macropad-agent` (por defecto `~/.config/macropad-agent`)
//! - macOS: `~/Library/Application Support/MacroPad Agent`
//!
//! `MACROPAD_AGENT_HOME` reemplaza la ubicación (útil para pruebas).

use std::env;
use std::path::PathBuf;

pub fn home_dir() -> PathBuf {
    env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

pub fn config_dir() -> PathBuf {
    if let Some(dir) = env::var_os("MACROPAD_AGENT_HOME") {
        return PathBuf::from(dir);
    }
    if cfg!(target_os = "macos") {
        return home_dir().join("Library/Application Support/MacroPad Agent");
    }
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home_dir().join(".config"));
    base.join("macropad-agent")
}

pub fn profiles_dir() -> PathBuf {
    config_dir().join("profiles")
}

pub fn settings_file() -> PathBuf {
    config_dir().join("config.yaml")
}

pub fn state_file() -> PathBuf {
    config_dir().join("state.json")
}

pub fn history_file() -> PathBuf {
    config_dir().join("history.jsonl")
}
