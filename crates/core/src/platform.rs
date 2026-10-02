use std::fmt;
use std::str::FromStr;

use anyhow::bail;
use serde::{Deserialize, Serialize};

/// Sistema operativo al que está dirigido un perfil.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Linux,
    Macos,
    /// Perfil válido para cualquier sistema; las acciones se resuelven
    /// con el sistema actual.
    Any,
}

impl Platform {
    pub fn current() -> Platform {
        if cfg!(target_os = "macos") {
            Platform::Macos
        } else {
            Platform::Linux
        }
    }

    /// Plataforma concreta con la que se resuelven las acciones.
    pub fn effective(self) -> Platform {
        match self {
            Platform::Any => Platform::current(),
            p => p,
        }
    }

    pub fn is_compatible_with(self, other: Platform) -> bool {
        self == Platform::Any || other == Platform::Any || self == other
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Platform::Linux => "linux",
            Platform::Macos => "macos",
            Platform::Any => "any",
        })
    }
}

impl FromStr for Platform {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "linux" => Platform::Linux,
            "macos" | "mac" | "darwin" => Platform::Macos,
            "any" => Platform::Any,
            other => bail!("plataforma desconocida: {other}"),
        })
    }
}
