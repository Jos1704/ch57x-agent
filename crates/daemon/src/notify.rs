//! Notificaciones breves de escritorio, sin dependencias adicionales.

use std::process::{Command, Stdio};

pub fn send(title: &str, body: &str) {
    let result = if cfg!(target_os = "macos") {
        let script = format!(
            "display notification {} with title {}",
            applescript_string(body),
            applescript_string(title)
        );
        Command::new("osascript").args(["-e", &script]).stdout(Stdio::null()).stderr(Stdio::null()).status()
    } else {
        Command::new("notify-send")
            .args(["--app-name=MacroPad Agent", "--expire-time=4000", title, body])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    };
    if let Err(e) = result {
        eprintln!("aviso: no se pudo mostrar la notificación: {e}");
    }
}

fn applescript_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}
