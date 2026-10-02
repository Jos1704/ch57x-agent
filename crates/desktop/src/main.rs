//! Ventana de configuración de MacroPad Agent. El agente de segundo plano
//! (`macropad-agent run`) sigue funcionando aunque esta ventana esté cerrada.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use macropad_core::actions::{self, Category};
use macropad_core::device::{self, Access};
use macropad_core::profile::Source;
use macropad_core::settings::Settings;
use macropad_core::{diagnose, paths, service, state, tool, Platform, Profile, Trigger};
use serde::Serialize;

/// Los errores llegan a la interfaz como texto.
type CmdResult<T> = Result<T, String>;

fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

#[derive(Serialize)]
struct Status {
    connected: bool,
    usb_path: Option<String>,
    /// `ok`, `denied` o el mensaje de error.
    access: Option<String>,
    platform: Platform,
    auto_profile: String,
    last_applied: Option<state::Record>,
    last_failure: Option<state::Record>,
    agent_active: Option<bool>,
    config_dir: String,
}

#[tauri::command(async)]
fn get_status() -> CmdResult<Status> {
    let settings = Settings::load().map_err(err)?;
    let platform = Platform::current();
    let found = device::find().map_err(err)?;
    let dev = found.first();
    let access = dev.map(|d| match device::describe(d).map(|r| r.access) {
        Ok(Access::Ok) => "ok".to_string(),
        Ok(Access::Denied) => "denied".to_string(),
        Ok(Access::Error(e)) => e,
        Err(e) => format!("{e:#}"),
    });
    Ok(Status {
        connected: dev.is_some(),
        usb_path: dev.map(|d| d.usb_path()),
        access,
        platform,
        auto_profile: settings.auto_profile_for(platform).to_string(),
        last_applied: state::last_applied(),
        last_failure: state::history(1).into_iter().find(|r| !r.ok),
        agent_active: service::is_active(),
        config_dir: paths::config_dir().display().to_string(),
    })
}

#[derive(Serialize)]
struct ProfileSummary {
    id: String,
    name: String,
    platform: Option<Platform>,
    builtin: bool,
    /// Perfil incluido reemplazado por una copia del usuario.
    modified: bool,
    error: Option<String>,
}

#[tauri::command(async)]
fn list_profiles() -> Vec<ProfileSummary> {
    Profile::list()
        .into_iter()
        .map(|entry| {
            let builtin = Profile::is_builtin(&entry.id);
            let modified = builtin && matches!(entry.source, Source::User(_));
            match Profile::load(&entry.id) {
                Ok((p, _)) => ProfileSummary {
                    id: entry.id,
                    name: p.name,
                    platform: Some(p.platform),
                    builtin,
                    modified,
                    error: None,
                },
                Err(e) => ProfileSummary {
                    name: entry.id.clone(),
                    id: entry.id,
                    platform: None,
                    builtin,
                    modified,
                    error: Some(format!("{e:#}")),
                },
            }
        })
        .collect()
}

#[tauri::command(async)]
fn get_profile(id: String) -> CmdResult<Profile> {
    Profile::load(&id).map(|(p, _)| p).map_err(err)
}

#[derive(Serialize)]
struct Preview {
    resolved: Vec<(String, Option<String>)>,
    warnings: Vec<String>,
    error: Option<String>,
}

/// Valida un perfil sin guardarlo y muestra cómo queda cada control.
#[tauri::command(async)]
fn preview_profile(profile: Profile) -> Preview {
    let target = profile.platform.effective();
    let resolved = profile
        .resolved(target)
        .into_iter()
        .map(|(c, v)| (c.to_string(), v))
        .collect();
    let check = profile.validate(target).and_then(|warnings| {
        let config = profile.to_ch57x_yaml(target)?;
        if let Some(t) = tool::locate() {
            tool::validate(&t, &config)?;
        }
        Ok(warnings)
    });
    match check {
        Ok(warnings) => Preview { resolved, warnings, error: None },
        Err(e) => Preview { resolved, warnings: Vec::new(), error: Some(format!("{e:#}")) },
    }
}

