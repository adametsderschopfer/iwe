use super::repository::Provider;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub directory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Project {
    pub name: String,
    pub directory: PathBuf,
    #[serde(default)]
    pub pinned: bool,
}

impl Project {
    pub(crate) fn from_directory(directory: PathBuf) -> Self {
        let name = directory
            .file_name()
            .unwrap_or(directory.as_os_str())
            .to_string_lossy()
            .into_owned();
        Self {
            name,
            directory,
            pinned: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Space {
    pub name: String,
    #[serde(default)]
    pub projects: Vec<Project>,
    #[serde(default)]
    pub deployments: Vec<super::deployment::Profile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Preferences {
    pub language: Option<String>,
    pub shortcuts: super::shortcuts::Shortcuts,
    pub appearance: Appearance,
    pub code_size: u16,
    pub show_git: bool,
    pub show_pipelines: bool,
    pub line_numbers: bool,
    pub collapse_unchanged: bool,
    pub diff_view: DiffView,
    pub file_panels: Vec<FilePanel>,
    pub file_panel_width: f32,
    pub disabled_modules: BTreeSet<String>,
    pub deployment_panel_width: f32,
}

impl Preferences {
    pub(crate) const DEFAULT_FILE_PANEL_WIDTH: f32 = 280.;
    pub(crate) const MIN_FILE_PANEL_WIDTH: f32 = 180.;
    pub(crate) const MAX_FILE_PANEL_WIDTH: f32 = 640.;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Appearance {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum FilePanel {
    Project,
    Changes,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum DiffView {
    Unified,
    #[default]
    SideBySide,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            language: None,
            shortcuts: super::shortcuts::Shortcuts::default(),
            appearance: Appearance::default(),
            code_size: 14,
            show_git: true,
            show_pipelines: false,
            line_numbers: true,
            collapse_unchanged: false,
            diff_view: DiffView::default(),
            file_panels: vec![FilePanel::Project, FilePanel::Changes],
            file_panel_width: Self::DEFAULT_FILE_PANEL_WIDTH,
            disabled_modules: BTreeSet::new(),
            deployment_panel_width: 300.,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(from = "StoredWorkspace")]
pub(crate) struct Workspace {
    pub spaces: Vec<Space>,
    pub selected_space: Option<usize>,
    pub selected_project: Option<usize>,
    pub preferences: Preferences,
    pub providers: BTreeMap<String, Provider>,
    pub deployment: super::deployment::Connections,
}

impl Workspace {
    pub(crate) fn project(&self) -> Option<&Project> {
        self.spaces
            .get(self.selected_space?)?
            .projects
            .get(self.selected_project?)
    }

    pub(crate) fn directory(&self) -> Option<&PathBuf> {
        self.project().map(|project| &project.directory)
    }

    pub(crate) fn normalize(&mut self) {
        self.preferences.shortcuts.normalize();
        let mut panels = Vec::new();
        for panel in self
            .preferences
            .file_panels
            .iter()
            .copied()
            .chain([FilePanel::Project, FilePanel::Changes])
        {
            if !panels.contains(&panel) {
                panels.push(panel);
            }
        }
        self.preferences.file_panels = panels;
        self.preferences.file_panel_width = if self.preferences.file_panel_width.is_finite() {
            self.preferences.file_panel_width.clamp(
                Preferences::MIN_FILE_PANEL_WIDTH,
                Preferences::MAX_FILE_PANEL_WIDTH,
            )
        } else {
            Preferences::DEFAULT_FILE_PANEL_WIDTH
        };
        self.preferences.code_size = self.preferences.code_size.clamp(11, 22);
        if self.preferences.show_git {
            self.preferences.show_pipelines = false;
        }
        if self.project().is_none() {
            let first = self
                .selected_space
                .filter(|&index| {
                    self.spaces
                        .get(index)
                        .is_some_and(|space| !space.projects.is_empty())
                })
                .or_else(|| {
                    self.spaces
                        .iter()
                        .position(|space| !space.projects.is_empty())
                });
            self.selected_space = first;
            self.selected_project = first.map(|_| 0);
        }
    }

    pub(crate) fn create_space(
        &mut self,
        name: String,
        directories: Vec<PathBuf>,
    ) -> Option<usize> {
        let name = name.trim();
        if name.is_empty() {
            return None;
        }
        let index = self.spaces.len();
        self.spaces.push(Space {
            name: name.to_owned(),
            projects: Vec::new(),
            deployments: Vec::new(),
        });
        for directory in directories {
            self.add_project(index, directory);
        }
        if !self.spaces[index].projects.is_empty() {
            self.select_project(index, 0);
        }
        Some(index)
    }

    pub(crate) fn add_project(&mut self, space: usize, directory: PathBuf) -> bool {
        let Some(target) = self.spaces.get_mut(space) else {
            return false;
        };
        let project = target
            .projects
            .iter()
            .position(|project| project.directory == directory)
            .unwrap_or_else(|| {
                target.projects.push(Project::from_directory(directory));
                target.projects.len() - 1
            });
        self.select_project(space, project)
    }

    pub(crate) fn update_space(
        &mut self,
        index: usize,
        name: String,
        directories: Vec<PathBuf>,
    ) -> bool {
        let name = name.trim();
        if name.is_empty() {
            return false;
        }
        let selected = self.directory().cloned();
        let Some(space) = self.spaces.get_mut(index) else {
            return false;
        };
        let previous = std::mem::take(&mut space.projects);
        space.name = name.to_owned();
        for directory in directories {
            if space
                .projects
                .iter()
                .any(|project| project.directory == directory)
            {
                continue;
            }
            let project = previous
                .iter()
                .find(|project| project.directory == directory)
                .cloned()
                .unwrap_or_else(|| Project::from_directory(directory));
            space.projects.push(project);
        }
        if self.selected_space == Some(index) {
            self.selected_project = space
                .projects
                .iter()
                .position(|project| Some(&project.directory) == selected.as_ref());
        }
        self.normalize();
        true
    }

    pub(crate) fn remove_space(&mut self, index: usize) -> bool {
        if index >= self.spaces.len() {
            return false;
        }
        self.spaces.remove(index);
        match self.selected_space {
            Some(selected) if selected == index => {
                self.selected_space = None;
                self.selected_project = None;
            }
            Some(selected) if selected > index => self.selected_space = Some(selected - 1),
            _ => {}
        }
        self.normalize();
        true
    }

    pub(crate) fn move_space(&mut self, from: usize, to: usize) -> bool {
        if from >= self.spaces.len() || to >= self.spaces.len() || from == to {
            return false;
        }
        let space = self.spaces.remove(from);
        self.spaces.insert(to, space);
        if let Some(selected) = self.selected_space.as_mut() {
            *selected = moved_index(*selected, from, to);
        }
        true
    }

    pub(crate) fn move_project(&mut self, space: usize, from: usize, to: usize) -> bool {
        let Some(group) = self.spaces.get_mut(space) else {
            return false;
        };
        if from >= group.projects.len() || to >= group.projects.len() || from == to {
            return false;
        }
        let project = group.projects.remove(from);
        group.projects.insert(to, project);
        if self.selected_space == Some(space)
            && let Some(selected) = self.selected_project.as_mut()
        {
            if *selected == from {
                *selected = to;
            } else if from < *selected && *selected <= to {
                *selected -= 1;
            } else if to <= *selected && *selected < from {
                *selected += 1;
            }
        }
        true
    }

    pub(crate) fn select_project(&mut self, space: usize, project: usize) -> bool {
        if self
            .spaces
            .get(space)
            .and_then(|space| space.projects.get(project))
            .is_none()
        {
            return false;
        }
        self.selected_space = Some(space);
        self.selected_project = Some(project);
        true
    }

    pub(crate) fn select_directory(&mut self, directory: &Path) -> bool {
        if self
            .directory()
            .is_some_and(|selected| selected == directory)
        {
            return true;
        }
        let selected = self.spaces.iter().enumerate().find_map(|(space, value)| {
            value
                .projects
                .iter()
                .position(|project| project.directory == directory)
                .map(|project| (space, project))
        });
        selected.is_some_and(|(space, project)| self.select_project(space, project))
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct StoredWorkspace {
    spaces: Option<Vec<Space>>,
    projects: Vec<LegacyProject>,
    selected_space: Option<usize>,
    selected_project: Option<usize>,
    selected_directory: usize,
    preferences: Preferences,
    providers: BTreeMap<String, Provider>,
    deployment: super::deployment::Connections,
}

#[derive(Deserialize)]
struct LegacyProject {
    name: String,
    directories: Vec<PathBuf>,
}

impl From<StoredWorkspace> for Workspace {
    fn from(stored: StoredWorkspace) -> Self {
        let (spaces, selected_space, selected_project) = if let Some(spaces) = stored.spaces {
            (spaces, stored.selected_space, stored.selected_project)
        } else {
            let spaces = stored
                .projects
                .into_iter()
                .map(|project| Space {
                    name: project.name,
                    deployments: Vec::new(),
                    projects: project
                        .directories
                        .into_iter()
                        .map(Project::from_directory)
                        .collect(),
                })
                .collect();
            (
                spaces,
                stored.selected_project,
                stored.selected_project.map(|_| stored.selected_directory),
            )
        };
        Self {
            spaces,
            selected_space,
            selected_project,
            preferences: stored.preferences,
            providers: stored.providers,
            deployment: stored.deployment,
        }
    }
}

#[cfg(test)]
mod tests;

pub(crate) fn moved_index(index: usize, from: usize, to: usize) -> usize {
    if index == from {
        to
    } else if from < index && index <= to {
        index - 1
    } else if to <= index && index < from {
        index + 1
    } else {
        index
    }
}
