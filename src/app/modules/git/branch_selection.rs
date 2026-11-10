use crate::app::{App, Message};
pub(crate) use crate::modules::git::selection::*;
use iced::{Task, keyboard};

impl App {
    pub(in crate::app) fn update_branch_selection(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Modifiers(id, modifiers) if self.window_id == Some(id) => {
                self.modules.git.branch_selection.modifiers = modifiers
            }
            Event::Unfocused(id) if self.window_id == Some(id) => {
                self.modules.git.branch_selection.modifiers = keyboard::Modifiers::empty();
                self.modules.git.branch_selection.selected.clear();
                self.modules.git.git_actions.bulk.menu = None;
            }
            Event::Clear => {
                self.modules.git.branch_selection.selected.clear();
                self.modules.git.git_actions.bulk.menu = None;
            }
            _ if self.modules.git.git_actions.pending
                || self.modules.git.git_actions.has_dialog()
                || self.space_draft.is_some()
                || self.auth_dialog.is_some()
                || self.line_navigation.is_some() => {}
            Event::Toggle(target) => {
                if let Some(repo) = &self.repository {
                    let keys = target.keys(repo);
                    if self.modules.git.branch_selection.covers(&keys) {
                        for key in keys {
                            self.modules.git.branch_selection.selected.remove(&key);
                        }
                    } else {
                        self.modules.git.branch_selection.selected.extend(keys);
                    }
                    self.modules.git.git_actions.menu = None;
                    self.modules.git.git_actions.bulk.menu = None;
                }
            }
            Event::Menu(target, point) => {
                let matches = self.repository.as_ref().is_some_and(|repo| {
                    target
                        .keys(repo)
                        .iter()
                        .any(|key| self.modules.git.branch_selection.selected.contains(key))
                });
                self.modules.git.git_actions.menu = None;
                self.modules.git.git_actions.bulk.menu = matches.then_some(point);
                if !matches {
                    self.modules.git.branch_selection.selected.clear();
                }
            }
            _ => {}
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests;
