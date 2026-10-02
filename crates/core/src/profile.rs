//! Perfiles YAML: lectura, validación y conversión al formato de
//! `ch57x-keyboard-tool`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::actions;
use crate::device;
use crate::paths;
use crate::platform::Platform;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub name: String,
    pub platform: Platform,
    pub bindings: Bindings,
}

/// Seis teclas y una perilla, en la misma posición física en todos los perfiles.
/// Con la perilla a la derecha, las teclas 1–3 forman la fila superior.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bindings {
    pub key_1: Option<String>,
    pub key_2: Option<String>,
    pub key_3: Option<String>,
    pub key_4: Option<String>,
    pub key_5: Option<String>,
    pub key_6: Option<String>,
    pub knob_left: Option<String>,
    pub knob_right: Option<String>,
    pub knob_press: Option<String>,
}

impl Bindings {
    pub fn keys(&self) -> [&Option<String>; 6] {
        [&self.key_1, &self.key_2, &self.key_3, &self.key_4, &self.key_5, &self.key_6]
    }

    /// Pares (control, valor) en orden físico.
    pub fn entries(&self) -> Vec<(&'static str, &Option<String>)> {
        vec![
            ("key_1", &self.key_1),
            ("key_2", &self.key_2),
            ("key_3", &self.key_3),
            ("key_4", &self.key_4),
            ("key_5", &self.key_5),
            ("key_6", &self.key_6),
            ("knob_left", &self.knob_left),
            ("knob_right", &self.knob_right),
            ("knob_press", &self.knob_press),
        ]
    }
}

/// Perfiles incluidos en el binario.
const BUILTIN: &[(&str, &str)] = &[
    ("desarrollo-linux", include_str!("../../../profiles/desarrollo-linux.yaml")),
    ("desarrollo-macos", include_str!("../../../profiles/desarrollo-macos.yaml")),
    ("prueba", include_str!("../../../profiles/prueba.yaml")),
];

#[derive(Debug, Clone)]
pub enum Source {
    Builtin,
    User(PathBuf),
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: String,
    pub source: Source,
}

impl Profile {
    pub fn from_yaml(text: &str) -> Result<Profile> {
        Ok(serde_yaml::from_str(text)?)
    }

    pub fn from_file(path: &Path) -> Result<Profile> {
        let text = fs::read_to_string(path).with_context(|| format!("leer {}", path.display()))?;
        Profile::from_yaml(&text).with_context(|| format!("interpretar {}", path.display()))
    }

    /// Busca un perfil por identificador o ruta. Los perfiles del usuario
    /// tienen prioridad sobre los incluidos.
    pub fn load(id_or_path: &str) -> Result<(Profile, Source)> {
        let as_path = Path::new(id_or_path);
        if id_or_path.ends_with(".yaml") || id_or_path.ends_with(".yml") || id_or_path.contains('/') {
            return Ok((Profile::from_file(as_path)?, Source::User(as_path.to_path_buf())));
        }
        for ext in ["yaml", "yml"] {
            let path = paths::profiles_dir().join(format!("{id_or_path}.{ext}"));
            if path.exists() {
                return Ok((Profile::from_file(&path)?, Source::User(path)));
            }
        }
        if let Some((_, text)) = BUILTIN.iter().find(|(id, _)| *id == id_or_path) {
            let profile = Profile::from_yaml(text)
                .with_context(|| format!("perfil incluido {id_or_path}"))?;
            return Ok((profile, Source::Builtin));
        }
        bail!("perfil no encontrado: {id_or_path}")
    }

    /// Lista los perfiles disponibles (usuario + incluidos, sin duplicados).
    pub fn list() -> Vec<Entry> {
        let mut map: BTreeMap<String, Source> = BUILTIN
            .iter()
            .map(|(id, _)| (id.to_string(), Source::Builtin))
            .collect();
        if let Ok(dir) = fs::read_dir(paths::profiles_dir()) {
            for entry in dir.flatten() {
                let path = entry.path();
                let is_yaml = matches!(path.extension().and_then(|e| e.to_str()), Some("yaml" | "yml"));
                if let (true, Some(stem)) = (is_yaml, path.file_stem().and_then(|s| s.to_str())) {
                    map.insert(stem.to_string(), Source::User(path.clone()));
                }
            }
        }
        map.into_iter().map(|(id, source)| Entry { id, source }).collect()
    }

    pub fn is_builtin(id: &str) -> bool {
        BUILTIN.iter().any(|(b, _)| *b == id)
    }

    /// Ruta del perfil del usuario con este identificador (exista o no).
    pub fn user_path(id: &str) -> Result<PathBuf> {
        let valid = !id.is_empty()
            && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !valid {
            bail!("identificador de perfil no válido: «{id}» (usa letras, números, - y _)");
        }
        Ok(paths::profiles_dir().join(format!("{id}.yaml")))
    }

    /// Guarda el perfil en la carpeta del usuario. Si el identificador es
    /// de un perfil incluido, lo reemplaza para este usuario.
    pub fn save(&self, id: &str) -> Result<PathBuf> {
        self.validate(self.platform.effective())?;
        let path = Profile::user_path(id)?;
        fs::create_dir_all(paths::profiles_dir())?;
        fs::write(&path, serde_yaml::to_string(self)?)
            .with_context(|| format!("escribir {}", path.display()))?;
        Ok(path)
    }

    /// Borra el perfil del usuario. Si reemplazaba a uno incluido, vuelve
    /// a quedar el incluido. Devuelve `false` si no había archivo.
    pub fn delete_user(id: &str) -> Result<bool> {
        let path = Profile::user_path(id)?;
        if !path.exists() {
            return Ok(false);
        }
        fs::remove_file(&path).with_context(|| format!("borrar {}", path.display()))?;
        Ok(true)
    }