#[tauri::command(async)]
fn save_profile(id: String, profile: Profile) -> CmdResult<String> {
    if let Some(t) = tool::locate() {
        let config = profile.to_ch57x_yaml(profile.platform.effective()).map_err(err)?;
        tool::validate(&t, &config).map_err(err)?;
    }
    profile.save(&id).map(|p| p.display().to_string()).map_err(err)
}

#[tauri::command(async)]
fn delete_profile(id: String) -> CmdResult<bool> {
    Profile::delete_user(&id).map_err(err)
}

#[tauri::command(async)]
fn apply_profile(id: String) -> CmdResult<String> {
    if device::find().map_err(err)?.is_empty() {
        return Err("el macro pad no está conectado".into());
    }
    macropad_core::apply_profile(&id, Platform::current(), Trigger::Manual)
        .map(|o| o.profile_name)
        .map_err(err)
}

#[tauri::command(async)]
fn set_auto_profile(id: String) -> CmdResult<()> {
    let (profile, _) = Profile::load(&id).map_err(err)?;
    profile.validate(Platform::current()).map_err(err)?;
    let mut settings = Settings::load().map_err(err)?;
    settings.set_auto_profile(Platform::current(), &id);
    settings.save().map_err(err)
}

/// Quita la copia del usuario del perfil de desarrollo, lo vuelve a poner
/// como automático y lo aplica si el teclado está conectado.
#[tauri::command(async)]
fn restore_development() -> CmdResult<String> {
    let id = match Platform::current() {
        Platform::Macos => "desarrollo-macos",
        _ => "desarrollo-linux",
    };
    Profile::delete_user(id).map_err(err)?;
    let mut settings = Settings::load().map_err(err)?;
    settings.set_auto_profile(Platform::current(), id);
    settings.save().map_err(err)?;
    if device::find().map_err(err)?.is_empty() {
        return Ok(format!("«{id}» restaurado; se aplicará al conectar el macro pad"));
    }
    macropad_core::apply_profile(id, Platform::current(), Trigger::Manual)
        .map(|o| format!("«{}» restaurado y aplicado", o.profile_name))
        .map_err(err)
}

#[derive(Serialize)]
struct ActionInfo {
    id: &'static str,
    aliases: &'static [&'static str],
    description: &'static str,
    category: &'static str,
    linux: &'static str,
    macos: &'static str,
}

#[derive(Serialize)]
struct Catalog {
    actions: Vec<ActionInfo>,
    keys: tool::KeyList,
}

#[tauri::command(async)]
fn get_catalog() -> Catalog {
    let actions = actions::ACTIONS
        .iter()
        .map(|a| ActionInfo {
            id: a.id,
            aliases: a.aliases,
            description: a.description,
            category: match a.category {
                Category::Edicion => "Edición",
                Category::Navegacion => "Navegación",
                Category::Multimedia => "Multimedia",
                Category::Desplazamiento => "Desplazamiento",
            },
            linux: a.linux,
            macos: a.macos,
        })
        .collect();
    let keys = tool::locate().and_then(|t| tool::keys(&t).ok()).unwrap_or_default();
    Catalog { actions, keys }
}

#[derive(Serialize)]
struct Diagnosis {
    report: diagnose::Report,
    text: String,
    history: Vec<state::Record>,
}

#[tauri::command(async)]
fn run_diagnose() -> Diagnosis {
    let report = diagnose::run();
    let mut text = report.to_text();
    let history = state::history(20);
    if !history.is_empty() {
        text.push_str("\n== Historial\n");
        for r in &history {
            let mark = if r.ok { "✓" } else { "✗" };
            text.push_str(&format!("{mark} {} {} {} {} {}\n", r.timestamp, r.profile, r.platform, r.trigger, r.message));
        }
    }
    Diagnosis { report, text, history }
}

#[tauri::command(async)]
fn restart_agent() -> CmdResult<()> {
    service::restart().map_err(err)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_status,
            list_profiles,
            get_profile,
            preview_profile,
            save_profile,
            delete_profile,
            apply_profile,
            set_auto_profile,
            restore_development,
            get_catalog,
            run_diagnose,
            restart_agent,
        ])
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar la ventana de MacroPad Agent");
}
