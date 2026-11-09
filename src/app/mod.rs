//! Coordinate asynchronous reads, workspace state, and the native window.

pub(crate) mod auth;
pub(crate) mod changes;
pub(crate) mod files;
pub(crate) mod git_actions;
pub(crate) mod jobs;
pub(crate) mod notifications;
mod pipeline_cache;
mod repositories;
pub(crate) mod spaces;
pub(crate) mod tabs;
pub(crate) mod updates;

use crate::domain::{
    Failure,
    repository::{
        ChangedFile, Pipeline, Provider,
        project::{BranchKey, CommitKey, ProjectHistoryPage as HistoryPage, RepositorySet},
    },
    workspace::{Appearance, FileEntry, FilePanel, Workspace},
};
use crate::i18n::{I18n, Key, Language};
use crate::infrastructure::{pipelines, window_effect, workspace};
use crate::ui::{code::Document, screens, theme};
use iced::{
    Element, Program, Size, Subscription, Task, Theme, keyboard, widget::pane_grid, window,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    rc::Rc,
};
use tokio_util::sync::CancellationToken;

pub(crate) fn run(i18n: Rc<I18n>) -> iced::Result {
    application(i18n).run()
}

fn application(
    i18n: Rc<I18n>,
) -> iced::Application<impl Program<State = App, Message = Message, Theme = Theme>> {
    iced::application(
        move || {
            (
                App::new(Rc::clone(&i18n)),
                Task::batch([
                    Task::perform(workspace::load(), Message::Loaded),
                    Task::done(Message::Update(updates::Event::Check)),
                ]),
            )
        },
        App::update,
        App::view,
    )
    .title(|state: &App| state.i18n.text(Key::AppTitle))
    .window(window::Settings {
        size: Size::new(1440.0, 920.0),
        min_size: Some(Size::new(1080.0, 720.0)),
        position: window::Position::Centered,
        icon: crate::ui::branding::window_icon(),
        transparent: cfg!(target_os = "macos"),
        decorations: !cfg!(target_os = "windows"),
        #[cfg(target_os = "macos")]
        platform_specific: window::settings::PlatformSpecific {
            title_hidden: true,
            titlebar_transparent: true,
            fullsize_content_view: true,
        },
        ..window::Settings::default()
    })
    .subscription(|app| {
        Subscription::batch([
            app.change_subscription(),
            iced::time::every(std::time::Duration::from_secs(6 * 60 * 60))
                .map(|_| Message::Update(updates::Event::Check)),
            app.pipeline_subscription(),
            app.jobs_subscription(),
            app.notifications.subscription().map(Message::Notification),
            window::open_events().map(Message::WindowOpened),
            window_state_subscription(),
            iced::event::listen_with(|event, _, _| shortcut(event)),
        ])
    })
    .theme(App::theme)
    .default_font(theme::font())
    .style(|state, theme| theme::application_style(state.glass_ready, theme))
}

fn window_state_subscription() -> Subscription<Message> {
    #[cfg(target_os = "windows")]
    {
        window::resize_events().map(|(id, _)| Message::WindowResized(id))
    }
    #[cfg(not(target_os = "windows"))]
    {
        Subscription::none()
    }
}

