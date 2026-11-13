//! Isolate operating-system access from the interface.
pub(crate) mod editor;

pub(crate) mod auth;
mod credentials;
pub(crate) mod graphics;
pub(crate) mod http;
pub(crate) mod locale;
pub(crate) mod modules;
#[cfg(target_os = "macos")]
pub(crate) mod native_menu;
pub(crate) use crate::modules::pipelines::transport as pipelines;
pub(crate) mod process;
pub(crate) mod project_clone;
pub(crate) mod project_index;
pub(crate) mod repository;
pub(crate) mod secret_files;
pub(crate) mod updates;
pub(crate) mod window_effect;
pub(crate) mod workspace;
