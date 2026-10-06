//! Ejecución de comandos y flujos del pad: confirmación, una ejecución a la
//! vez por tecla, historial y notificaciones.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use macropad_core::automation::{self, Binding};
use macropad_core::runs::{self, Run};

use crate::notify;

const CONFIRM_WINDOW: Duration = Duration::from_secs(3);

#[derive(Default)]
pub struct Executor {
    running: Arc<Mutex<HashSet<String>>>,
    pending: HashMap<String, Instant>,
}

impl Executor {
    /// Atiende una pulsación (o una conexión). No bloquea: el comando corre
    /// en su propio hilo.
    pub fn trigger(&mut self, profile: &str, control: &str, binding: &Binding, trigger: &str) {
        let label = binding.display_name();
        let key = format!("{profile}:{control}");

        if binding.confirm() && trigger == "tecla" {
            match self.pending.remove(&key) {
                Some(at) if at.elapsed() <= CONFIRM_WINDOW => {}
                _ => {
                    self.pending.insert(key, Instant::now());
                    eprintln!("{control}: esperando confirmación de «{label}»");
                    notify::send("MacroPad Agent", &format!("Pulsa otra vez para confirmar: {label}"));
                    return;
                }
            }
        }

        if binding.single() && !self.running.lock().unwrap().insert(key.clone()) {
            eprintln!("{control}: «{label}» ya se está ejecutando; se ignora");
            let _ = runs::save(&Run::ignored(profile, control, &label, trigger, "ya se estaba ejecutando"));
            return;
        }

        let running = self.running.clone();
        let (profile, control, binding, trigger) =
            (profile.to_string(), control.to_string(), binding.clone(), trigger.to_string());
        thread::spawn(move || {
            eprintln!("{control}: ejecutando «{label}» ({trigger})");
            let outcome = automation::run(&binding);
            running.lock().unwrap().remove(&key);
            let run = Run::finished(&profile, &control, &label, &trigger, &outcome);
            if let Err(e) = runs::save(&run) {
                eprintln!("aviso: no se pudo guardar el historial de comandos: {e:#}");
            }
            let last_line = outcome.output.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").to_string();
            if outcome.ok {
                eprintln!("{control}: «{label}» terminó en {} ms", outcome.duration_ms);
                if binding.notify() {
                    notify::send(&format!("✓ {label}"), &last_line);
                }
            } else {
                eprintln!("{control}: «{label}» falló: {}", outcome.output.replace('\n', " | "));
                // Los errores se notifican siempre.
                let code = outcome.code.map(|c| format!(" (código {c})")).unwrap_or_default();
                notify::send(&format!("✗ {label}{code}"), &last_line);
            }
        });
    }
}
