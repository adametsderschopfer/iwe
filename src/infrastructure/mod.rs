//! Isolate operating-system access from the interface.

pub(crate) mod auth;
mod credentials;
pub(crate) mod git;
pub(crate) mod graphics;
mod http;
pub(crate) mod locale;
pub(crate) mod pipelines;
pub(crate) mod process;
pub(crate) mod project_index;
pub(crate) mod repository;
pub(crate) mod updates;
pub(crate) mod window_effect;
pub(crate) mod workspace;
