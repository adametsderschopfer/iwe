use super::{App, AppMessage, allowed, git};
use crate::i18n::FluentArgs;
use crate::i18n::Key;
use crate::modules::git::actions::bulk::{Action, Dialog, Event, Outcome, Target};
use iced::Task;

impl App {
    fn selected_branch_targets(&self) -> Vec<Target> {
        let Some(repos) = &self.repository else {
            return Vec::new();
        };
        self.modules
            .git
            .branch_selection
            .selected
            .iter()
            .filter_map(|key| {
                let repo = repos.get(&key.root)?;
                let branch = repo
                    .branches
                    .iter()
                    .find(|branch| branch.reference == key.reference)?
                    .clone();
                let remote = repo
                    .remotes
                    .iter()
                    .find(|remote| remote.as_str() == "origin")
                    .or(repo.remotes.first())
                    .cloned()
                    .unwrap_or_default();
                Some(Target {
                    key: key.clone(),
                    branch,
                    remote,
                    error: None,
                })
            })
            .collect()
    }

    pub(crate) fn bulk_allowed(&self, action: Action) -> bool {
        let targets = self.selected_branch_targets();
        !self.modules.git.git_actions.pending
            && !targets.is_empty()
            && targets.len() == self.modules.git.branch_selection.selected.len()
            && targets.iter().all(|target| {
                allowed(action.single(), &target.branch)
                    && (action != Action::Push || !target.remote.is_empty())
            })
    }

    pub(in crate::app) fn update_bulk_git(&mut self, event: Event) -> Task<AppMessage> {
        match event {
            Event::Finished(request, generation, outcomes)
                if self.modules.git.git_actions.pending
                    && request == self.modules.git.git_actions.request =>
            {
                self.modules.git.git_actions.pending = false;
                let dialog = self.modules.git.git_actions.bulk.dialog.take();
                if generation != self.generation {
                    return Task::none();
                }
                let mut success = 0;
                let mut failures = Vec::new();
                for outcome in outcomes {
                    match outcome.result {
                        Ok(()) => success += 1,
                        Err(error) => {
                            let mut target = outcome.target;
                            target.error = Some(error);
                            failures.push(target);
                        }
                    }
                }
                let mut args = FluentArgs::new();
                args.set("success", success);
                args.set("failed", failures.len());
                let summary = Some(self.i18n.format(Key::GitBulkResult, &args));
                if failures.is_empty() {
                    self.notifications.success(Key::GitBulkDone, summary);
                } else {
                    self.notifications.error(Key::GitBulkFailed, summary);
                    if let Some(mut dialog) = dialog {
                        dialog.targets = failures;
                        self.modules.git.git_actions.bulk.dialog = Some(dialog);
                    }
                }
                return self.load_directory();
            }
            _ if self.modules.git.git_actions.pending
                || self.space_draft.is_some()
                || self.auth_dialog.is_some()
                || self.line_navigation.is_some() => {}
            Event::Choose(action)
                if self.bulk_allowed(action) && !self.modules.git.git_actions.has_dialog() =>
            {
                let targets = self.selected_branch_targets();
                self.modules.git.git_actions.menu = None;
                self.modules.git.git_actions.bulk.menu = None;
                self.modules.git.git_actions.bulk.dialog = Some(Dialog { action, targets });
            }
            Event::Remote(root, remote) => {
                if self
                    .repository
                    .as_ref()
                    .and_then(|repos| repos.get(&root))
                    .is_some_and(|repo| repo.remotes.contains(&remote))
                    && let Some(dialog) = &mut self.modules.git.git_actions.bulk.dialog
                {
                    for target in &mut dialog.targets {
                        if target.key.root == root {
                            target.remote = remote.clone();
                            target.error = None;
                        }
                    }
                }
            }
            Event::Submit => {
                let Some(dialog) = &mut self.modules.git.git_actions.bulk.dialog else {
                    return Task::none();
                };
                if dialog.targets.is_empty()
                    || dialog.targets.iter().any(|target| {
                        !allowed(dialog.action.single(), &target.branch)
                            || (dialog.action == Action::Push && target.remote.is_empty())
                    })
                {
                    return Task::none();
                }
                for target in &mut dialog.targets {
                    target.error = None;
                }
                let action = dialog.action;
                let targets = dialog.targets.clone();
                self.modules.git.git_actions.pending = true;
                self.modules.git.git_actions.request += 1;
                let (request, generation) = (self.modules.git.git_actions.request, self.generation);
                return Task::perform(
                    async move {
                        let mut outcomes = Vec::new();
                        for target in targets {
                            let result =
                                git::execute(target.key.root.clone(), target.operation(action))
                                    .await;
                            outcomes.push(Outcome { target, result });
                        }
                        outcomes
                    },
                    move |outcomes| {
                        AppMessage::Git(super::Message::Bulk(Event::Finished(
                            request, generation, outcomes,
                        )))
                    },
                );
            }
            _ => {}
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests;
