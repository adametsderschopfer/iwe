use super::{App, Message as AppMessage, Navigation};
use crate::infrastructure::workspace;
use iced::{Task, widget::operation};
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub(crate) struct Draft {
    pub editing: Option<usize>,
    pub name: String,
    pub directories: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    New,
    Edit(usize),
    NameChanged(String),
    PickProject(Option<usize>),
    ProjectsPicked(u64, Option<usize>, Vec<PathBuf>),
    RemoveProject(usize),
    MoveProject(Option<usize>, usize, usize),
    ScrollProjects(Option<usize>, f32),
    Create,
    Cancel,
}

impl App {
    pub(super) fn open_space(&mut self, directories: Vec<PathBuf>) -> Task<AppMessage> {
        self.space_draft = Some(Draft {
            editing: None,
            name: String::new(),
            directories,
        });
        self.space_request += 1;
        operation::focus("space-name")
    }

    pub(super) fn update_space(&mut self, message: Message) -> Task<AppMessage> {
        if self.git_actions.dialog.is_some() {
            return Task::none();
        }
        match message {
            Message::New if self.loaded && !self.picking && self.auth_dialog.is_none() => {
                return self.open_space(Vec::new());
            }
            Message::Edit(index) if self.loaded && !self.picking && self.auth_dialog.is_none() => {
                let Some(space) = self.workspace.spaces.get(index) else {
                    return Task::none();
                };
                self.space_draft = Some(Draft {
                    editing: Some(index),
                    name: space.name.clone(),
                    directories: space
                        .projects
                        .iter()
                        .map(|project| project.directory.clone())
                        .collect(),
                });
                self.space_request += 1;
                return operation::focus("space-name");
            }
            Message::NameChanged(name) => {
                if let Some(draft) = self.space_draft.as_mut() {
                    draft.name = name;
                }
            }
            Message::PickProject(space) if self.loaded && !self.picking => {
                if space.is_some_and(|index| index >= self.workspace.spaces.len())
                    || (space.is_none() && self.space_draft.is_none())
                {
                    return Task::none();
                }
                self.picking = true;
                self.space_request += 1;
                let request = self.space_request;
                return Task::perform(workspace::choose_directories(), move |directories| {
                    AppMessage::Space(Message::ProjectsPicked(request, space, directories))
                });
            }
            Message::ProjectsPicked(request, space, directories)
                if request == self.space_request =>
            {
                self.picking = false;
                if let Some(space) = space {
                    let mut added = false;
                    for directory in directories {
                        added |= self.workspace.add_project(space, directory);
                    }
                    if added {
                        self.navigation = Navigation::Projects;
                        return Task::batch([self.load_directory(), self.save()]);
                    }
                } else if let Some(draft) = self.space_draft.as_mut() {
                    for directory in directories {
                        if !draft.directories.contains(&directory) {
                            draft.directories.push(directory);
                        }
                    }
                }
            }
            Message::MoveProject(space, from, to)
                if self.loaded && !self.picking && self.auth_dialog.is_none() =>
            {
                if let Some(space) = space {
                    if self.space_draft.is_none() && self.workspace.move_project(space, from, to) {
                        return self.save();
                    }
                } else if let Some(draft) = self.space_draft.as_mut()
                    && from < draft.directories.len()
                    && to < draft.directories.len()
                    && from != to
                {
                    let directory = draft.directories.remove(from);
                    draft.directories.insert(to, directory);
                }
            }
            Message::ScrollProjects(space, offset) if !self.picking => {
                let id = if space.is_some() && self.space_draft.is_none() {
                    "project-browser"
                } else if space.is_none() && self.space_draft.is_some() {
                    "space-project-list"
                } else {
                    return Task::none();
                };
                return operation::scroll_by(id, operation::AbsoluteOffset { x: 0., y: offset });
            }
            Message::RemoveProject(index) if !self.picking => {
                if let Some(draft) = self.space_draft.as_mut()
                    && index < draft.directories.len()
                {
                    draft.directories.remove(index);
                }
            }
            Message::Create if self.loaded && !self.picking => {
                if self
                    .space_draft
                    .as_ref()
                    .is_none_or(|draft| draft.name.trim().is_empty())
                {
                    return Task::none();
                }
                let previous = self.workspace.directory().cloned();
                if let Some(draft) = self.space_draft.take() {
                    if let Some(index) = draft.editing {
                        self.workspace
                            .update_space(index, draft.name, draft.directories);
                    } else {
                        self.workspace.create_space(draft.name, draft.directories);
                        self.navigation = Navigation::Projects;
                    }
                }
                let load = if self.workspace.directory() != previous.as_ref() {
                    self.load_directory()
                } else {
                    Task::none()
                };
                return Task::batch([load, self.save()]);
            }
            Message::Cancel if self.space_draft.is_some() => {
                self.space_draft = None;
                self.space_request += 1;
                self.picking = false;
            }
            _ => {}
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests;
