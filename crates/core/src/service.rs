//! Control del agente de segundo plano: servicio de usuario `systemd` en
//! Linux y `LaunchAgent` en macOS.

use std::process::Command;

use anyhow::{bail, Context, Result};

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

fn run(cmd: &mut Command) -> Result<()> {
    let out = cmd.output().with_context(|| format!("ejecutar {cmd:?}"))?;
    if !out.status.success() {
        bail!("{cmd:?} falló: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(())
}
