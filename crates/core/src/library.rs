//! Biblioteca de comandos y flujos: los preconfigurados (incluidos en el
//! binario) y los que guarda el usuario (`biblioteca.yaml`).

use std::collections::BTreeMap;
use std::fs;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::automation::Binding;
use crate::paths;
use crate::platform::Platform;

const BUILTIN: &str = include_str!("../templates/biblioteca.yaml");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Template {
    pub id: String,
    /// `flujos`, `desplegar`, `desarrollo`, `sistema` o `guardados`.
    pub category: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Vacío: todos los sistemas.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub platforms: Vec<Platform>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub needs: Vec<Need>,
    pub binding: Binding,
    /// Lo pone la biblioteca al listar: `true` si lo guardó el usuario.
    #[serde(default, skip_serializing)]
    pub user: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Need {
    /// `cwd` (carpeta de trabajo) o el nombre de una variable.
    pub key: String,
    pub label: String,
    /// `folder` o `text` (por defecto).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default)]
    pub default: String,
}

impl Template {
    /// La automatización con los datos que pidió la plantilla.
    pub fn instantiate(&self, values: &BTreeMap<String, String>) -> Result<Binding> {
        let mut binding = self.binding.clone();
        for need in &self.needs {
            let value = values.get(&need.key).cloned().unwrap_or_else(|| need.default.clone());
            if value.trim().is_empty() {
                bail!("falta «{}»", need.label);
            }
            let (cwd, vars) = match &mut binding {
                Binding::Command(c) => (&mut c.cwd, &mut c.vars),
                Binding::Flow(f) => (&mut f.cwd, &mut f.vars),
                Binding::Keys(_) => bail!("la plantilla no es un comando ni un flujo"),
            };
            if need.key == "cwd" {
                *cwd = Some(value);
            } else {
                vars.insert(need.key.clone(), value);
            }
        }
        binding.validate()?;
        Ok(binding)
    }
}

fn user_file() -> std::path::PathBuf {
    paths::config_dir().join("biblioteca.yaml")
}

fn builtin() -> Vec<Template> {
    serde_yaml::from_str(BUILTIN).expect("biblioteca incluida válida")
}

fn load_user() -> Result<Vec<Template>> {
    let path = user_file();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&path).with_context(|| format!("leer {}", path.display()))?;
    serde_yaml::from_str(&text).with_context(|| format!("interpretar {}", path.display()))
}

/// Plantillas para este sistema: primero las del usuario.
pub fn list(platform: Platform) -> Result<Vec<Template>> {
    let platform = platform.effective();
    let mut user = load_user()?;
    for t in &mut user {
        t.user = true;
    }
    Ok(user
        .into_iter()
        .chain(builtin())
        .filter(|t| t.platforms.is_empty() || t.platforms.iter().any(|p| p.is_compatible_with(platform)))
        .collect())
}

pub fn find(id: &str, platform: Platform) -> Result<Template> {
    list(platform)?.into_iter().find(|t| t.id == id).with_context(|| format!("no está en la biblioteca: {id}"))
}

/// Guarda una automatización en la biblioteca del usuario (reemplaza la de igual título).
pub fn save_user(title: &str, description: &str, binding: Binding) -> Result<Template> {
    if !binding.is_automation() {
        bail!("solo se guardan comandos y flujos");
    }
    binding.validate()?;
    let title = title.trim();
    if title.is_empty() {
        bail!("ponle un nombre");
    }
    let id: String = format!("mio-{}", slug(title));
    let template = Template {
        id: id.clone(),
        category: "guardados".into(),
        title: title.into(),
        description: description.trim().into(),
        platforms: vec![Platform::current()],
        needs: Vec::new(),
        binding,
        user: true,
    };
    let mut user = load_user()?;
    user.retain(|t| t.id != id);
    user.insert(0, template.clone());
    fs::create_dir_all(paths::config_dir())?;
    fs::write(user_file(), serde_yaml::to_string(&user)?)?;
    Ok(template)
}

pub fn delete_user(id: &str) -> Result<bool> {
    let mut user = load_user()?;
    let before = user.len();
    user.retain(|t| t.id != id);
    if user.len() == before {
        return Ok(false);
    }
    fs::write(user_file(), serde_yaml::to_string(&user)?)?;
    Ok(true)
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        let c = match c {
            'á' | 'à' | 'ä' => 'a',
            'é' | 'è' | 'ë' => 'e',
            'í' | 'ì' | 'ï' => 'i',
            'ó' | 'ò' | 'ö' => 'o',
            'ú' | 'ù' | 'ü' => 'u',
            'ñ' => 'n',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_templates_are_valid_and_unique() {
        let all = builtin();
        let mut ids: Vec<&str> = all.iter().map(|t| t.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), all.len(), "ids repetidos");
        for t in &all {
            assert!(t.binding.is_automation(), "{}", t.id);
            let values: BTreeMap<String, String> = t.needs.iter().map(|n| (n.key.clone(), n.default.clone())).collect();
            t.instantiate(&values).unwrap_or_else(|e| panic!("{}: {e}", t.id));
        }
    }

    #[test]
    fn each_platform_gets_one_of_each_title() {
        for p in [Platform::Linux, Platform::Macos] {
            let list: Vec<Template> = builtin()
                .into_iter()
                .filter(|t| t.platforms.is_empty() || t.platforms.contains(&p))
                .collect();
            let mut titles: Vec<&str> = list.iter().map(|t| t.title.as_str()).collect();
            let total = titles.len();
            titles.sort();
            titles.dedup();
            assert_eq!(titles.len(), total, "{p}: títulos repetidos");
        }
    }

    #[test]
    fn instantiate_fills_cwd_and_vars() {
        let t = builtin().into_iter().find(|t| t.id == "empezar-a-trabajar").unwrap();
        let values = BTreeMap::from([("cwd".to_string(), "~/x".to_string()), ("PUERTO".to_string(), "8080".to_string())]);
        let Binding::Flow(f) = t.instantiate(&values).unwrap() else { panic!() };
        assert_eq!(f.cwd.as_deref(), Some("~/x"));
        assert_eq!(f.vars["PUERTO"], "8080");
    }

    #[test]
    fn slugs() {
        assert_eq!(slug("Mi Flujo: Cañón!"), "mi-flujo-canon");
    }
}
