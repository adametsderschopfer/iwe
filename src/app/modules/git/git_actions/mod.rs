use crate::app::{App, Message as AppMessage};
use crate::domain::git::Operation;
use crate::i18n::Key;
pub(crate) use crate::modules::git::actions::*;
use crate::modules::git::transport as git;
use iced::Task;
use std::path::PathBuf;
mod bulk;

impl App {
    pub(in crate::app) fn update_git(&mut self, message: Message) -> Task<AppMessage> {
        match message {
            Message::Bulk(event) => return self.update_bulk_git(event),
            Message::Fetched(request, generation, results)
                if self.modules.git.git_actions.pending
                    && request == self.modules.git.git_actions.request =>
            {
                self.modules.git.git_actions.pending = false;
                if generation != self.generation {
                    return Task::none();
                }
                for (root, result) in results {
                    let context = root
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned());
                    match result {
                        Ok(()) => self.notifications.success(Key::GitFetchDone, context),
                        Err(error) => self.notifications.error(error_key(error), context),
                    }
                }
                return self.load_directory();
            }
            Message::Finished(request, root, result)
                if self.modules.git.git_actions.pending
                    && request == self.modules.git.git_actions.request =>
            {
                self.modules.git.git_actions.pending = false;
                let completion = self.modules.git.git_actions.completion.take();
                if self
                    .repository
                    .as_ref()
                    .is_none_or(|repos| repos.get(&root).is_none())
                {
                    self.modules.git.git_actions.dialog = None;
                    return Task::none();
                }
                match result {
                    Ok(()) => {
                        if let Some((message, context)) = completion {
                            self.notifications.success(message, context);
                        }
                        self.modules.git.git_actions.dialog = None;
                        return self.load_directory();
                    }
                    Err(error) => {
                        self.notifications.error(
                            error_key(error),
                            completion.and_then(|(_, context)| context),
                        );
                        if let Some(dialog) = &mut self.modules.git.git_actions.dialog {
                            dialog.error = Some(error);
                        }
                        return Task::batch([self.load_repository(), self.refresh_changes()]);
                    }
                }
            }
            Message::Close => {
                self.modules.git.branch_selection.selected.clear();
                self.modules.git.git_actions.menu = None;
                self.modules.git.git_actions.dialog = None;
                self.modules.git.git_actions.bulk.menu = None;
                self.modules.git.git_actions.bulk.dialog = None;
            }
            _ if self.modules.git.git_actions.pending
                || self.space_draft.is_some()
                || self.project_add.is_some()
                || self.auth_dialog.is_some() => {}
            Message::Menu(key, position) => {
                if self.modules.git.branch_selection.selected.contains(&key) {
                    self.modules.git.git_actions.menu = None;
                    self.modules.git.git_actions.bulk.menu = Some(position);
                    return Task::none();
                }
                self.modules.git.branch_selection.selected.clear();
                self.modules.git.git_actions.bulk.menu = None;
                if let Some(branch) = self
                    .repository
                    .as_ref()
                    .and_then(|repos| repos.get(&key.root))
                    .and_then(|repo| {
                        repo.branches
                            .iter()
                            .find(|branch| branch.reference == key.reference)
                    })
                    .cloned()
                {
                    self.project_menu = None;
                    self.space_menu = None;
                    self.modules.git.git_actions.menu = Some((key, branch, position));
                }
            }
            Message::Fetch => return self.fetch_repositories(),
            Message::Choose(action, key) => {
                self.modules.git.git_actions.menu = None;
                let Some(repo) = self
                    .repository
                    .as_ref()
                    .and_then(|repos| repos.get(&key.root))
                else {
                    return Task::none();
                };
                let Some(branch) = repo
                    .branches
                    .iter()
                    .find(|branch| branch.reference == key.reference)
                    .cloned()
                else {
                    return Task::none();
                };
                if !allowed(action, &branch) {
                    return Task::none();
                }
                match action {
                    Action::Checkout => {
                        return self.run_git(key.root, Operation::Checkout(key.reference));
                    }
                    Action::Update => {
                        return self.run_git(key.root, Operation::Update(key.reference));
                    }
                    _ => {
                        self.modules.git.git_actions.dialog = Some(Dialog {
                            root: key.root,
                            name: if action == Action::Create {
                                String::new()
                            } else {
                                branch.name.clone()
                            },
                            remote: repo
                                .remotes
                                .iter()
                                .find(|name| name.as_str() == "origin")
                                .or(repo.remotes.first())
                                .cloned()
                                .unwrap_or_default(),
                            action,
                            branch,
                            checkout: true,
                            error: None,
                        });
                    }
                }
            }
            Message::Name(value) => {
                if let Some(dialog) = &mut self.modules.git.git_actions.dialog {
                    dialog.name = value;
                    dialog.error = None;
                }
            }
            Message::Remote(value) => {
                if let Some(dialog) = &mut self.modules.git.git_actions.dialog {
                    dialog.remote = value;
                }
            }
            Message::Checkout(value) => {
                if let Some(dialog) = &mut self.modules.git.git_actions.dialog {
                    dialog.checkout = value;
                }
            }
            Message::Submit => {
                let Some(dialog) = &self.modules.git.git_actions.dialog else {
                    return Task::none();
                };
                let source = dialog.branch.reference.clone();
                let name = dialog.name.trim().to_owned();
                let operation = match dialog.action {
                    Action::Create => Operation::Create {
                        source,
                        name,
                        checkout: dialog.checkout,
                    },
                    Action::Rename => Operation::Rename { source, name },
                    Action::Push => Operation::Push {
                        source,
                        name,
                        remote: dialog.remote.clone(),
                    },
                    Action::Delete => Operation::Delete(source),
                    _ => return Task::none(),
                };
                return self.run_git(dialog.root.clone(), operation);
            }
            _ => {}
        }
        Task::none()
    }

    fn run_git(&mut self, root: PathBuf, operation: Operation) -> Task<AppMessage> {
        let Some(repo) = self.repository.as_ref().and_then(|repos| repos.get(&root)) else {
            return Task::none();
        };
        if self.modules.git.git_actions.pending {
            return Task::none();
        }
        let branch_name = |reference: &str| {
            let name = repo
                .branches
                .iter()
                .find(|branch| branch.reference == reference)
                .map_or_else(|| reference.to_owned(), |branch| branch.name.clone());
            Some(name)
        };
        self.modules.git.git_actions.completion = Some(match &operation {
            Operation::Fetch => (
                Key::GitFetchDone,
                self.workspace.project().map(|project| project.name.clone()),
            ),
            Operation::Checkout(reference) => (Key::GitCheckoutDone, branch_name(reference)),
            Operation::Update(reference) => (Key::GitUpdateDone, branch_name(reference)),
            Operation::Delete(reference) => (Key::GitDeleteDone, branch_name(reference)),
            Operation::Create { name, .. } => (Key::GitCreateDone, Some(name.clone())),
            Operation::Rename { name, .. } => (Key::GitRenameDone, Some(name.clone())),
            Operation::Push { name, .. } => (Key::GitPushDone, Some(name.clone())),
        });
        if self
            .repository
            .as_ref()
            .is_some_and(|repos| repos.repositories.len() > 1)
            && let Some((_, Some(context))) = &mut self.modules.git.git_actions.completion
        {
            *context = format!(
                "{} · {context}",
                root.file_name().unwrap_or_default().to_string_lossy()
            );
        }
        self.modules.git.git_actions.pending = true;
        self.modules.git.git_actions.request += 1;
        let request = self.modules.git.git_actions.request;
        Task::perform(git::execute(root.clone(), operation), move |result| {
            AppMessage::Git(Message::Finished(request, root.clone(), result))
        })
    }

    fn fetch_repositories(&mut self) -> Task<AppMessage> {
        let Some(repos) = &self.repository else {
            return Task::none();
        };
        if repos.repositories.len() == 1 {
            return self.run_git(repos.active_root.clone(), Operation::Fetch);
        }
        let roots: Vec<_> = repos
            .repositories
            .iter()
            .map(|repo| repo.root.clone())
            .collect();
        self.modules.git.git_actions.pending = true;
        self.modules.git.git_actions.request += 1;
        let (request, generation) = (self.modules.git.git_actions.request, self.generation);
        Task::perform(
            async move {
                let mut results = Vec::new();
                for root in roots {
                    let result = git::execute(root.clone(), Operation::Fetch).await;
                    results.push((root, result));
                }
                results
            },
            move |results| AppMessage::Git(Message::Fetched(request, generation, results)),
        )
    }
}

#[cfg(test)]
mod tests;
