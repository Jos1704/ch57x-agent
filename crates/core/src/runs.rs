//! Historial de comandos y flujos ejecutados (`commands.jsonl`) y el registro
//! de los flujos que corren al conectar el pad una vez al día.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::process::Command;
use std::time::SystemTime;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::automation::Outcome;
use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub timestamp: String,
    pub profile: String,
    pub control: String,
    pub label: String,
    /// `tecla`, `conexión` o `prueba`.
    pub trigger: String,
    /// `ok`, `error` o `ignorada`.
    pub result: String,
    pub code: Option<i32>,
    pub duration_ms: u64,
    pub output: String,
}

impl Run {
    fn now(profile: &str, control: &str, label: &str, trigger: &str) -> Run {
        Run {
            timestamp: humantime::format_rfc3339_seconds(SystemTime::now()).to_string(),
            profile: profile.into(),
            control: control.into(),
            label: label.into(),
            trigger: trigger.into(),
            result: String::new(),
            code: None,
            duration_ms: 0,
            output: String::new(),
        }
    }

    pub fn finished(profile: &str, control: &str, label: &str, trigger: &str, outcome: &Outcome) -> Run {
        Run {
            result: if outcome.ok { "ok" } else { "error" }.into(),
            code: outcome.code,
            duration_ms: outcome.duration_ms,
            output: outcome.output.clone(),
            ..Run::now(profile, control, label, trigger)
        }
    }

    pub fn ignored(profile: &str, control: &str, label: &str, trigger: &str, reason: &str) -> Run {
        Run { result: "ignorada".into(), output: reason.into(), ..Run::now(profile, control, label, trigger) }
    }
}

fn history_file() -> std::path::PathBuf {
    paths::config_dir().join("commands.jsonl")
}

fn connect_file() -> std::path::PathBuf {
    paths::config_dir().join("on-connect.json")
}

pub fn save(run: &Run) -> Result<()> {
    fs::create_dir_all(paths::config_dir())?;
    let mut file = OpenOptions::new().create(true).append(true).open(history_file())?;
    writeln!(file, "{}", serde_json::to_string(run)?)?;
    Ok(())
}

/// Últimas `n` ejecuciones, de la más reciente a la más antigua.
pub fn history(n: usize) -> Vec<Run> {
    let Ok(text) = fs::read_to_string(history_file()) else { return Vec::new() };
    text.lines().rev().filter_map(|l| serde_json::from_str(l).ok()).take(n).collect()
}

/// Fecha local `AAAA-MM-DD`.
fn today() -> String {
    Command::new("date")
        .arg("+%F")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| humantime::format_rfc3339_seconds(SystemTime::now()).to_string()[..10].to_string())
}

/// `true` (y lo registra) si el flujo de ese control no ha corrido hoy al conectar.
pub fn claim_on_connect(profile: &str, control: &str) -> bool {
    let mut seen: BTreeMap<String, String> = fs::read_to_string(connect_file())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    let key = format!("{profile}:{control}");
    let today = today();
    if seen.get(&key) == Some(&today) {
        return false;
    }
    seen.insert(key, today);
    let _ = fs::create_dir_all(paths::config_dir());
    let _ = serde_json::to_string(&seen).map(|t| fs::write(connect_file(), t));
    true
}
