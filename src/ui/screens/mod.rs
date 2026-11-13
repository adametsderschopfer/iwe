mod auth;
mod diff;
mod file_menu;
pub(crate) mod git;
mod git_actions;
mod header_repositories;
mod line_navigation;
#[cfg(target_os = "windows")]
mod menu_bar;
mod modal;
pub(crate) mod pipelines;
mod project_add;
mod search_everywhere;
pub(crate) mod settings;
mod space_form;
mod tab_menu;
mod unified;
mod working_changes;
pub(crate) mod workspace;
