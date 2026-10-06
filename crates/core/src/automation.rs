//! Comandos y flujos que ejecuta el agente al pulsar una tecla del pad.
//!
//! El teclado solo guarda pulsaciones, así que una tecla con comando se
//! programa para enviar una tecla reservada (`F13`–`F19` o `Shift+F13`/`F14`,
//! una por control; ver `CONTROL_KEYS`) y
//! el agente, que escucha solo los dispositivos del pad, ejecuta lo asignado.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::paths;

/// Lo que hace un control: una pulsación que guarda el teclado (atajo,
/// multimedia, valor directo) o una automatización que ejecuta el agente.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Binding {
    Keys(String),
    Command(CommandAction),
    Flow(FlowAction),
}

fn yes() -> bool {
    true
}

fn is_true(b: &bool) -> bool {
    *b
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandAction {
    /// Se ejecuta con el shell del usuario; acepta tuberías y variables.
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Carpeta de trabajo (`~` permitido).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Variables de entorno extra, por ejemplo `PUERTO: "3000"`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vars: BTreeMap<String, String>,
    /// Notificar cuando termine.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub notify: bool,
    /// Abrir en una terminal para ver la salida.
    #[serde(default, skip_serializing_if = "is_false")]
    pub terminal: bool,
    /// Ignorar la tecla si ya se está ejecutando.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub single: bool,
    /// Pedir una segunda pulsación en 3 s antes de ejecutar.
    #[serde(default, skip_serializing_if = "is_false")]
    pub confirm: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowAction {
    pub flow: Vec<Step>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vars: BTreeMap<String, String>,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub notify: bool,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub single: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub confirm: bool,
    /// Ejecutar también al conectar el pad, una vez al día.
    #[serde(default, skip_serializing_if = "is_false")]
    pub on_connect: bool,
}

/// Un paso de un flujo: exactamente uno de `run`, `terminal` u `open`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Comando que se espera a que termine (salvo `detach`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    /// Abre una terminal en la carpeta; el valor es el comando a correr
    /// dentro (vacío: solo el shell).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<String>,
    /// URL o archivo que se abre con la app predeterminada.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<String>,
    /// No esperar a que `run` termine (apps que se quedan abiertas).
    #[serde(default, skip_serializing_if = "is_false")]
    pub detach: bool,
    /// Espera antes del paso, en milisegundos.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<u64>,
}

impl Binding {
    pub fn is_automation(&self) -> bool {
        !matches!(self, Binding::Keys(_))
    }

    pub fn label(&self) -> Option<&str> {
        match self {
            Binding::Keys(_) => None,
            Binding::Command(c) => c.label.as_deref(),
            Binding::Flow(f) => f.label.as_deref(),
        }
    }

    /// Nombre para mostrar: la etiqueta o, si no hay, el comando.
    pub fn display_name(&self) -> String {
        match self {
            Binding::Keys(k) => k.clone(),
            Binding::Command(c) => c.label.clone().unwrap_or_else(|| c.command.clone()),
            Binding::Flow(f) => f.label.clone().unwrap_or_else(|| format!("Flujo de {} pasos", f.flow.len())),
        }
    }

    pub fn confirm(&self) -> bool {
        match self {
            Binding::Keys(_) => false,
            Binding::Command(c) => c.confirm,
            Binding::Flow(f) => f.confirm,
        }
    }

    pub fn single(&self) -> bool {
        match self {
            Binding::Keys(_) => false,
            Binding::Command(c) => c.single,
            Binding::Flow(f) => f.single,
        }
    }

    pub fn notify(&self) -> bool {
        match self {
            Binding::Keys(_) => false,
            Binding::Command(c) => c.notify,
            Binding::Flow(f) => f.notify,
        }
    }

    pub fn on_connect(&self) -> bool {
        matches!(self, Binding::Flow(f) if f.on_connect)
    }

    /// Validación de la automatización (las teclas las valida la herramienta).
    pub fn validate(&self) -> Result<()> {
        match self {
            Binding::Keys(k) if k.trim().is_empty() => bail!("el valor está vacío"),
            Binding::Keys(_) => Ok(()),
            Binding::Command(c) => {
                if c.command.trim().is_empty() {
                    bail!("el comando está vacío");
                }
                check_vars(&c.vars)
            }
            Binding::Flow(f) => {
                if f.flow.is_empty() {
                    bail!("el flujo no tiene pasos");
                }
                for (i, step) in f.flow.iter().enumerate() {
                    let kinds = [&step.run, &step.terminal, &step.open].iter().filter(|s| s.is_some()).count();
                    if kinds != 1 {
                        bail!("el paso {} debe tener exactamente uno de: run, terminal u open", i + 1);
                    }
                    let empty = |s: &Option<String>| s.as_deref().is_some_and(|v| v.trim().is_empty());
                    if empty(&step.run) || empty(&step.open) {
                        bail!("el paso {} está vacío", i + 1);
                    }
                }
                check_vars(&f.vars)
            }
        }
    }
}

fn check_vars(vars: &BTreeMap<String, String>) -> Result<()> {
    for name in vars.keys() {
        let valid = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !valid {
            bail!("nombre de variable no válido: «{name}»");
        }
    }
    Ok(())
}

/// Controles del pad y lo que envían cuando tienen una automatización:
/// (control, valor para `ch57x-keyboard-tool`, código de tecla de Linux, con Shift).
///
/// Son teclas que ningún teclado normal tiene. `F20` y `F21` no se usan porque
/// el mapa de teclado de Linux las convierte en «silenciar micrófono» y
/// «activar/desactivar touchpad»; la perilla usa `F19` y `Shift+F13`/`Shift+F14`.
pub const CONTROL_KEYS: &[(&str, &str, u16, bool)] = &[
    ("key_1", "f13", 183, false),
    ("key_2", "f14", 184, false),
    ("key_3", "f15", 185, false),
    ("key_4", "f16", 186, false),
    ("key_5", "f17", 187, false),
    ("key_6", "f18", 188, false),
    ("knob_left", "f19", 189, false),
    ("knob_press", "shift-f13", 183, true),
    ("knob_right", "shift-f14", 184, true),
];

pub fn reserved_key(control: &str) -> Option<&'static str> {
    CONTROL_KEYS.iter().find(|(c, ..)| *c == control).map(|(_, k, ..)| *k)
}

