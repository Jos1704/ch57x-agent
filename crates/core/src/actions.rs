//! Catálogo de acciones con nombre. Una acción como `guardar` se traduce a
//! la combinación correspondiente de cada sistema (`ctrl-s` / `cmd-s`).
//! Cualquier valor que no sea una acción se pasa tal cual a
//! `ch57x-keyboard-tool` (por ejemplo `ctrl-alt-t` o `f5`).

use crate::platform::Platform;

pub struct Action {
    pub id: &'static str,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
    pub category: Category,
    pub linux: &'static str,
    pub macos: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Edicion,
    Navegacion,
    Multimedia,
    Desplazamiento,
}

macro_rules! action {
    ($id:literal, [$($alias:literal),*], $desc:literal, $cat:ident, $linux:literal, $macos:literal) => {
        Action {
            id: $id,
            aliases: &[$($alias),*],
            description: $desc,
            category: Category::$cat,
            linux: $linux,
            macos: $macos,
        }
    };
}

pub const ACTIONS: &[Action] = &[
    action!("guardar", ["save"], "Guardar", Edicion, "ctrl-s", "cmd-s"),
    action!("deshacer", ["undo"], "Deshacer", Edicion, "ctrl-z", "cmd-z"),
    action!("rehacer", ["redo"], "Rehacer", Edicion, "ctrl-shift-z", "cmd-shift-z"),
    action!("copiar", ["copy"], "Copiar", Edicion, "ctrl-c", "cmd-c"),
    action!("cortar", ["cut"], "Cortar", Edicion, "ctrl-x", "cmd-x"),
    action!("pegar", ["paste"], "Pegar", Edicion, "ctrl-v", "cmd-v"),
    action!("seleccionar-todo", ["select-all"], "Seleccionar todo", Edicion, "ctrl-a", "cmd-a"),
    action!("comentar", ["toggle-comment"], "Comentar línea", Edicion, "ctrl-slash", "cmd-slash"),
    action!("buscar", ["find"], "Buscar", Navegacion, "ctrl-f", "cmd-f"),
    action!("abrir-archivo", ["open-file"], "Abrir archivo", Navegacion, "ctrl-p", "cmd-p"),
    action!("paleta-comandos", ["command-palette"], "Paleta de comandos", Navegacion, "ctrl-shift-p", "cmd-shift-p"),
    action!("terminal", ["toggle-terminal"], "Mostrar terminal", Navegacion, "ctrl-grave", "ctrl-grave"),
    action!("nueva-pestana", ["new-tab"], "Nueva pestaña", Navegacion, "ctrl-t", "cmd-t"),
    action!("cerrar-pestana", ["close-tab"], "Cerrar pestaña", Navegacion, "ctrl-w", "cmd-w"),
    action!("ejecutar", ["run", "depurar", "debug"], "Ejecutar o depurar", Navegacion, "f5", "f5"),
    action!("volumen-subir", ["volume_up", "volume-up"], "Subir volumen", Multimedia, "volumeup", "volumeup"),
    action!("volumen-bajar", ["volume_down", "volume-down"], "Bajar volumen", Multimedia, "volumedown", "volumedown"),
    action!("silenciar", ["mute"], "Silenciar", Multimedia, "mute", "mute"),
    action!("reproducir", ["play", "play_pause"], "Reproducir/pausar", Multimedia, "play", "play"),
    action!("siguiente", ["next"], "Pista siguiente", Multimedia, "next", "next"),
    action!("anterior", ["prev", "previous"], "Pista anterior", Multimedia, "prev", "prev"),
    action!("desplazar-arriba", ["scroll_up", "scroll-up"], "Desplazar hacia arriba", Desplazamiento, "wheelup", "wheelup"),
    action!("desplazar-abajo", ["scroll_down", "scroll-down"], "Desplazar hacia abajo", Desplazamiento, "wheeldown", "wheeldown"),
];

pub fn find(name: &str) -> Option<&'static Action> {
    let name = name.trim().to_ascii_lowercase();
    ACTIONS
        .iter()
        .find(|a| a.id == name || a.aliases.contains(&name.as_str()))
}

/// Traduce un valor de perfil a la sintaxis de `ch57x-keyboard-tool`.
pub fn resolve(value: &str, platform: Platform) -> String {
    match find(value) {
        Some(action) => match platform.effective() {
            Platform::Macos => action.macos.to_string(),
            _ => action.linux.to_string(),
        },
        None => value.trim().to_string(),
    }
}

/// Modificadores propios de la otra plataforma, usados para advertir
/// cuando un perfil de Linux usa `cmd` o uno de macOS usa `ctrl` para atajos.
pub fn foreign_modifier(raw: &str, platform: Platform) -> Option<&'static str> {
    let parts: Vec<&str> = raw.split(['-', ',']).collect();
    match platform {
        Platform::Linux if parts.iter().any(|p| matches!(*p, "cmd" | "rcmd")) => Some("cmd"),
        Platform::Macos if parts.iter().any(|p| matches!(*p, "win" | "rwin")) => Some("win"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_per_platform() {
        assert_eq!(resolve("guardar", Platform::Linux), "ctrl-s");
        assert_eq!(resolve("save", Platform::Macos), "cmd-s");
        assert_eq!(resolve("volume_down", Platform::Linux), "volumedown");
        assert_eq!(resolve("ctrl-alt-t", Platform::Linux), "ctrl-alt-t");
    }

    #[test]
    fn aliases_are_unique() {
        let mut names: Vec<&str> = ACTIONS.iter().flat_map(|a| std::iter::once(a.id).chain(a.aliases.iter().copied())).collect();
        let total = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), total);
    }
}
