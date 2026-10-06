use std::fmt;

use anyhow::Result;

use crate::platform::Platform;
use crate::profile::Profile;
use crate::{state, tool};

#[derive(Debug, Clone, Copy)]
pub enum Trigger {
    Manual,
    Connect,
    Startup,
}

impl fmt::Display for Trigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Trigger::Manual => "manual",
            Trigger::Connect => "connect",
            Trigger::Startup => "startup",
        })
    }
}

pub struct Outcome {
    pub profile_name: String,
    pub warnings: Vec<String>,
}

/// Carga, valida y programa un perfil, y registra el resultado.
pub fn apply_profile(id: &str, target: Platform, trigger: Trigger) -> Result<Outcome> {
    let mut name = id.to_string();
    let mut warnings = Vec::new();
    let result = (|| -> Result<()> {
        let (profile, _) = Profile::load(id)?;
        name = profile.name.clone();
        warnings = profile.validate(target)?;
        let config = profile.to_ch57x_yaml(target)?;
        let tool = tool::require()?;
        tool::validate(&tool, &config)?;
        tool::upload(&tool, &config)?;
        let _ = std::fs::write(applied_file(), &config);
        Ok(())
    })();
    let record = state::Record::now(id, &target.effective().to_string(), &trigger.to_string(), &result);
    if let Err(e) = state::save(&record) {
        eprintln!("aviso: no se pudo guardar el historial: {e:#}");
    }
    result.map(|()| Outcome { profile_name: name, warnings })
}

fn applied_file() -> std::path::PathBuf {
    crate::paths::config_dir().join("applied.yaml")
}

/// ¿Hay que reprogramar el teclado para que tenga este perfil? Pasa cuando
/// es el perfil activo y alguna tecla cambió lo que envía (por ejemplo, de
/// atajo a comando). Cambiar solo el comando no lo requiere.
pub fn needs_reapply(id: &str, target: Platform) -> bool {
    if state::active_profile() != id {
        return false;
    }
    let Ok((profile, _)) = Profile::load(id) else { return false };
    let Ok(config) = profile.to_ch57x_yaml(target) else { return false };
    std::fs::read_to_string(applied_file()).map(|applied| applied != config).unwrap_or(true)
}