fn shortcut(event: iced::Event) -> Option<Message> {
    let iced::Event::Keyboard(keyboard::Event::KeyPressed {
        key,
        physical_key,
        modifiers,
        repeat: false,
        ..
    }) = event
    else {
        return None;
    };
    if key == keyboard::Key::Named(keyboard::key::Named::Escape) {
        return Some(Message::DismissDialog);
    }
    let close = physical_key == keyboard::key::Physical::Code(keyboard::key::Code::KeyW)
        || matches!(&key, keyboard::Key::Character(character) if character.eq_ignore_ascii_case("w"));
    let reopen = physical_key == keyboard::key::Physical::Code(keyboard::key::Code::KeyT)
        || matches!(&key, keyboard::Key::Character(character) if character.eq_ignore_ascii_case("t"));
    if close && modifiers == keyboard::Modifiers::COMMAND {
        Some(Message::CloseActiveTab)
    } else if reopen && modifiers == keyboard::Modifiers::COMMAND | keyboard::Modifiers::SHIFT {
        Some(Message::ReopenClosedTab)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Navigation {
    Projects,
    Files,
    Settings,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum SettingsTab {
    #[default]
    General,
    Code,
    Panels,
    Shortcuts,
    About,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Panel {
    Workspace,
    Git,
    Pipelines,
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum GitPanel {
    Branches,
    Commits,
    Files,
}

fn git_panes() -> pane_grid::State<GitPanel> {
    let (mut panes, branches) = pane_grid::State::new(GitPanel::Branches);
    let (_, left) = panes
        .split(pane_grid::Axis::Vertical, branches, GitPanel::Commits)
        .unwrap();
    panes.resize(left, 0.22);
    panes
}
#[derive(Debug, Clone, Default)]
pub(crate) enum PipelineState {
    #[default]
    Idle,
    Loading,
    Ready(Vec<Pipeline>),
    Failed(Failure),
}

pub(crate) struct App {
    pub i18n: Rc<I18n>,
    pub workspace: Workspace,
    pub navigation: Navigation,
    pub settings_tab: SettingsTab,
    pub panes: pane_grid::State<Panel>,
    pub git_panes: pane_grid::State<GitPanel>,
    pub repository: Option<RepositorySet>,
    pub repository_error: Option<Failure>,
    pub loading: bool,
    pub history_loading: bool,
    pub history_error: Option<Failure>,
    pub history_scroll_y: f32,
    pub tree: BTreeMap<PathBuf, Vec<FileEntry>>,
    pub expanded: BTreeSet<PathBuf>,
    pub files: files::State,
    pub file_panel: FilePanel,
    pub file_panel_visible: bool,
    pub git_actions: git_actions::State,
    pub project_menu: Option<(usize, PathBuf, iced::Point)>,
    pub collapsed_worktree_folders: BTreeSet<String>,
    pub changes: changes::State,
    pub tabs: tabs::State,
    pub branch: Option<BranchKey>,
    pub query: String,
    pub selected_commit: Option<CommitKey>,
    pub changed_files: Vec<ChangedFile>,
    pub collapsed_commit_folders: BTreeSet<String>,
    pub expanded_branch_folders: BTreeSet<String>,
    pub collapsed_spaces: BTreeSet<usize>,
    pub changes_loading: bool,
    pub changes_error: Option<Failure>,
    pub pipelines: PipelineState,
    pub jobs: jobs::State,
    pub auth_dialog: Option<auth::Dialog>,
    pub space_draft: Option<spaces::Draft>,
    pub account_username: Option<String>,
    pub disconnecting: bool,
    pub notifications: notifications::State,
    pub updates: updates::State,
    pub picking: bool,
    pub loaded: bool,
    window_id: Option<window::Id>,
    pub window_maximized: bool,
    glass_ready: bool,
    generation: u64,
    project_directory: Option<PathBuf>,
    repo_request: u64,
    file_request: u64,
    commit_request: u64,
    pipeline_request: u64,
    pipeline_refreshing: bool,
    pipeline_cache: pipeline_cache::Cache,
    pipeline_poll_blocked: bool,
    pipeline_reload_after_repository: bool,
    auth_service: crate::infrastructure::auth::Service,
    auth_cancel: Option<CancellationToken>,
    auth_request: u64,
    space_request: u64,
    saving: bool,
    save_pending: bool,
    save_blocked: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    WindowOpened(window::Id),
    Update(updates::Event),
    DragWindow,
    ToggleWindowMaximized,
    WindowMaximized(bool),
    #[cfg(target_os = "windows")]
    WindowResized(window::Id),
    #[cfg(target_os = "windows")]
    MinimizeWindow,
    #[cfg(target_os = "windows")]
    CloseWindow,
    #[cfg(target_os = "windows")]
    ResizeWindow(window::Direction),
    GlassConfigured(bool),
    Loaded(Result<(Workspace, Option<PathBuf>), Failure>),
    Saved(Result<(), Failure>),
    Navigate(Navigation),
    SelectSettingsTab(SettingsTab),
    Space(spaces::Message),
    Git(git_actions::Message),
    ProjectMenu(usize, PathBuf, iced::Point),
    TogglePinned,
    RemoveProjectFromSpace,
    CloseProjectMenu,
    DismissDialog,
    SelectProject(usize, usize),
    ToggleSpace(usize),
    Refresh,
    RepositoryLoaded(u64, u64, Result<RepositorySet, Failure>),
    HistoryLoaded(u64, u64, usize, Result<HistoryPage, Failure>),
    HistoryScrolled {
        offset: f32,
        remaining: f32,
    },
    LoadMoreHistory,
    ToggleFolder(PathBuf),
    SelectFilePanel(FilePanel),
    ToggleFilePanel(FilePanel),
    ResizeFilePanel(f32, bool),
    MoveFilePanel(usize, usize),
    ToggleWorktreeFolder(String),
    Files(files::Event),
    Worktree(changes::Event),
    OpenFile(PathBuf),
    CodeSelection(u64, crate::ui::code::selection::Action),
    ReadFilePart(u64, tabs::PageDirection),
    OpenCommitFile(usize),
    FileLoaded(u64, u64, Result<Document, Failure>),
    FilePrepared(u64, u64, tabs::Preparation, Result<Document, Failure>),
    SelectBranch(BranchKey),
    AllRepositories,
    Search(String),
    SelectCommit(CommitKey),
    CloseCommitFiles,
    ToggleCommitFolder(String),
    ToggleBranchFolder(String),
    ChangesLoaded(u64, u64, Result<Vec<ChangedFile>, Failure>),
    TogglePanel(Panel),
    Resize(pane_grid::ResizeEvent),
    ResizeGit(pane_grid::ResizeEvent),
    RefreshPipelines,
    PollPipelines,
    PipelinesLoaded(u64, u64, pipelines::Loaded),
    Jobs(jobs::Event),
    ChooseProvider(Provider),
    SignIn,
    Auth(auth::Message),
    OpenUrl(String),
    Opened(Result<(), Failure>),
    SetLanguage(String),
    SetAppearance(Appearance),
    SetCodeSize(u16),
    SetLineNumbers(bool),
    CopyPath,
    SelectTab(u64),
    CloseTab(u64),
    CloseActiveTab,
    ReopenClosedTab,
    ToggleUnchanged(u64),
    ToggleDiffView(u64),
    ExpandUnchanged(u64, std::ops::Range<usize>),
    CodeScrolled {
        id: u64,
        x: f32,
        y: f32,
    },
    DiffScrolled {
        id: u64,
        side: crate::ui::code::comparison::Side,
        x: f32,
        y: f32,
    },
    Notification(notifications::Event),
}

impl App {
    pub(crate) fn theme(&self) -> Theme {
        match self.workspace.preferences.appearance {
            Appearance::Dark => Theme::Dark,
            Appearance::Light => Theme::Light,
        }
    }

    fn new(i18n: Rc<I18n>) -> Self {
        let mut app = Self {
            i18n,
            workspace: Workspace::default(),
            navigation: Navigation::Projects,
            settings_tab: SettingsTab::default(),
            panes: pane_grid::State::new(Panel::Workspace).0,
            git_panes: git_panes(),
            repository: None,
            repository_error: None,
            loading: false,
            history_loading: false,
            history_error: None,
            history_scroll_y: 0.,
            tree: BTreeMap::new(),
            expanded: BTreeSet::new(),
            files: files::State::default(),
            file_panel: FilePanel::Project,
            file_panel_visible: true,
            git_actions: git_actions::State::default(),
            project_menu: None,
            collapsed_worktree_folders: BTreeSet::new(),
            changes: changes::State::default(),
            tabs: tabs::State::default(),
            branch: None,
            query: String::new(),
            selected_commit: None,
            changed_files: Vec::new(),
            collapsed_commit_folders: BTreeSet::new(),
            expanded_branch_folders: BTreeSet::from(["false".to_owned()]),
            collapsed_spaces: BTreeSet::new(),
            changes_loading: false,
            changes_error: None,
            pipelines: PipelineState::Idle,
            jobs: jobs::State::default(),
            auth_dialog: None,
            space_draft: None,
            account_username: None,
            disconnecting: false,
            notifications: notifications::State::default(),
            updates: if std::env::var_os("IWE_UPDATE_FAILED").as_deref() == Some("1".as_ref()) {
                updates::State::Failed(crate::infrastructure::updates::Error::Install)
            } else {
                updates::State::default()
            },
            picking: false,
            loaded: false,
            window_id: None,
            window_maximized: false,
            glass_ready: false,
            generation: 0,
            project_directory: None,
            repo_request: 0,
            file_request: 0,
            commit_request: 0,
            pipeline_request: 0,
            pipeline_refreshing: false,
            pipeline_cache: pipeline_cache::Cache::default(),
            pipeline_poll_blocked: false,
            pipeline_reload_after_repository: false,
            auth_service: crate::infrastructure::auth::Service::new(),
            auth_cancel: None,
            auth_request: 0,
            space_request: 0,
            saving: false,
            save_pending: false,
            save_blocked: false,
        };
        if matches!(app.updates, updates::State::Failed(_)) {
            app.notifications.error(Key::UpdateInstallFailed, None);
        }
        app
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        if matches!(
            self.updates,
            updates::State::Saving(_) | updates::State::Installing(_)
        ) && !matches!(
            message,
            Message::Saved(_) | Message::Update(updates::Event::Started(_))
        ) {
            return Task::none();
        }
        if (self.auth_dialog.is_some()
            || self.space_draft.is_some()
            || self.git_actions.dialog.is_some())
            && matches!(
                message,
                Message::Navigate(_)
                    | Message::SelectSettingsTab(_)
                    | Message::SelectProject(_, _)
                    | Message::RemoveProjectFromSpace
                    | Message::SelectFilePanel(_)
                    | Message::ToggleFilePanel(_)
                    | Message::ResizeFilePanel(_, _)
                    | Message::MoveFilePanel(_, _)
                    | Message::Refresh
                    | Message::SelectBranch(_)
                    | Message::AllRepositories
                    | Message::SelectCommit(_)
                    | Message::SelectTab(_)
                    | Message::CloseTab(_)
                    | Message::CloseActiveTab
                    | Message::ReopenClosedTab
                    | Message::Update(updates::Event::Download | updates::Event::Install)
                    | Message::ToggleUnchanged(_)
                    | Message::ToggleDiffView(_)
                    | Message::ExpandUnchanged(_, _)
                    | Message::Jobs(
                        jobs::Event::Select(_)
                            | jobs::Event::Back
                            | jobs::Event::Control(_, _)
                            | jobs::Event::Overview(jobs::overview::Event::Open(_, _, _))
                    )
            )
        {
            return Task::none();
        }
        match message {
            Message::Update(event) => return self.update_release(event),
            Message::Git(message) => return self.update_git(message),
            Message::ProjectMenu(space, path, point)
                if self.space_draft.is_none()
                    && self.auth_dialog.is_none()
                    && self.git_actions.dialog.is_none() =>
            {
                self.git_actions.menu = None;
                self.files.menu = None;
                self.project_menu = Some((space, path, point));
            }
            Message::CloseProjectMenu => self.project_menu = None,
            Message::RemoveProjectFromSpace => return self.remove_project_from_space(),
            Message::TogglePinned => {
                if let Some((space, path, _)) = self.project_menu.take()
                    && let Some(project) = self.workspace.spaces.get_mut(space).and_then(|group| {
                        group
                            .projects
                            .iter_mut()
                            .find(|project| project.directory == path)
                    })
                {
                    project.pinned = !project.pinned;
                    return self.save();
                }
            }
            Message::WindowOpened(id) => {
                self.window_id = Some(id);
                return Task::batch([
                    window::run(id, window_effect::apply).map(Message::GlassConfigured),
                    window::is_maximized(id).map(Message::WindowMaximized),
                ]);
            }
            Message::DragWindow => {
                return self.window_id.map_or_else(Task::none, window::drag);
            }
            Message::ToggleWindowMaximized => {
                return self.window_id.map_or_else(Task::none, |id| {
                    window::toggle_maximize::<Message>(id)
                        .chain(window::is_maximized(id).map(Message::WindowMaximized))
                });
            }
            Message::WindowMaximized(maximized) => self.window_maximized = maximized,
            #[cfg(target_os = "windows")]
            Message::WindowResized(id) if self.window_id == Some(id) => {
                return window::is_maximized(id).map(Message::WindowMaximized);
            }
            #[cfg(target_os = "windows")]
            Message::MinimizeWindow => {
                return self
                    .window_id
                    .map_or_else(Task::none, |id| window::minimize(id, true));
            }
            #[cfg(target_os = "windows")]
            Message::CloseWindow => {
                return self.window_id.map_or_else(Task::none, window::close);
            }
            #[cfg(target_os = "windows")]
            Message::ResizeWindow(direction) if !self.window_maximized => {
                return self
                    .window_id
                    .map_or_else(Task::none, |id| window::drag_resize(id, direction));
            }
            Message::GlassConfigured(ready) => self.glass_ready = ready,
            Message::Loaded(result) => {
                self.loaded = true;
                let mut dialog = Task::none();
                match result {
                    Ok((workspace, directory)) => {
                        self.workspace = workspace;
                        self.workspace.normalize();
                        self.file_panel = self.workspace.preferences.file_panels[0];
                        if let Some(directory) = directory {
                            dialog = self.open_space(vec![directory]);
                        }
                    }
                    Err(_) => {
                        self.notifications.error(Key::StorageLoadFailed, None);
                        self.save_blocked = true;
                    }
                }
                self.apply_language();
                return Task::batch([self.load_directory(), self.save(), dialog]);
            }
            Message::Saved(result) => {
                self.saving = false;
                if result.is_err() {
                    self.notifications.error(Key::StorageSaveFailed, None);
                }
                if result.is_err() && matches!(self.updates, updates::State::Saving(_)) {
                    return self.restart_after_save(false);
                }
                if self.save_pending {
                    self.save_pending = false;
                    return self.save();
                }
                if matches!(self.updates, updates::State::Saving(_)) {
                    return self.restart_after_save(true);
                }
            }
            Message::Navigate(navigation) => {
                self.navigation = navigation;
                self.jobs.overview.popup = None;
                if navigation != Navigation::Settings {
                    return self.refresh_pipeline_overviews(false);
                }
            }
            Message::SelectSettingsTab(tab) => self.settings_tab = tab,
            Message::Space(message) => return self.update_space(message),
            Message::DismissDialog => {
                if self.files.menu.take().is_some() { return Task::none(); }
                self.project_menu = None;
                if self.git_actions.menu.is_some() || self.git_actions.dialog.is_some() {
                    return self.update_git(git_actions::Message::Close);
                }
                if self.space_draft.is_some() {
                    return self.update_space(spaces::Message::Cancel);
                }
                if self.auth_dialog.is_none() && self.jobs.overview.popup.take().is_some() {
                    return Task::none();
                }
                return self.update_auth(auth::Message::Cancel);
            }
            Message::SelectProject(space, project) => {
                self.navigation = Navigation::Projects;
                if self.workspace.selected_space == Some(space)
                    && self.workspace.selected_project == Some(project)
                {
                    return Task::none();
                }
                if self.workspace.select_project(space, project) {
                    return Task::batch([self.load_directory(), self.save()]);
                }
            }
            Message::ToggleSpace(index) if index < self.workspace.spaces.len() => {
                if !self.collapsed_spaces.remove(&index) {
                    self.collapsed_spaces.insert(index);
                }
            }
            Message::Refresh => return self.load_directory(),
            Message::SelectFilePanel(panel) => {
                self.file_panel = panel;
                self.file_panel_visible = true;
            }
            Message::ToggleFilePanel(panel) => {
                self.file_panel_visible = !self.file_panel_visible || self.file_panel != panel;
                self.file_panel = panel;
            }
            Message::ResizeFilePanel(width, finished)
                if self.file_panel_visible && width.is_finite() =>
            {
                use crate::domain::workspace::Preferences;
                self.workspace.preferences.file_panel_width = width.clamp(
                    Preferences::MIN_FILE_PANEL_WIDTH,
                    Preferences::MAX_FILE_PANEL_WIDTH,
                );
                if finished {
                    return self.save();
                }
            }
            Message::MoveFilePanel(from, to) => {
                let panels = &mut self.workspace.preferences.file_panels;
                if from < panels.len() && to < panels.len() && from != to {
                    let panel = panels.remove(from);
                    panels.insert(to, panel);
                    self.file_panel = panels[0];
                    self.file_panel_visible = true;
                    return self.save();
                }
            }
            Message::ToggleWorktreeFolder(path) => {
                if !self.collapsed_worktree_folders.remove(&path) {
                    self.collapsed_worktree_folders.insert(path);
                }
            }
            Message::RepositoryLoaded(generation, request, result)
                if generation == self.generation && request == self.repo_request =>
            {
                self.loading = false;
                let reload_pipelines = std::mem::take(&mut self.pipeline_reload_after_repository);
                match result {
                    Ok(mut repo) => {
                        let multiple = repo.repositories.len() > 1;
                        for member in &mut repo.repositories {
                            if let Some(remote) = member.remote.as_mut()
                                && let Some(provider) = self.workspace.providers.get(&remote.host)
                            {
                                remote.provider = Some(*provider);
                            }
                            let known = self.repository.as_ref().is_some_and(|previous| {
                                previous.repositories.len() > 1
                                    && previous.get(&member.root).is_some()
                            });
                            if multiple && !known {
                                self.expanded_branch_folders
                                    .insert(format!("repo:{}", member.root.display()));
                                self.expanded_branch_folders
                                    .insert(format!("{}:false", member.root.display()));
                            }
                        }
                        let changed_remote = self
                            .active_repository()
                            .and_then(|repo| repo.remote.as_ref())
                            != repo.active().and_then(|repo| repo.remote.as_ref());
                        if changed_remote {
                            self.reset_jobs();
                            self.pipelines = PipelineState::Idle;
                            self.account_username = None;
                            self.pipeline_request += 1;
                            self.pipeline_refreshing = false;
                        }
                        if self.branch.as_ref().is_some_and(|key| {
                            repo.get(&key.root).is_none_or(|member| {
                                !member
                                    .branches
                                    .iter()
                                    .any(|branch| branch.reference == key.reference)
                            })
                        }) {
                            self.branch = None;
                        }
                        if self.branch.is_none() && repo.repositories.len() == 1 {
                            self.branch =
                                repo.active().and_then(|member| {
                                    member.branches.iter().find(|branch| branch.current).map(
                                        |branch| BranchKey {
                                            root: member.root.clone(),
                                            reference: branch.reference.clone(),
                                        },
                                    )
                                });
                        }
                        self.repository = Some(repo);
                        self.repository_error = None;
                        self.sync_panels();
                        let pipeline_task = if (self.workspace.preferences.show_pipelines
                            || self.workspace.preferences.show_git)
                            && (reload_pipelines || matches!(self.pipelines, PipelineState::Idle))
                        {
                            self.load_pipelines()
                        } else {
                            Task::none()
                        };
                        let history_task = if self.query.is_empty() {
                            Task::none()
                        } else {
                            self.load_more_history()
                        };
                        return Task::batch([pipeline_task, history_task]);
                    }
                    Err(error) => {
                        self.repository_error = Some(error);
                        if error == Failure::NotRepository {
                            self.reset_jobs();
                            self.repository = None;
                            self.pipelines = PipelineState::Idle;
                            self.account_username = None;
                            self.pipeline_request += 1;
                            self.pipeline_refreshing = false;
                        }
                        self.sync_panels();
                    }
                }
            }
            Message::HistoryLoaded(generation, request, offset, result)
                if generation == self.generation
                    && request == self.repo_request
                    && self.history_loading
                    && self
                        .repository
                        .as_ref()
                        .is_some_and(|repo| repo.commits.len() == offset) =>
            {
                self.history_loading = false;
                match result {
                    Ok(page) => {
                        if let Some(repo) = self.repository.as_mut() {
                            repo.commits.extend(page.commits);
                            repo.next_history = page.next;
                        }
                        self.history_error = None;
                        if !self.query.is_empty() {
                            return self.load_more_history();
                        }
                    }
                    Err(error) => self.history_error = Some(error),
                }
            }
            Message::HistoryScrolled { offset, remaining } => {
                self.history_scroll_y = offset;
                if remaining <= 480. && self.history_error.is_none() {
                    return self.load_more_history();
                }
            }
            Message::LoadMoreHistory => return self.load_more_history(),
            Message::ToggleFolder(path) => return self.toggle_folder(path),
            Message::Files(event) => return self.update_files(event),
            Message::Worktree(event) => return self.update_changes(event),
            Message::ReadFilePart(id, direction) => return self.read_file_part(id, direction),
            Message::CodeSelection(id, action) => {
                if self.preview().id == id
                    && let Some(preview) = self.preview_for_request(id)
                    && let Some(document) = &preview.document
                {
                    preview.selection.update(action, document);
                }
            }
            Message::OpenFile(path) => return self.open_file(path),
            Message::OpenCommitFile(index) => return self.open_commit_file(index),
            Message::FilePrepared(generation, request, stage, result)
                if generation == self.generation =>
            {
                return self.continue_preview(request, stage, result);
            }
            Message::FileLoaded(generation, request, result) if generation == self.generation => {
                return self.finish_preview(request, result);
            }
            Message::SelectBranch(reference) => {
                if self
                    .repository
                    .as_ref()
                    .and_then(|repos| repos.get(&reference.root))
                    .is_none_or(|repo| {
                        !repo
                            .branches
                            .iter()
                            .any(|branch| branch.reference == reference.reference)
                    })
                {
                    return Task::none();
                }
                self.branch = Some(reference);
                self.close_commit_files();
                return self.load_repository();
            }
            Message::AllRepositories => {
                self.branch = None;
                self.close_commit_files();
                return self.load_repository();
            }
            Message::Search(query) => {
                self.query = query;
                let scroll = self.reset_history_scroll();
                let history = if self.query.is_empty() {
                    Task::none()
                } else {
                    self.load_more_history()
                };
                return Task::batch([scroll, history]);
            }
            Message::ToggleCommitFolder(path) => {
                if !self.collapsed_commit_folders.remove(&path) {
                    self.collapsed_commit_folders.insert(path);
                }
            }
            Message::ToggleBranchFolder(path) => {
                if !self.expanded_branch_folders.remove(&path) {
                    self.expanded_branch_folders.insert(path);
                }
            }
            Message::SelectCommit(id) => return self.select_commit(id),
            Message::CloseCommitFiles => self.close_commit_files(),
            Message::ChangesLoaded(generation, request, result)
                if generation == self.generation && request == self.commit_request =>
            {
                self.changes_loading = false;
                match result {
                    Ok(files) => self.changed_files = files,
                    Err(error) => self.changes_error = Some(error),
                }
            }
            Message::TogglePanel(panel) => {
                self.jobs.overview.popup = None;
                match panel {
                    Panel::Git => {
                        self.workspace.preferences.show_git ^= true;
                        if self.workspace.preferences.show_git {
                            self.workspace.preferences.show_pipelines = false;
                        }
                    }
                    Panel::Pipelines => {
                        self.workspace.preferences.show_pipelines ^= true;
                        if self.workspace.preferences.show_pipelines {
                            self.workspace.preferences.show_git = false;
                        }
                    }
                    Panel::Workspace => {}
                }
                self.sync_panels();
                let pipelines = if (self.workspace.preferences.show_pipelines
                    || self.workspace.preferences.show_git)
                    && matches!(self.pipelines, PipelineState::Idle)
                {
                    self.load_pipelines()
                } else {
                    Task::none()
                };
                let jobs = if self.workspace.preferences.show_pipelines {
                    Task::batch([self.refresh_jobs(), self.refresh_pipeline_overviews(false)])
                } else {
                    Task::none()
                };
                return Task::batch([pipelines, jobs, self.save()]);
            }
            Message::Resize(event) => {
                self.jobs.overview.popup = None;
                self.panes
                    .resize(event.split, event.ratio.clamp(0.15, 0.85));
            }
            Message::ResizeGit(event) => self
                .git_panes
                .resize(event.split, event.ratio.clamp(0.12, 0.88)),
            Message::RefreshPipelines => return self.request_pipelines(true),
            Message::PollPipelines => return self.load_pipelines(),
            Message::Jobs(event) => return self.update_jobs(event),
            Message::PipelinesLoaded(generation, request, result)
                if generation == self.generation && request == self.pipeline_request =>
            {
                self.account_username = result.username;
                self.pipeline_refreshing = false;
                match result.runs {
                    Ok(runs) => {
                        if let Some(repo) = self.active_repository()
                            && let Some(remote) = &repo.remote
                        {
                            let root = repo.root.clone();
                            let remote = remote.clone();
                            self.pipeline_cache.insert(root, remote, runs.clone());
                        }
                        self.pipeline_poll_blocked = false;
                        self.pipelines = PipelineState::Ready(runs);
                    }
                    Err(error) => {
                        self.pipeline_poll_blocked = true;
                        if matches!(
                            error,
                            Failure::Unavailable | Failure::Read | Failure::RateLimited
                        ) && matches!(self.pipelines, PipelineState::Ready(_))
                        {
                            self.notifications.error(error_key(error), None);
                        } else {
                            self.pipeline_cache.clear();
                            self.pipelines = PipelineState::Failed(error);
                        }
                    }
                }
                if let Some(selected) = &mut self.jobs.selected
                    && let PipelineState::Ready(runs) = &self.pipelines
                    && let Some(run) = runs.iter().find(|run| run.id == selected.id)
                {
                    *selected = run.clone();
                }
                if self.workspace.preferences.show_pipelines
                    && self.navigation != Navigation::Settings
                {
                    return Task::batch([
                        self.refresh_jobs(),
                        self.refresh_pipeline_overviews(false),
                    ]);
                }
            }
            Message::ChooseProvider(provider) => {
                self.reset_jobs();
                self.cancel_auth();
                self.account_username = None;
                self.pipelines = PipelineState::Idle;
                if let Some(remote) = self
                    .repository
                    .as_mut()
                    .and_then(RepositorySet::active_mut)
                    .and_then(|repo| repo.remote.as_mut())
                {
                    remote.provider = Some(provider);
                    self.pipeline_refreshing = false;
                    self.workspace
                        .providers
                        .insert(remote.host.clone(), provider);
                    return Task::batch([self.load_pipelines(), self.save()]);
                }
            }
            Message::SignIn => return self.begin_auth(),
            Message::Auth(message) => return self.update_auth(message),
            Message::OpenUrl(url) => {
                return Task::perform(pipelines::open_url(url), Message::Opened);
            }
            Message::Opened(Err(_)) => self.notifications.error(Key::OpenFailed, None),
            Message::SetLanguage(language) => {
                self.workspace.preferences.language = (language != "auto").then_some(language);
                self.apply_language();
                return self.save();
            }
            Message::SetAppearance(appearance) => {
                self.workspace.preferences.appearance = appearance;
                return self.save();
            }
            Message::SetCodeSize(size) => {
                self.workspace.preferences.code_size = size.clamp(11, 22);
                return self.save();
            }
            Message::SetLineNumbers(value) => {
                self.workspace.preferences.line_numbers = value;
                return self.save();
            }
            Message::CopyPath => {
                if let Some(path) = self.preview().path.as_ref().or(self.workspace.directory()) {
                    return iced::clipboard::write(path.to_string_lossy().into_owned());
                }
            }
            Message::SelectTab(id) => return self.select_tab(id),
            Message::CloseTab(id) => return self.close_tab(id),
            Message::ToggleUnchanged(id) => return self.toggle_unchanged(id),
            Message::ToggleDiffView(id) => return self.toggle_diff_view(id),
            Message::ExpandUnchanged(id, range) => return self.expand_unchanged(id, range),
            Message::DiffScrolled { id, side, x, y } => return self.scroll_diff(id, side, x, y),
            Message::CloseActiveTab if self.navigation != Navigation::Settings => {
                return self.close_tab(self.preview().id);
            }
            Message::ReopenClosedTab if self.navigation != Navigation::Settings => {
                return self.reopen_closed_tab();
            }
            Message::CodeScrolled { id, x, y } => {
                if let Some(preview) = self.preview_for_request(id) {
                    preview.scroll_x = x;
                    preview.scroll_y = y;
                }
            }
            Message::Notification(event) => self.notifications.update(event),
            _ => {}
        }
        Task::none()
    }

    fn save(&mut self) -> Task<Message> {
        if self.save_blocked {
            return Task::none();
        }
        if self.saving {
            self.save_pending = true;
            return Task::none();
        }
        self.saving = true;
        Task::perform(workspace::save(self.workspace.clone()), Message::Saved)
    }

    fn apply_language(&mut self) {
        let language = self
            .workspace
            .preferences
            .language
            .as_deref()
            .map(|language| Language::select(Some(language), None))
            .unwrap_or_else(crate::infrastructure::locale::detect);
        self.i18n = Rc::new(I18n::new(language));
    }

    fn load_directory(&mut self) -> Task<Message> {
        self.git_actions.menu = None;
        self.project_menu = None;
        let directory = self.workspace.directory().cloned();
        let same_project = self.project_directory == directory;
        self.project_directory = directory;
        self.cancel_auth();
        self.disconnecting = false;
        self.generation += 1;
        if !same_project {
            self.file_panel = self.workspace.preferences.file_panels[0];
            self.file_panel_visible = true;
            self.collapsed_worktree_folders.clear();
            self.reset_jobs();
            self.repository = None;
            self.account_username = None;
            self.pipelines = PipelineState::Idle;
        }
        self.repository_error = None;
        self.history_loading = false;
        self.history_error = None;
        self.history_scroll_y = 0.;
        self.changes = changes::State::default();
        self.branch = None;
        self.query.clear();
        self.close_commit_files();
        self.expanded_branch_folders = BTreeSet::from(["false".to_owned()]);
        if let Some(repos) = &self.repository
            && repos.repositories.len() > 1
        {
            for repo in &repos.repositories {
                self.expanded_branch_folders
                    .insert(format!("repo:{}", repo.root.display()));
                self.expanded_branch_folders
                    .insert(format!("{}:false", repo.root.display()));
            }
        }
        self.collapsed_commit_folders.clear();
        self.changed_files.clear();
        self.changes_loading = false;
        self.changes_error = None;
        self.pipeline_refreshing = false;
        self.pipeline_reload_after_repository = true;
        self.sync_panels();
        let previews = self.sync_preview_project();
        let files = self.load_file_tree();
        if self.workspace.directory().is_some() {
            Task::batch([
                self.load_repository(),
                self.refresh_changes(),
                files,
                previews,
            ])
        } else {
            self.loading = false;
            Task::none()
        }
    }

    fn show_commit_files(&mut self) {
        if self
            .git_panes
            .iter()
            .any(|(_, panel)| matches!(panel, GitPanel::Files))
        {
            return;
        }
        let commit_pane = self
            .git_panes
            .iter()
            .find(|(_, panel)| matches!(panel, GitPanel::Commits))
            .map(|(pane, _)| *pane);
        if let Some(pane) = commit_pane
            && let Some((_, split)) =
                self.git_panes
                    .split(pane_grid::Axis::Vertical, pane, GitPanel::Files)
        {
            self.git_panes.resize(split, 0.72);
        }
    }

    fn close_commit_files(&mut self) {
        self.selected_commit = None;
        self.changed_files.clear();
        self.changes_loading = false;
        self.changes_error = None;
        self.commit_request += 1;
        let pane = self
            .git_panes
            .iter()
            .find(|(_, panel)| matches!(panel, GitPanel::Files))
            .map(|(pane, _)| *pane);
        if let Some(pane) = pane {
            let _ = self.git_panes.close(pane);
        }
    }

    fn load_pipelines(&mut self) -> Task<Message> {
        self.request_pipelines(false)
    }

    fn request_pipelines(&mut self, retry_credentials: bool) -> Task<Message> {
        if self.disconnecting || self.pipeline_refreshing {
            return Task::none();
        }
        let Some(remote) = self
            .active_repository()
            .and_then(|repo| repo.remote.clone())
            .filter(|remote| remote.provider.is_some())
        else {
            return Task::none();
        };
        self.pipeline_request += 1;
        self.pipeline_refreshing = true;
        if !matches!(self.pipelines, PipelineState::Ready(_))
            && let Some(root) = self.active_repository().map(|repo| repo.root.clone())
            && let Some(runs) = self.pipeline_cache.get(&root, &remote)
        {
            self.pipelines = PipelineState::Ready(runs);
        }
        if !matches!(self.pipelines, PipelineState::Ready(_)) {
            self.pipelines = PipelineState::Loading;
        }
        let (generation, request) = (self.generation, self.pipeline_request);
        let service = self.auth_service.clone();
        Task::perform(
            async move {
                if retry_credentials && let Err(error) = service.retry_credentials(&remote).await {
                    return pipelines::Loaded {
                        runs: Err(error),
                        username: None,
                    };
                }
                service.load(remote).await
            },
            move |result| Message::PipelinesLoaded(generation, request, result),
        )
    }

    fn pipeline_subscription(&self) -> Subscription<Message> {
        if self.navigation != Navigation::Settings
            && !self.pipeline_poll_blocked
            && self.has_remote()
            && (self.workspace.preferences.show_git || self.workspace.preferences.show_pipelines)
            && matches!(self.pipelines, PipelineState::Ready(_))
        {
            iced::time::every(std::time::Duration::from_secs(30)).map(|_| Message::PollPipelines)
        } else {
            Subscription::none()
        }
    }

    pub(crate) fn active_pipelines(&self, commit: &str) -> Vec<&Pipeline> {
        match &self.pipelines {
            PipelineState::Ready(runs) => runs
                .iter()
                .filter(|run| run.commit.as_deref() == Some(commit) && run.is_active())
                .collect(),
            _ => Vec::new(),
        }
    }

    pub(crate) fn has_remote(&self) -> bool {
        self.active_repository()
            .is_some_and(|repo| repo.remote.is_some())
    }

    fn sync_panels(&mut self) {
        let show_git = self.workspace.preferences.show_git && self.workspace.directory().is_some();
        let show_pipelines = self.workspace.preferences.show_pipelines && self.has_remote();
        if self.panes.iter().any(|(_, panel)| *panel == Panel::Git) == show_git
            && self
                .panes
                .iter()
                .any(|(_, panel)| *panel == Panel::Pipelines)
                == show_pipelines
        {
            return;
        }
        let (mut panes, top) = pane_grid::State::new(Panel::Workspace);
        if show_git
            && let Some((_, split)) = panes.split(pane_grid::Axis::Horizontal, top, Panel::Git)
        {
            panes.resize(split, 0.62);
        } else if show_pipelines
            && let Some((_, split)) =
                panes.split(pane_grid::Axis::Horizontal, top, Panel::Pipelines)
        {
            panes.resize(split, 0.72);
        }
        self.panes = panes;
    }

    fn view(&self) -> Element<'_, Message> {
        screens::workspace::view(self)
    }
}

async fn prepare_document(
    result: Result<crate::domain::changes::Source, Failure>,
    extension: String,
) -> Result<Document, Failure> {
    let source = result?;
    tokio::task::spawn_blocking(move || Document::from_source(source, &extension))
        .await
        .map_err(|_| Failure::Read)
}

pub(crate) fn error_key(error: Failure) -> Key {
    match error {
        Failure::MissingTool => Key::GitMissing,
        Failure::NotRepository => Key::NotRepository,
        Failure::Read => Key::ReadFailed,
        Failure::TooLarge => Key::TooLarge,
        Failure::Binary => Key::Binary,
        Failure::Access => Key::AccessRequired,
        Failure::Unavailable => Key::Unavailable,
        Failure::Storage => Key::StorageSaveFailed,
        Failure::Credentials => Key::CredentialsUnavailable,
        Failure::AuthNotConfigured => Key::AuthNotConfigured,
        Failure::AuthCancelled => Key::Cancelled,
        Failure::AuthExpired => Key::AuthExpired,
        Failure::AuthDenied => Key::AuthDenied,
        Failure::AuthPortBusy => Key::AuthPortBusy,
        Failure::InvalidToken => Key::InvalidToken,
        Failure::PermissionDenied => Key::PermissionDenied,
        Failure::RateLimited => Key::RateLimited,
    }
}

#[cfg(test)]
mod tests;