/// Control al que corresponde un código de tecla de Linux (evdev), según
/// si Shift está pulsado.
pub fn control_for_keycode(code: u16, shift: bool) -> Option<&'static str> {
    CONTROL_KEYS.iter().find(|(_, _, k, s)| *k == code && *s == shift).map(|(c, ..)| *c)
}

pub fn control_label(control: &str) -> String {
    match control {
        "knob_left" => "Perilla ↺".into(),
        "knob_right" => "Perilla ↻".into(),
        "knob_press" => "Perilla (pulsar)".into(),
        c => c.strip_prefix("key_").map(|n| format!("Tecla {n}")).unwrap_or_else(|| c.to_string()),
    }
}

// ---------- ejecución ----------

#[derive(Debug, Clone, Serialize)]
pub struct Outcome {
    pub ok: bool,
    pub code: Option<i32>,
    pub duration_ms: u64,
    /// Últimas líneas de la salida (stdout y stderr juntos).
    pub output: String,
}

const OUTPUT_LIMIT: usize = 4000;

/// Ejecuta la automatización y espera a que termine (los pasos con
/// `detach`, `terminal` u `open` solo se lanzan).
pub fn run(binding: &Binding) -> Outcome {
    let start = Instant::now();
    let (ok, code, output) = match binding {
        Binding::Keys(k) => (false, None, format!("«{k}» es una pulsación, no un comando")),
        Binding::Command(c) => run_command(c),
        Binding::Flow(f) => run_flow(f),
    };
    Outcome { ok, code, duration_ms: start.elapsed().as_millis() as u64, output: tail(&output) }
}

fn run_command(c: &CommandAction) -> (bool, Option<i32>, String) {
    let cwd = resolve_cwd(c.cwd.as_deref(), &c.vars);
    if c.terminal {
        return match open_terminal(&c.command, cwd.as_ref(), &c.vars) {
            Ok(()) => (true, None, "abierto en una terminal".into()),
            Err(e) => (false, None, format!("{e:#}")),
        };
    }
    shell_wait(&c.command, cwd.as_ref(), &c.vars)
}

