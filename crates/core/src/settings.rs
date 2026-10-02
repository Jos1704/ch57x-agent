use std::fs;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::paths;
use crate::platform::Platform;

/// Configuración del agente (`config.yaml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Perfil que se aplica automáticamente en cada sistema.
    pub auto_profile: AutoProfile,
    /// Espera tras la conexión para que el sistema cree las interfaces USB.
    pub settle_delay_ms: u64,
    /// Mostrar una notificación de escritorio al aplicar un perfil.
    pub notifications: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AutoProfile {
    pub linux: String,
    pub macos: String,
}

impl Default for AutoProfile {
    fn default() -> Self {
        AutoProfile {
            linux: "desarrollo-linux".into(),
            macos: "desarrollo-macos".into(),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            auto_profile: AutoProfile::default(),
            settle_delay_ms: 1500,
            notifications: true,
        }
    }
}

impl Settings {
    pub fn load() -> Result<Settings> {
        let path = paths::settings_file();
        if !path.exists() {
            return Ok(Settings::default());
        }
        let text = fs::read_to_string(&path).with_context(|| format!("leer {}", path.display()))?;
        serde_yaml::from_str(&text).with_context(|| format!("interpretar {}", path.display()))
    }

    pub fn save(&self) -> Result<()> {
        fs::create_dir_all(paths::config_dir())?;
        fs::write(paths::settings_file(), serde_yaml::to_string(self)?)?;
        Ok(())
    }

    pub fn set_auto_profile(&mut self, platform: Platform, id: &str) {
        match platform.effective() {
            Platform::Macos => self.auto_profile.macos = id.to_string(),
            _ => self.auto_profile.linux = id.to_string(),
        }
    }

    pub fn auto_profile_for(&self, platform: Platform) -> &str {
        match platform.effective() {
            Platform::Macos => &self.auto_profile.macos,
            _ => &self.auto_profile.linux,
        }
    }
}
