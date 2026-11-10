//! Keep screen composition separate from application state.
pub(crate) mod editor;

pub(crate) mod activity;
pub(crate) mod branch_selection;
pub(crate) mod branding;
pub(crate) mod changed_tree;
pub(crate) mod code;
pub(crate) mod context_menu;
pub(crate) mod ellipsis;
pub(crate) mod file_icons;
pub(crate) mod graph;
pub(crate) mod icons;
pub(crate) mod modules;
pub(crate) mod notifications;
pub(crate) mod reorder;
pub(crate) mod resize;
pub(crate) mod screens;
pub(crate) mod shortcuts;
pub(crate) mod tab_label;
pub(crate) mod theme;
pub(crate) mod tooltip;

pub(crate) mod updates;
#[cfg(target_os = "windows")]
pub(crate) mod window_chrome;

pub(crate) mod modal;
pub(crate) mod presentation;