fn run_flow(f: &FlowAction) -> (bool, Option<i32>, String) {
    let cwd = resolve_cwd(f.cwd.as_deref(), &f.vars);
    let mut log = String::new();
    for (i, step) in f.flow.iter().enumerate() {
        if let Some(ms) = step.delay_ms {
            thread::sleep(Duration::from_millis(ms));
        }
        let name = step.label.clone().unwrap_or_else(|| format!("Paso {}", i + 1));
        log.push_str(&format!("▸ {name}\n"));
        let result: std::result::Result<(), (Option<i32>, String)> = if let Some(cmd) = &step.run {
            if step.detach {
                spawn_detached(shell(cmd, cwd.as_ref(), &f.vars)).map_err(|e| (None, format!("{e:#}")))
            } else {
                let (ok, code, out) = shell_wait(cmd, cwd.as_ref(), &f.vars);
                log.push_str(&out);
                if ok { Ok(()) } else { Err((code, String::new())) }
            }
        } else if let Some(cmd) = &step.terminal {
            open_terminal(cmd, cwd.as_ref(), &f.vars).map_err(|e| (None, format!("{e:#}")))
        } else if let Some(target) = &step.open {
            open_target(&expand_vars(target, &f.vars)).map_err(|e| (None, format!("{e:#}")))
        } else {
            Err((None, "paso sin acción".into()))
        };
        if let Err((code, msg)) = result {
            if !msg.is_empty() {
                log.push_str(&msg);
                log.push('\n');
            }
            log.push_str(&format!("✗ el flujo se detuvo en «{name}»\n"));
            return (false, code, log);
        }
    }
    (true, Some(0), log)
}

fn tail(s: &str) -> String {
    let s = s.trim_end();
    if s.len() <= OUTPUT_LIMIT {
        return s.to_string();
    }
    let mut start = s.len() - OUTPUT_LIMIT;
    while !s.is_char_boundary(start) {
        start += 1;
    }
    format!("…{}", &s[start..])
}

fn resolve_cwd(cwd: Option<&str>, vars: &BTreeMap<String, String>) -> Option<PathBuf> {
    let cwd = cwd.map(str::trim).filter(|c| !c.is_empty())?;
    Some(expand_home(&expand_vars(cwd, vars)))
}

pub fn expand_home(path: &str) -> PathBuf {
    match path.strip_prefix("~") {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => {
            paths::home_dir().join(rest.trim_start_matches('/'))
        }
        _ => PathBuf::from(path),
    }
}

/// Sustituye `$NOMBRE` y `${NOMBRE}` por las variables declaradas (las demás
/// quedan tal cual).
pub fn expand_vars(s: &str, vars: &BTreeMap<String, String>) -> String {
    let mut out = s.to_string();
    for (name, value) in vars {
        out = out.replace(&format!("${{{name}}}"), value);
    }
    // `$NOMBRE` después, de los nombres más largos a los más cortos.
    let mut names: Vec<&String> = vars.keys().collect();
    names.sort_by_key(|n| std::cmp::Reverse(n.len()));
    for name in names {
        out = out.replace(&format!("${name}"), &vars[name]);
    }
    out
}

fn shell(cmd: &str, cwd: Option<&PathBuf>, vars: &BTreeMap<String, String>) -> Command {
    let user_shell = std::env::var("SHELL").ok().filter(|s| !s.is_empty());
    let mut command = if cfg!(target_os = "macos") {
        // launchd arranca con un PATH mínimo: el shell de sesión carga el del usuario.
        let mut c = Command::new(user_shell.as_deref().unwrap_or("/bin/zsh"));
        c.args(["-lc", cmd]);
        c
    } else {
        let mut c = Command::new(user_shell.as_deref().unwrap_or("/bin/sh"));
        c.args(["-c", cmd]);
        c
    };
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    command.envs(vars);
    command
}

fn shell_wait(cmd: &str, cwd: Option<&PathBuf>, vars: &BTreeMap<String, String>) -> (bool, Option<i32>, String) {
    if let Some(dir) = cwd.filter(|d| !d.is_dir()) {
        return (false, None, format!("la carpeta {} no existe", dir.display()));
    }
    let mut command = shell(cmd, cwd, vars);
    // stdout y stderr al mismo pipe, en el orden en que llegan.
    let (mut reader, writer) = match std::io::pipe() {
        Ok(p) => p,
        Err(e) => return (false, None, format!("no se pudo crear la tubería: {e}")),
    };
    let writer2 = match writer.try_clone() {
        Ok(w) => w,
        Err(e) => return (false, None, format!("no se pudo crear la tubería: {e}")),
    };
    command.stdin(Stdio::null()).stdout(writer).stderr(writer2);
    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) => return (false, None, format!("no se pudo ejecutar: {e}")),
    };
    drop(command); // cierra nuestras copias del extremo de escritura
    let mut output = Vec::new();
    let _ = reader.read_to_end(&mut output);
    let status = child.wait();
    let text = String::from_utf8_lossy(&output).to_string();
    match status {
        Ok(s) => (s.success(), s.code(), text),
        Err(e) => (false, None, format!("{text}\n{e}")),
    }
}

