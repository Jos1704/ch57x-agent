//! Control del agente de segundo plano: servicio de usuario `systemd` en
//! Linux y `LaunchAgent` en macOS.

use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::paths;

pub const SYSTEMD_UNIT: &str = "macropad-agent.service";
pub const LAUNCHD_LABEL: &str = "com.macropad-agent";

/// `Some(true)` si el agente está corriendo, `None` si no se pudo saber.
pub fn is_active() -> Option<bool> {
    if cfg!(target_os = "macos") {
        let out = Command::new("launchctl").args(["list", LAUNCHD_LABEL]).output().ok()?;
        Some(out.status.success())
    } else {
        let out = Command::new("systemctl")
            .args(["--user", "is-active", SYSTEMD_UNIT])
            .output()
            .ok()?;
        Some(String::from_utf8_lossy(&out.stdout).trim() == "active")
    }
}

/// Activa el inicio automático y (re)inicia el agente.
pub fn restart() -> Result<()> {
    if cfg!(target_os = "macos") {
        let uid = String::from_utf8(Command::new("id").arg("-u").output()?.stdout)?;
        let target = format!("gui/{}/{LAUNCHD_LABEL}", uid.trim());
        run(Command::new("launchctl").args(["kickstart", "-k", &target]))
    } else {
        run(Command::new("systemctl").args(["--user", "enable", SYSTEMD_UNIT]))?;
        run(Command::new("systemctl").args(["--user", "restart", SYSTEMD_UNIT]))
    }
}

/// Plantilla del LaunchAgent, la misma que usa `installers/macos/install.sh`.
const LAUNCHD_PLIST: &str = include_str!("../../../installers/macos/com.macropad-agent.plist");

/// macOS: instala o actualiza el LaunchAgent para que ejecute `agent`, y lo
/// recarga si cambió. Devuelve `true` si hubo que escribirlo.
pub fn install_launch_agent(agent: &Path) -> Result<bool> {
    let plist = paths::home_dir().join("Library/LaunchAgents").join(format!("{LAUNCHD_LABEL}.plist"));
    let log = paths::config_dir().join("agent.log");
    let content = LAUNCHD_PLIST
        .replace("__BIN__", &xml_escape(&agent.to_string_lossy()))
        .replace("__LOG__", &xml_escape(&log.to_string_lossy()));
    if fs::read_to_string(&plist).is_ok_and(|old| old == content) {
        return Ok(false);
    }
    fs::create_dir_all(paths::config_dir())?;
    fs::create_dir_all(plist.parent().unwrap())?;
    fs::write(&plist, content).with_context(|| format!("escribir {}", plist.display()))?;
    let uid = String::from_utf8(Command::new("id").arg("-u").output()?.stdout)?;
    let domain = format!("gui/{}", uid.trim());
    let plist = plist.to_string_lossy();
    // Puede fallar si el agente no estaba cargado; no importa.
    let _ = Command::new("launchctl").args(["bootout", &domain, &plist]).output();
    run(Command::new("launchctl").args(["bootstrap", &domain, &plist]))?;
    Ok(true)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn run(cmd: &mut Command) -> Result<()> {
    let out = cmd.output().with_context(|| format!("ejecutar {cmd:?}"))?;
    if !out.status.success() {
        bail!("{cmd:?} falló: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(())
}
