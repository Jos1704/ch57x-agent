mod agent;
mod notify;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use macropad_core::device::{self, Access};
use macropad_core::profile::Source;
use macropad_core::settings::Settings;
use macropad_core::{actions, paths, service, state, tool, Platform, Profile, Trigger};

/// Agente local que programa el macro pad USB 1189:8890 según el sistema operativo.
#[derive(Parser)]
#[command(name = "macropad-agent", version)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Estado de conexión, último perfil aplicado y servicio.
    Status,
    /// Aplica un perfil ahora (identificador o ruta a un .yaml).
    Apply {
        profile: String,
        /// Sistema destino; por defecto el actual.
        #[arg(long)]
        platform: Option<Platform>,
    },
    /// Revisa hardware, permisos y herramienta de programación.
    Diagnose {
        /// Además, programa el perfil de prueba (cada tecla escribe su número).
        #[arg(long)]
        apply_test: bool,
    },
    /// Comprueba un perfil y muestra cómo quedará cada control, sin programar.
    Validate {
        profile: String,
        #[arg(long)]
        platform: Option<Platform>,
    },
    /// Lista los perfiles disponibles.
    Profiles,
    /// Lista las acciones con nombre que se pueden usar en los perfiles.
    Actions,
    /// Historial de intentos de programación.
    History {
        #[arg(short, default_value_t = 10)]
        n: usize,
    },
    /// Ejecuta el agente: escucha conexiones USB y aplica el perfil automático.
    Run,
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Cmd::Status => status(),
        Cmd::Apply { profile, platform } => apply(&profile, platform),
        Cmd::Diagnose { apply_test } => diagnose(apply_test),
        Cmd::Validate { profile, platform } => validate(&profile, platform),
        Cmd::Profiles => profiles(),
        Cmd::Actions => list_actions(),
        Cmd::History { n } => history(n),
        Cmd::Run => agent::run(),
    };
    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn status() -> Result<()> {
    let settings = Settings::load()?;
    let found = device::find()?;
    match found.first() {
        Some(d) => {
            let access = match device::describe(d).map(|r| r.access) {
                Ok(Access::Ok) => "con permisos".to_string(),
                Ok(Access::Denied) => "SIN permisos (falta la regla udev)".to_string(),
                Ok(Access::Error(e)) => format!("error al abrir: {e}"),
                Err(e) => format!("error al inspeccionar: {e:#}"),
            };
            println!("Macro pad:        conectado en {} ({access})", d.usb_path());
        }
        None => println!("Macro pad:        no conectado"),
    }
    println!("Sistema:          {}", Platform::current());
    println!("Perfil automático: {}", settings.auto_profile_for(Platform::current()));
    match state::last_applied() {
        Some(r) => println!("Último aplicado:  {} ({}, {}, {})", r.profile, r.platform, r.trigger, r.timestamp),
        None => println!("Último aplicado:  ninguno"),
    }
    if let Some(r) = state::history(1).first().filter(|r| !r.ok) {
        println!("Último intento:   FALLÓ {} — {}", r.timestamp, r.message);
    }
    let agent = match service::is_active() {
        Some(true) => "activo",
        Some(false) => "detenido",
        None => "desconocido",
    };
    println!("Agente:           {agent}");
    println!("Configuración:    {}", paths::config_dir().display());
    Ok(())
}

fn apply(id: &str, platform: Option<Platform>) -> Result<()> {
    let target = platform.unwrap_or_else(Platform::current);
    if device::find()?.is_empty() {
        bail!("el macro pad {:04x}:{:04x} no está conectado", device::VENDOR_ID, device::PRODUCT_ID);
    }
    let outcome = macropad_core::apply_profile(id, target, Trigger::Manual)?;
    for w in &outcome.warnings {
        println!("aviso: {w}");
    }
    println!("✓ Perfil «{}» aplicado ({})", outcome.profile_name, target.effective());
    Ok(())
}

fn validate(id: &str, platform: Option<Platform>) -> Result<()> {
    let (profile, _) = Profile::load(id)?;
    let target = platform.unwrap_or_else(|| profile.platform.effective());
    let warnings = profile.validate(target)?;
    println!("{} ({})", profile.name, target.effective());
    for (control, value) in profile.resolved(target) {
        println!("  {control:<11} {}", value.as_deref().unwrap_or("—"));
    }
    for w in &warnings {
        println!("aviso: {w}");
    }
    let config = profile.to_ch57x_yaml(target)?;
    match tool::locate() {
        Some(t) => {
            tool::validate(&t, &config)?;
            println!("✓ válido según {}", tool::BINARY);
        }
        None => println!("aviso: {} no encontrado; solo se hizo la validación propia", tool::BINARY),
    }
    Ok(())
}

fn profiles() -> Result<()> {
    for entry in Profile::list() {
        let origin = match &entry.source {
            Source::Builtin => "incluido".to_string(),
            Source::User(p) => p.display().to_string(),
        };
        match Profile::load(&entry.id) {
            Ok((p, _)) => println!("{:<20} {:<24} {:<6} {origin}", entry.id, p.name, p.platform),
            Err(e) => println!("{:<20} ERROR: {e:#}", entry.id),
        }
    }
    Ok(())
}

fn list_actions() -> Result<()> {
    println!("{:<18} {:<14} {:<14} descripción", "acción", "linux", "macos");
    for a in actions::ACTIONS {
        println!("{:<18} {:<14} {:<14} {}", a.id, a.linux, a.macos, a.description);
    }
    println!("\nTambién se acepta cualquier valor de `{} show-keys` (p. ej. ctrl-alt-t).", tool::BINARY);
    Ok(())
}

fn history(n: usize) -> Result<()> {
    for r in state::history(n) {
        let mark = if r.ok { "✓" } else { "✗" };
        println!("{mark} {} {:<18} {:<6} {:<8} {}", r.timestamp, r.profile, r.platform, r.trigger, r.message);
    }
    Ok(())
}

fn diagnose(apply_test: bool) -> Result<()> {
    let report = macropad_core::diagnose::run();
    print!("{}", report.to_text());

    if apply_test && report.connected {
        let auto = Settings::load()?.auto_profile_for(Platform::current()).to_string();
        println!("\n== Perfil de prueba");
        let outcome = macropad_core::apply_profile("prueba", Platform::current(), Trigger::Manual)?;
        println!("✓ «{}» aplicado.", outcome.profile_name);
        println!("  Pulsa las teclas en un editor: deben escribir 1 a 6 en orden (fila superior 1-2-3).");
        println!("  Gira la perilla (volumen) y púlsala (silencio).");
        println!("  Para volver al perfil de desarrollo: macropad-agent apply {auto}");
    }

    println!();
    match report.problems() {
        0 => {
            println!("Todo listo.");
            Ok(())
        }
        n => bail!("{n} problema(s) encontrado(s)"),
    }
}