/// Lanza sin esperar; un hilo recoge el proceso al terminar.
fn spawn_detached(mut command: Command) -> Result<()> {
    command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    let mut child = command.spawn()?;
    thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

fn open_terminal(cmd: &str, cwd: Option<&PathBuf>, vars: &BTreeMap<String, String>) -> Result<()> {
    if let Some(dir) = cwd.filter(|d| !d.is_dir()) {
        bail!("la carpeta {} no existe", dir.display());
    }
    let cmd = cmd.trim();
    if cfg!(target_os = "macos") {
        let dir = cwd.map(|d| d.display().to_string()).unwrap_or_else(|| paths::home_dir().display().to_string());
        let exports: String = vars.iter().map(|(k, v)| format!("export {k}={}; ", sh_quote(v))).collect();
        let script = format!("cd {}; {exports}{cmd}", sh_quote(&dir));
        let apple = format!(
            "tell application \"Terminal\"\nactivate\ndo script {}\nend tell",
            applescript_string(&script)
        );
        let mut c = Command::new("osascript");
        c.args(["-e", &apple]);
        return spawn_detached(c);
    }
    let user_shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    let term = linux_terminal().with_context(|| {
        format!("no se encontró una terminal; instala una de: {}", TERMINALS.iter().map(|t| t.bin).collect::<Vec<_>>().join(", "))
    })?;
    let mut c = Command::new(term.bin);
    c.args(term.pre);
    if let Some(dir) = cwd {
        match term.cwd {
            Cwd::Joined(flag) => {
                c.arg(format!("{flag}{}", dir.display()));
            }
            Cwd::Separate(flag) => {
                c.arg(flag).arg(dir);
            }
            Cwd::Inherit => {}
        }
        // Para las terminales sin opción de carpeta, y como respaldo.
        c.current_dir(dir);
    }
    if !cmd.is_empty() {
        // Al terminar el comando, la terminal queda abierta con el shell.
        c.args(term.exec);
        c.args([user_shell.as_str(), "-c", &format!("{cmd}; exec {user_shell}")]);
    }
    c.envs(vars);
    spawn_detached(c)
}

/// Cómo se le indica la carpeta de trabajo a una terminal.
enum Cwd {
    /// `--opcion=carpeta`
    Joined(&'static str),
    /// `--opcion carpeta`
    Separate(&'static str),
    /// La hereda del proceso.
    Inherit,
}

struct Terminal {
    bin: &'static str,
    pre: &'static [&'static str],
    cwd: Cwd,
    /// Lo que va antes del comando a ejecutar.
    exec: &'static [&'static str],
}

/// Terminales conocidas, en orden de preferencia. `$TERMINAL` va primero si es una de ellas.
const TERMINALS: &[Terminal] = &[
    Terminal { bin: "xdg-terminal-exec", pre: &[], cwd: Cwd::Joined("--dir="), exec: &[] },
    Terminal { bin: "gnome-terminal", pre: &[], cwd: Cwd::Joined("--working-directory="), exec: &["--"] },
    Terminal { bin: "ptyxis", pre: &["--new-window"], cwd: Cwd::Joined("--working-directory="), exec: &["--"] },
    Terminal { bin: "kgx", pre: &[], cwd: Cwd::Joined("--working-directory="), exec: &["--"] },
    Terminal { bin: "konsole", pre: &[], cwd: Cwd::Separate("--workdir"), exec: &["-e"] },
    Terminal { bin: "xfce4-terminal", pre: &[], cwd: Cwd::Joined("--working-directory="), exec: &["-x"] },
    Terminal { bin: "kitty", pre: &[], cwd: Cwd::Separate("--directory"), exec: &[] },
    Terminal { bin: "alacritty", pre: &[], cwd: Cwd::Separate("--working-directory"), exec: &["-e"] },
    Terminal { bin: "foot", pre: &[], cwd: Cwd::Joined("--working-directory="), exec: &[] },
    Terminal { bin: "wezterm", pre: &["start"], cwd: Cwd::Separate("--cwd"), exec: &["--"] },
    Terminal { bin: "x-terminal-emulator", pre: &[], cwd: Cwd::Inherit, exec: &["-e"] },
    Terminal { bin: "xterm", pre: &[], cwd: Cwd::Inherit, exec: &["-e"] },
];

fn linux_terminal() -> Option<&'static Terminal> {
    let preferred = std::env::var("TERMINAL").ok();
    let preferred = preferred.as_deref().and_then(|t| TERMINALS.iter().find(|x| x.bin == t));
    preferred.filter(|t| which(t.bin)).or_else(|| TERMINALS.iter().find(|t| which(t.bin)))
}

