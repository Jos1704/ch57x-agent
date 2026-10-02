//! Proceso en segundo plano: espera la conexión del macro pad y aplica el
//! perfil del sistema operativo.

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use macropad_core::device::{self, Found};
use macropad_core::settings::Settings;
use macropad_core::{Platform, Trigger};
use rusb::{Context, Device, Hotplug, HotplugBuilder, UsbContext};

use crate::notify;

const RETRIES: u32 = 3;
const POLL_INTERVAL: Duration = Duration::from_secs(2);
/// Ignora una segunda llegada del mismo dispositivo en este intervalo.
const DEDUP_WINDOW: Duration = Duration::from_secs(5);

enum Event {
    Arrived(Found, Trigger),
    Left(Found),
}

struct Watcher(Sender<Event>);

impl Hotplug<Context> for Watcher {
    fn device_arrived(&mut self, device: Device<Context>) {
        let _ = self.0.send(Event::Arrived(found(&device), Trigger::Connect));
    }

    fn device_left(&mut self, device: Device<Context>) {
        let _ = self.0.send(Event::Left(found(&device)));
    }
}

fn found(device: &Device<Context>) -> Found {
    Found { bus: device.bus_number(), address: device.address() }
}

pub fn run() -> Result<()> {
    let settings = Settings::load()?;
    let platform = Platform::current();
    let profile = settings.auto_profile_for(platform).to_string();
    eprintln!(
        "macropad-agent {} iniciado; sistema {platform}, perfil automático «{profile}»",
        env!("CARGO_PKG_VERSION")
    );

    let (tx, rx) = mpsc::channel();
    let _registration = if rusb::has_hotplug() {
        let ctx = Context::new().context("iniciar libusb")?;
        let registration = HotplugBuilder::new()
            .vendor_id(device::VENDOR_ID)
            .product_id(device::PRODUCT_ID)
            .register(ctx.clone(), Box::new(Watcher(tx.clone())))
            .context("registrar eventos USB")?;
        thread::spawn(move || loop {
            if let Err(e) = ctx.handle_events(None) {
                eprintln!("error en eventos USB: {e}");
                thread::sleep(Duration::from_secs(1));
            }
        });
        eprintln!("escuchando eventos USB (hotplug)");
        Some(registration)
    } else {
        spawn_poller(tx.clone());
        eprintln!("hotplug no disponible; revisando cada {}s", POLL_INTERVAL.as_secs());
        None
    };

    // Al iniciar sesión el teclado puede estar ya conectado.
    for dev in device::find()? {
        let _ = tx.send(Event::Arrived(dev, Trigger::Startup));
    }

    handle_events(rx, settings, platform);
    Ok(())
}

fn spawn_poller(tx: Sender<Event>) {
    thread::spawn(move || {
        let mut known: Vec<Found> = device::find().unwrap_or_default();
        loop {
            thread::sleep(POLL_INTERVAL);
            let Ok(now) = device::find() else { continue };
            for d in now.iter().filter(|d| !known.contains(d)) {
                let _ = tx.send(Event::Arrived(d.clone(), Trigger::Connect));
            }
            for d in known.iter().filter(|d| !now.contains(d)) {
                let _ = tx.send(Event::Left(d.clone()));
            }
            known = now;
        }
    });
}

fn handle_events(rx: Receiver<Event>, mut settings: Settings, platform: Platform) {
    let mut last: Option<(Found, Instant)> = None;
    for event in rx {
        match event {
            Event::Left(dev) => eprintln!("desconectado: {}", dev.usb_path()),
            Event::Arrived(dev, trigger) => {
                if let Some((prev, at)) = &last
                    && *prev == dev
                    && at.elapsed() < DEDUP_WINDOW
                {
                    continue;
                }
                eprintln!("conectado: {} ({trigger})", dev.usb_path());
                // La ventana de configuración puede haber cambiado el perfil automático.
                match Settings::load() {
                    Ok(s) => settings = s,
                    Err(e) => eprintln!("aviso: se conserva la configuración anterior: {e:#}"),
                }
                thread::sleep(Duration::from_millis(settings.settle_delay_ms));
                let profile = settings.auto_profile_for(platform).to_string();
                apply_with_retry(&dev, &settings, platform, &profile, trigger);
                last = Some((dev, Instant::now()));
            }
        }
    }
}

fn apply_with_retry(dev: &Found, settings: &Settings, platform: Platform, profile: &str, trigger: Trigger) {
    for attempt in 1..=RETRIES {
        if !device::find().map(|f| f.contains(dev)).unwrap_or(false) {
            eprintln!("el dispositivo se desconectó antes de programarlo");
            return;
        }
        match macropad_core::apply_profile(profile, platform, trigger) {
            Ok(outcome) => {
                for w in &outcome.warnings {
                    eprintln!("aviso: {w}");
                }
                eprintln!("perfil «{}» aplicado", outcome.profile_name);
                if settings.notifications {
                    notify::send("MacroPad Agent", &format!("Perfil «{}» aplicado", outcome.profile_name));
                }
                return;
            }
            Err(e) if attempt < RETRIES => {
                eprintln!("intento {attempt} falló: {e:#}; reintentando");
                thread::sleep(Duration::from_secs(attempt as u64));
            }
            Err(e) => {
                eprintln!("no se pudo aplicar «{profile}»: {e:#}");
                if settings.notifications {
                    notify::send("MacroPad Agent", &format!("Error al programar el macro pad: {e}"));
                }
            }
        }
    }
}
