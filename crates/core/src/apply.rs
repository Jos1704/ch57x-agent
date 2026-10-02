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
        tool::upload(&tool, &config)
    })();
    let record = state::Record::now(id, &target.effective().to_string(), &trigger.to_string(), &result);
    if let Err(e) = state::save(&record) {
        eprintln!("aviso: no se pudo guardar el historial: {e:#}");
    }
    result.map(|()| Outcome { profile_name: name, warnings })
}