fn open_target(target: &str) -> Result<()> {
    let target = target.trim();
    let mut c = if cfg!(target_os = "macos") {
        Command::new("open")
    } else {
        Command::new("xdg-open")
    };
    let target = if target.starts_with('~') { expand_home(target).display().to_string() } else { target.to_string() };
    c.arg(target);
    spawn_detached(c)
}

fn which(bin: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join(bin).is_file()))
        .unwrap_or(false)
}

fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn applescript_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_binding_kinds() {
        let yaml = r#"
- ctrl-s
- command: ./deploy.sh staging
  label: Desplegar
  confirm: true
- flow:
    - terminal: ""
    - run: code .
      detach: true
    - open: http://localhost:$PUERTO
      delay_ms: 500
  label: Empezar a trabajar
  cwd: ~/Projects/demo
  vars: { PUERTO: "3000" }
  on_connect: true
"#;
        let list: Vec<Binding> = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(&list[0], Binding::Keys(k) if k == "ctrl-s"));
        assert!(matches!(&list[1], Binding::Command(c) if c.confirm && c.notify && c.single));
        let Binding::Flow(f) = &list[2] else { panic!("no es flujo") };
        assert_eq!(f.flow.len(), 3);
        assert!(f.on_connect);
        for b in &list {
            b.validate().unwrap();
        }
        // Ida y vuelta sin perder nada.
        let again: Vec<Binding> = serde_yaml::from_str(&serde_yaml::to_string(&list).unwrap()).unwrap();
        assert_eq!(again, list);
    }

    #[test]
    fn rejects_invalid_steps() {
        let bad: Binding = serde_yaml::from_str("flow:\n  - run: a\n    open: b\n").unwrap();
        assert!(bad.validate().is_err());
        assert!(serde_yaml::from_str::<Binding>("command: a\nflow: []\n").is_err());
    }

    #[test]
    fn expands_vars() {
        let vars = BTreeMap::from([("PUERTO".to_string(), "3000".to_string()), ("P".to_string(), "x".to_string())]);
        assert_eq!(expand_vars("http://localhost:$PUERTO/${P}", &vars), "http://localhost:3000/x");
    }

    #[test]
    fn runs_command_and_flow() {
        let c = Binding::Command(CommandAction {
            command: "echo hola; echo error >&2; exit 3".into(),
            label: None,
            cwd: Some("/".into()),
            vars: BTreeMap::new(),
            notify: true,
            terminal: false,
            single: true,
            confirm: false,
        });
        let out = run(&c);
        assert!(!out.ok);
        assert_eq!(out.code, Some(3));
        assert!(out.output.contains("hola") && out.output.contains("error"));

        let f: Binding = serde_yaml::from_str(
            "flow:\n  - run: echo $NOMBRE\n    label: Saludo\n  - run: \"false\"\n  - run: echo nunca\nvars: { NOMBRE: mundo }\n",
        )
        .unwrap();
        let out = run(&f);
        assert!(!out.ok);
        assert!(out.output.contains("mundo"));
        assert!(out.output.contains("se detuvo en «Paso 2»"));
        assert!(!out.output.contains("nunca"));
    }

    #[test]
    fn maps_controls_to_reserved_keys() {
        assert_eq!(reserved_key("key_6"), Some("f18"));
        assert_eq!(reserved_key("knob_press"), Some("shift-f13"));
        assert_eq!(control_for_keycode(183, false), Some("key_1"));
        assert_eq!(control_for_keycode(183, true), Some("knob_press"));
        assert_eq!(control_for_keycode(190, false), None);
    }
}