    /// Validación propia (antes de consultar a `ch57x-keyboard-tool`).
    /// Devuelve advertencias no fatales.
    pub fn validate(&self, target: Platform) -> Result<Vec<String>> {
        let mut warnings = Vec::new();
        if self.name.trim().is_empty() {
            bail!("el perfil no tiene nombre");
        }
        if !self.platform.is_compatible_with(target) {
            bail!(
                "el perfil «{}» es para {} y el sistema destino es {}",
                self.name, self.platform, target
            );
        }
        let platform = self.resolution_platform(target);
        for (control, value) in self.bindings.entries() {
            let Some(value) = value else {
                warnings.push(format!("{control} no tiene acción asignada"));
                continue;
            };
            if value.trim().is_empty() {
                bail!("{control} está vacío");
            }
            let raw = actions::resolve(value, platform);
            if let Some(m) = actions::foreign_modifier(&raw, platform) {
                warnings.push(format!("{control} = «{raw}» usa el modificador «{m}», propio de otro sistema"));
            }
        }
        Ok(warnings)
    }

    fn resolution_platform(&self, target: Platform) -> Platform {
        match self.platform {
            Platform::Any => target.effective(),
            p => p,
        }
    }

    /// Valor ya traducido para cada control.
    pub fn resolved(&self, target: Platform) -> Vec<(&'static str, Option<String>)> {
        let platform = self.resolution_platform(target);
        self.bindings
            .entries()
            .into_iter()
            .map(|(control, value)| (control, value.as_deref().map(|v| actions::resolve(v, platform))))
            .collect()
    }

    /// Genera la configuración de `ch57x-keyboard-tool` para este perfil.
    pub fn to_ch57x_yaml(&self, target: Platform) -> Result<String> {
        let platform = self.resolution_platform(target);
        let r = |v: &Option<String>| v.as_deref().map(|v| actions::resolve(v, platform));
        let keys: Vec<Option<String>> = self.bindings.keys().iter().map(|k| r(k)).collect();
        let (rows, cols) = (device::ROWS as usize, device::COLUMNS as usize);
        let buttons: Vec<Vec<Option<String>>> = keys.chunks(cols).map(|c| c.to_vec()).collect();
        debug_assert_eq!(buttons.len(), rows);

        let config = Ch57xConfig {
            model: device::MODEL,
            orientation: "normal",
            rows: device::ROWS,
            columns: device::COLUMNS,
            knobs: device::KNOBS,
            layers: vec![Ch57xLayer {
                buttons,
                knobs: vec![Ch57xKnob {
                    ccw: r(&self.bindings.knob_left),
                    press: r(&self.bindings.knob_press),
                    cw: r(&self.bindings.knob_right),
                }],
            }],
        };
        Ok(serde_yaml::to_string(&config)?)
    }
}

#[derive(Serialize)]
struct Ch57xConfig {
    model: &'static str,
    orientation: &'static str,
    rows: u8,
    columns: u8,
    knobs: u8,
    layers: Vec<Ch57xLayer>,
}

#[derive(Serialize)]
struct Ch57xLayer {
    buttons: Vec<Vec<Option<String>>>,
    knobs: Vec<Ch57xKnob>,
}

#[derive(Serialize)]
struct Ch57xKnob {
    ccw: Option<String>,
    press: Option<String>,
    cw: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_profiles_parse_and_validate() {
        for (id, text) in BUILTIN {
            let p = Profile::from_yaml(text).unwrap_or_else(|e| panic!("{id}: {e}"));
            let target = p.platform.effective();
            p.validate(target).unwrap_or_else(|e| panic!("{id}: {e}"));
        }
    }

    #[test]
    fn linux_profile_renders_ctrl() {
        let (p, _) = Profile::load("desarrollo-linux").unwrap();
        let yaml = p.to_ch57x_yaml(Platform::Linux).unwrap();
        assert!(yaml.contains("ctrl-s"));
        assert!(yaml.contains("ctrl-shift-p"));
        assert!(yaml.contains("volumedown"));
        assert!(yaml.contains("model: ch57x-2"));
    }

    #[test]
    fn macos_profile_renders_cmd() {
        let (p, _) = Profile::load("desarrollo-macos").unwrap();
        let yaml = p.to_ch57x_yaml(Platform::Macos).unwrap();
        assert!(yaml.contains("cmd-s"));
        assert!(!yaml.contains("ctrl-"));
    }

    #[test]
    fn rejects_wrong_platform() {
        let (p, _) = Profile::load("desarrollo-macos").unwrap();
        assert!(p.validate(Platform::Linux).is_err());
    }

    #[test]
    fn user_profile_overrides_and_restores_builtin() {
        let dir = std::env::temp_dir().join(format!("macropad-test-{}", std::process::id()));
        // Único test que usa MACROPAD_AGENT_HOME.
        unsafe { std::env::set_var("MACROPAD_AGENT_HOME", &dir) };

        let (mut p, _) = Profile::load("desarrollo-linux").unwrap();
        p.bindings.key_6 = Some("ctrl-alt-t".into());
        p.save("desarrollo-linux").unwrap();
        let (loaded, source) = Profile::load("desarrollo-linux").unwrap();
        assert!(matches!(source, Source::User(_)));
        assert_eq!(loaded.bindings.key_6.as_deref(), Some("ctrl-alt-t"));

        assert!(Profile::delete_user("desarrollo-linux").unwrap());
        let (_, source) = Profile::load("desarrollo-linux").unwrap();
        assert!(matches!(source, Source::Builtin));
        assert!(Profile::user_path("../x").is_err());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_unknown_controls() {
        let text = "name: x\nplatform: linux\nbindings:\n  key_7: a\n";
        assert!(Profile::from_yaml(text).is_err());
    }
}
