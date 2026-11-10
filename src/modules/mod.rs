//! Register bundled modules and expose their explicit integration boundaries.

mod catalog;
pub(crate) mod deployment;
pub(crate) mod terminal;

pub(crate) use catalog::{MODULES, ModuleId, Origin};
