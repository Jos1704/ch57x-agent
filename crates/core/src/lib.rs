//! Núcleo de MacroPad Agent: perfiles, detección USB y programación del
//! macro pad `1189:8890` mediante `ch57x-keyboard-tool`.

pub mod actions;
pub mod apply;
pub mod device;
pub mod diagnose;
pub mod paths;
pub mod platform;
pub mod profile;
pub mod service;
pub mod settings;
pub mod state;
pub mod tool;

pub use apply::{apply_profile, Trigger};
pub use platform::Platform;
pub use profile::Profile;
