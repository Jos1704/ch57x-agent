//! Envoltorio de `ch57x-keyboard-tool`, que se encarga del protocolo USB.

use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};

use crate::paths;

pub const BINARY: &str = "ch57x-keyboard-tool";

/// Busca el binario en `MACROPAD_CH57X_TOOL`, junto al ejecutable actual
/// (paquete .deb o bundle .app), en el `PATH` y en las ubicaciones
/// habituales de `cargo install` y Homebrew.
pub fn locate() -> Option<PathBuf> {
    if let Some(path) = env::var_os("MACROPAD_CH57X_TOOL").map(PathBuf::from) {
        return path.is_file().then_some(path);
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(dir) = env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
        candidates.push(dir.join(BINARY));
    }
    candidates.push(PathBuf::from("/usr/lib/macropad-agent").join(BINARY));
    if let Some(path) = env::var_os("PATH") {
        candidates.extend(env::split_paths(&path).map(|d| d.join(BINARY)));
    }
    let cargo_home = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| paths::home_dir().join(".cargo"));
    candidates.push(cargo_home.join("bin").join(BINARY));
    candidates.push(PathBuf::from("/opt/homebrew/bin").join(BINARY));
    candidates.push(PathBuf::from("/usr/local/bin").join(BINARY));
    candidates.into_iter().find(|p| p.is_file())
}

pub fn require() -> Result<PathBuf> {
    locate().with_context(|| {
        format!("no se encontró {BINARY}; instálalo con `cargo install {BINARY}` o define MACROPAD_CH57X_TOOL")
    })
}

pub fn version(tool: &Path) -> Result<String> {
    let out = Command::new(tool).arg("--version").output()?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Valida una configuración de `ch57x-keyboard-tool` sin tocar el dispositivo.
pub fn validate(tool: &Path, config: &str) -> Result<()> {
    run(tool, "validate", config).context("la configuración no es válida")
}

/// Programa el teclado con la configuración dada.
pub fn upload(tool: &Path, config: &str) -> Result<()> {
    run(tool, "upload", config).context("no se pudo programar el teclado")
}

fn run(tool: &Path, command: &str, stdin: &str) -> Result<()> {
    let mut child = Command::new(tool)
        .arg(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("ejecutar {}", tool.display()))?;
    child.stdin.take().expect("stdin").write_all(stdin.as_bytes())?;
    let out = child.wait_with_output()?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let detail = stderr
            .lines()
            .map(|l| l.trim().trim_start_matches("Error:").trim())
            .filter(|l| !l.is_empty() && *l != "Caused by:")
            .collect::<Vec<_>>()
            .join(": ");
        let hint = if stderr.contains("Access denied") || stderr.contains("insufficient permissions") {
            " (sin permisos sobre el USB: instala la regla udev con installers/linux/install-udev.sh)"
        } else {
            ""
        };
        bail!("{BINARY} {command} falló: {detail}{hint}");
    }
    Ok(())
}

/// Teclas que acepta la herramienta, agrupadas como en `show-keys`.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct KeyList {
    pub modifiers: Vec<String>,
    pub keys: Vec<String>,
    pub media: Vec<String>,
}

pub fn keys(tool: &Path) -> Result<KeyList> {
    let out = Command::new(tool).arg("show-keys").output()?;
    Ok(parse_show_keys(&String::from_utf8_lossy(&out.stdout)))
}

fn parse_show_keys(text: &str) -> KeyList {
    let mut list = KeyList::default();
    let mut current: Option<&mut Vec<String>> = None;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("Modifiers") {
            current = Some(&mut list.modifiers);
        } else if line.starts_with("Keys") {
            current = Some(&mut list.keys);
        } else if line.starts_with("Media") {
            current = Some(&mut list.media);
        } else if line.starts_with("Mouse") || line.starts_with("Custom") {
            current = None;
        } else if let (Some(item), Some(target)) = (line.strip_prefix("- "), current.as_deref_mut()) {
            // "alt / opt" → "alt"
            target.push(item.split(" / ").next().unwrap_or(item).trim().to_string());
        }
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_show_keys() {
        let text = "Modifiers: \n - ctrl\n - alt / opt\n\nKeys:\n - a\n - f5\n\nCustom key syntax (use decimal code): <110>\n\nMedia keys:\n - mute\n - previous / prev\n\nMouse actions:\n - wheel(-100)\n";
        let list = parse_show_keys(text);
        assert_eq!(list.modifiers, ["ctrl", "alt"]);
        assert_eq!(list.keys, ["a", "f5"]);
        assert_eq!(list.media, ["mute", "previous"]);
    }
}
