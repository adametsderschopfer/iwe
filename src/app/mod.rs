//! Coordinate asynchronous reads, workspace state, and the native window.
pub(crate) mod editor;
use crate::modules::ModuleId;
pub(crate) use crate::modules::{git::GitPanel, pipelines::PipelineState};

pub(crate) mod auth;
pub(crate) use modules::git::branch_selection;
pub(crate) mod changes;
pub(crate) mod files;
pub(crate) use modules::git::git_actions;
pub(crate) use modules::pipelines::jobs;
pub(crate) mod line_navigation;
#[cfg(target_os = "windows")]
pub(crate) mod menu_bar;
pub(crate) mod modules;
#[cfg(target_os = "macos")]
mod native_menu;
pub(crate) mod notifications;
pub(crate) mod project_add;
pub(crate) mod project_changes;
mod repositories;
mod repository_cache;
pub(crate) mod search_everywhere;
pub(crate) mod shortcuts;
pub(crate) mod spaces;
pub(crate) mod tabs;
pub(crate) mod updates;
pub(crate) mod windows;

use crate::domain::{
    Failure,
    repository::{
        ChangedFile, Provider,
        project::{BranchKey, CommitKey, ProjectHistoryPage as HistoryPage, RepositorySet},
    },
    workspace::{Appearance, FileEntry, FilePanel, Workspace},
};
use crate::i18n::{I18n, Key, Language};
use crate::infrastructure::{pipelines, workspace};
use crate::ui::{code::Document, screens, theme};
use iced::{Element, Program, Subscription, Task, Theme, keyboard, widget::pane_grid, window};
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
) -> iced::Daemon<impl Program<State = App, Message = Message, Theme = Theme>> {
    iced::daemon(
        move || {
            let mut app = App::new(Rc::clone(&i18n));
            let (id, open) = window::open(windows::main_settings());
            app.window_id = Some(id);
            (
                app,
                Task::batch([
                    open.map(Message::WindowOpened),
                    Task::perform(workspace::load(), Message::Loaded),
                    modules::external::startup()
                        .map(|event| Message::Module(modules::Event::External(event))),
                    Task::done(Message::Update(updates::Event::Automatic(
                        updates::Trigger::Periodic,
                    ))),
                ]),
            )
        },
        App::update,
        App::view_window,
    )
    .title(App::window_title)
    .subscription(|app| {
        Subscription::batch([
            app.change_subscription(),
            app.project_change_subscription(),
            app.module_subscription(),
            iced::time::every(std::time::Duration::from_secs(60))
                .map(|_| Message::Update(updates::Event::Automatic(updates::Trigger::Periodic))),
            window::events().filter_map(|(id, event)| match event {
                window::Event::Unfocused => Some(Message::BranchSelection(
                    branch_selection::Event::Unfocused(id),
                )),
                window::Event::Focused => Some(Message::Update(updates::Event::Automatic(
                    updates::Trigger::Focused,
                ))),
                _ => None,
            }),
            app.pipeline_subscription(),
            app.jobs_subscription(),
            app.notifications.subscription().map(Message::Notification),
            window::close_requests().map(|id| Message::Window(windows::Event::Close(id))),
            window::close_events().map(|id| Message::Window(windows::Event::Closed(id))),
            #[cfg(target_os = "macos")]
            app.native_menu_subscription(),
            window_state_subscription(),
            iced::event::listen_with(|event, status, id| {
                if let iced::Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = event {
                    Some(Message::BranchSelection(
                        branch_selection::Event::Modifiers(id, modifiers),
                    ))
                } else if let iced::Event::Keyboard(event) = event {
                    Some(Message::Shortcut(shortcuts::Event::Key(id, event, status)))
                } else {
                    None
                }
            }),
        ])
    })
    .theme(|app: &App, _| app.theme())
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

#[cfg(test)]
fn shortcut(event: iced::Event) -> Option<Message> {
    let bindings = crate::domain::shortcuts::Shortcuts::default();
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
    shortcuts::key_message(&key, physical_key, modifiers, &bindings)
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
    Modules,
    Connections,
    Deployment,
    Shortcuts,
    About,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Panel {
    Workspace,
    Module(crate::modules::ModuleId),
}
pub(crate) struct App {
    pub i18n: Rc<I18n>,
    pub workspace: Workspace,
    pub navigation: Navigation,
    pub settings_tab: SettingsTab,
    pub shortcuts: shortcuts::State,
    pub modules: modules::State,
    pub panes: pane_grid::State<Panel>,
    pub repository: Option<RepositorySet>,
    pub repository_error: Option<Failure>,
    pub loading: bool,
    pub tree: BTreeMap<PathBuf, Vec<FileEntry>>,
    pub expanded: BTreeSet<PathBuf>,
    pub files: files::State,
    pub file_panel: FilePanel,
    pub file_panel_visible: bool,
    pub project_menu: Option<(usize, PathBuf, iced::Point)>,
    pub space_menu: Option<(usize, iced::Point)>,
    pub repository_menu: Option<iced::Point>,
    #[cfg(target_os = "windows")]
    pub(crate) menu_bar: Option<(menu_bar::Menu, iced::Point)>,
    pub collapsed_worktree_folders: BTreeSet<String>,
    pub changes: changes::State,
    repository_cache: repository_cache::Cache,
    pub project_changes: project_changes::State,
    pub tabs: tabs::State,
    pub editor: editor::State,
    pub collapsed_spaces: BTreeSet<usize>,
    pub auth_dialog: Option<auth::Dialog>,
    pub space_draft: Option<spaces::Draft>,
    pub project_add: Option<project_add::Draft>,
    pub line_navigation: Option<line_navigation::Dialog>,
    pub search_everywhere: search_everywhere::State,
    pub account_username: Option<String>,
    pub disconnecting: bool,
    pub notifications: notifications::State,
    pub updates: updates::State,
    update_schedule: updates::Schedule,
    pub picking: bool,
    pub loaded: bool,
    window_id: Option<window::Id>,
    pub(crate) windows: windows::State,
    #[cfg(target_os = "macos")]
    native_menu: native_menu::State,
    pub window_maximized: bool,
    glass_ready: bool,
    generation: u64,
    project_directory: Option<PathBuf>,
    repo_request: u64,
    file_request: u64,
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
    Editor(editor::Event),
    WindowOpened(window::Id),
    Window(windows::Event),
    WindowShortcut(window::Id, Box<Message>),
    #[cfg(target_os = "macos")]
    NativeMenu(native_menu::Event),
    Update(updates::Event),
    DragWindow,
    ToggleWindowMaximized,
    RevealGitBranches,
    RevealRepositoryBranches(PathBuf),
    OpenRepositoryMenu(iced::Point),
    CloseRepositoryMenu,
    ClearBranchAttention(u64),
    WindowMaximized(bool),
    #[cfg(target_os = "windows")]
    WindowResized(window::Id),
    #[cfg(target_os = "windows")]
    MinimizeWindow,
    #[cfg(target_os = "windows")]
    CloseWindow,
    #[cfg(target_os = "windows")]
    ResizeWindow(window::Direction),
    #[cfg(target_os = "windows")]
    OpenMenuBar(menu_bar::Menu, iced::Point),
    #[cfg(target_os = "windows")]
    CloseMenuBar,
    #[cfg(target_os = "windows")]
    MenuAbout,
    GlassConfigured(bool),
    Loaded(Result<(Workspace, Option<PathBuf>), Failure>),
    Saved(Result<(), Failure>),
    Navigate(Navigation),
    SelectSettingsTab(SettingsTab),
    Module(modules::Event),
    Space(spaces::Message),
    ProjectAdd(project_add::Message),
    Git(git_actions::Message),
    BranchSelection(branch_selection::Event),
    ProjectMenu(usize, PathBuf, iced::Point),
    SpaceMenu(usize, iced::Point),
    CloseSpaceMenu,
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
    ToggleWorktreeFolder {
        repository: Option<PathBuf>,
        path: String,
    },
    Files(files::Event),
    Worktree(changes::Event),
    ProjectChanges(project_changes::Event),
    OpenFile(PathBuf),
    OpenWorkingChange(PathBuf),
    GoToLine(line_navigation::Event),
    SearchEverywhere(search_everywhere::Event),
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
    Shortcut(shortcuts::Event),
    SetCodeSize(u16),
    SetLineNumbers(bool),
    CopyPath,
    OpenTabMenu(iced::Point),
    CloseTabMenu,
    CopyCommitId(String),
    CommitIdCopied(u64, String),
    ClearCopiedCommit(u64),
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

    pub(crate) fn new(i18n: Rc<I18n>) -> Self {
        let mut app = Self {
            i18n,
            workspace: Workspace::default(),
            navigation: Navigation::Projects,
            settings_tab: SettingsTab::default(),
            shortcuts: shortcuts::State::default(),
            modules: modules::State::default(),
            panes: pane_grid::State::new(Panel::Workspace).0,
            repository: None,
            repository_error: None,
            loading: false,
            tree: BTreeMap::new(),
            expanded: BTreeSet::new(),
            files: files::State::default(),
            file_panel: FilePanel::Project,
            file_panel_visible: true,
            project_menu: None,
            space_menu: None,
            repository_menu: None,
            #[cfg(target_os = "windows")]
            menu_bar: None,
            collapsed_worktree_folders: BTreeSet::new(),
            changes: changes::State::default(),
            repository_cache: repository_cache::Cache::default(),
            project_changes: project_changes::State::default(),
            tabs: tabs::State::default(),
            editor: editor::State::default(),
            collapsed_spaces: BTreeSet::new(),
            auth_dialog: None,
            space_draft: None,
            project_add: None,
            line_navigation: None,
            search_everywhere: search_everywhere::State::default(),
            account_username: None,
            disconnecting: false,
            notifications: notifications::State::default(),
            updates: if std::env::var_os("IWE_UPDATE_FAILED").as_deref() == Some("1".as_ref()) {
                updates::State::Failed(crate::infrastructure::updates::Error::Install)
            } else {
                updates::State::default()
            },
            picking: false,
            update_schedule: updates::Schedule::default(),
            loaded: false,
            window_id: None,
            windows: windows::State::default(),
            #[cfg(target_os = "macos")]
            native_menu: native_menu::State::default(),
            window_maximized: false,
            glass_ready: false,
            generation: 0,
            project_directory: None,
            repo_request: 0,
            file_request: 0,
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
        if self.protect_editor(&message) {
            return self.flush_editor();
        }
        #[cfg(target_os = "windows")]
        if !matches!(message, Message::OpenMenuBar(..)) {
            self.menu_bar = None;
        }
        if self.editor.pending.is_some() && matches!(message, Message::DismissDialog) {
            return self.update_editor(editor::Event::Cancel);
        }
        use crate::modules::ModuleId;
        let required = match &message {
            Message::OpenWorkingChange(_)
            | Message::Git(_)
            | Message::BranchSelection(_)
            | Message::RevealGitBranches
            | Message::RevealRepositoryBranches(_)
            | Message::OpenRepositoryMenu(_)
            | Message::SelectBranch(_)
            | Message::AllRepositories
            | Message::Search(_)
            | Message::SelectCommit(_)
            | Message::OpenCommitFile(_)
            | Message::LoadMoreHistory
            | Message::HistoryScrolled { .. }
            | Message::HistoryLoaded(..)
            | Message::ChangesLoaded(..)
            | Message::Worktree(_)
            | Message::ProjectChanges(_) => Some(ModuleId::Git),
            Message::Jobs(_)
            | Message::RefreshPipelines
            | Message::PollPipelines
            | Message::PipelinesLoaded(..)
            | Message::ChooseProvider(_)
            | Message::SignIn => Some(ModuleId::Pipelines),
            Message::SelectSettingsTab(SettingsTab::Connections | SettingsTab::Deployment) => {
                Some(ModuleId::Deployment)
            }
            Message::TogglePanel(Panel::Module(id)) => Some(*id),
            Message::SelectFilePanel(FilePanel::Changes)
            | Message::ToggleFilePanel(FilePanel::Changes) => Some(ModuleId::Git),
            _ => None,
        };
        if required.is_some_and(|id| !self.module_enabled(id)) {
            return Task::none();
        }

        if matches!(
            self.updates,
            updates::State::Saving(_) | updates::State::Installing(_)
        ) && !matches!(
            message,
            Message::Saved(_) | Message::Update(updates::Event::InstallFailed(_))
        ) {
            return Task::none();
        }
        if (self.editor.pending.is_some()
            || self.auth_dialog.is_some()
            || self.line_navigation.is_some()
            || self.search_everywhere.dialog.is_some()
            || self.project_add.is_some()
            || self.space_draft.is_some()
            || self.modules.git.git_actions.has_dialog())
            && matches!(
                message,
                Message::Navigate(_)
                    | Message::SelectSettingsTab(_)
                    | Message::Module(modules::Event::Toggle(_))
                    | Message::Module(modules::Event::Terminal(
                        crate::modules::terminal::Event::Start
                            | crate::modules::terminal::Event::Close(_)
                            | crate::modules::terminal::Event::Select(_)
                            | crate::modules::terminal::Event::Hide
                            | crate::modules::terminal::Event::Input(..)
                            | crate::modules::terminal::Event::Paste(_)
                            | crate::modules::terminal::Event::Pasted(..)
                            | crate::modules::terminal::Event::Focus(_)
                    ))
                    | Message::SelectProject(_, _)
                    | Message::RevealGitBranches
                    | Message::RevealRepositoryBranches(_)
                    | Message::RemoveProjectFromSpace
                    | Message::SelectFilePanel(_)
                    | Message::ToggleFilePanel(_)
                    | Message::ResizeFilePanel(_, _)
                    | Message::MoveFilePanel(_, _)
                    | Message::Refresh
                    | Message::SelectBranch(_)
                    | Message::AllRepositories
                    | Message::SelectCommit(_)
                    | Message::CopyCommitId(_)
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
            Message::Editor(event) => return self.update_editor(event),
            Message::OpenTabMenu(point)
                if self.space_draft.is_none()
                    && self.project_add.is_none()
                    && self.auth_dialog.is_none()
                    && !self.modules.git.git_actions.has_dialog() =>
            {
                self.space_menu = None;
                self.project_menu = None;
                self.files.menu = None;
                self.modules.git.git_actions.menu = None;
                self.modules.git.git_actions.bulk.menu = None;
                self.tabs.menu = Some(point);
                self.repository_menu = None;
            }
            Message::CloseTabMenu => self.tabs.menu = None,
            Message::OpenRepositoryMenu(point)
                if self.space_draft.is_none()
                    && self.project_add.is_none()
                    && self.auth_dialog.is_none()
                    && self.line_navigation.is_none()
                    && !self.modules.git.git_actions.has_dialog()
                    && self
                        .repository
                        .as_ref()
                        .is_some_and(|repos| repos.repositories.len() > 2) =>
            {
                self.tabs.menu = None;
                self.space_menu = None;
                self.project_menu = None;
                self.files.menu = None;
                self.modules.git.git_actions.menu = None;
                self.modules.git.git_actions.bulk.menu = None;
                self.repository_menu = Some(point);
            }
            Message::CloseRepositoryMenu => self.repository_menu = None,
            Message::RevealRepositoryBranches(root) => {
                return self.reveal_repository_branches(root);
            }
            Message::ProjectChanges(event) => return self.update_project_changes(event),
            Message::Update(event) => return self.update_release(event),
            Message::GoToLine(event) => return self.update_line_navigation(event),
            Message::SearchEverywhere(event) => return self.update_search_everywhere(event),
            Message::Git(message) => return self.update_git(message),
            Message::BranchSelection(event) => return self.update_branch_selection(event),
            Message::ProjectMenu(space, path, point)
                if self.space_draft.is_none()
                    && self.project_add.is_none()
                    && self.auth_dialog.is_none()
                    && !self.modules.git.git_actions.has_dialog() =>
            {
                self.space_menu = None;
                self.modules.git.git_actions.menu = None;
                self.files.menu = None;
                self.project_menu = Some((space, path, point));
            }
            Message::CloseProjectMenu => self.project_menu = None,
            Message::SpaceMenu(index, point)
                if self.loaded
                    && !self.picking
                    && self.space_draft.is_none()
                    && self.project_add.is_none()
                    && self.auth_dialog.is_none()
                    && !self.modules.git.git_actions.has_dialog()
                    && index < self.workspace.spaces.len() =>
            {
                self.project_menu = None;
                self.modules.git.git_actions.menu = None;
                self.modules.git.git_actions.bulk.menu = None;
                self.files.menu = None;
                self.space_menu = Some((index, point));
            }
            Message::CloseSpaceMenu => self.space_menu = None,
            #[cfg(target_os = "windows")]
            Message::OpenMenuBar(menu, point) => {
                self.space_menu = None;
                self.project_menu = None;
                self.files.menu = None;
                self.modules.git.git_actions.menu = None;
                self.modules.git.git_actions.bulk.menu = None;
                self.tabs.menu = None;
                self.repository_menu = None;
                self.menu_bar = Some((menu, point));
            }
            #[cfg(target_os = "windows")]
            Message::CloseMenuBar => self.menu_bar = None,
            #[cfg(target_os = "windows")]
            Message::MenuAbout if !self.modules.deployment.has_dialog() => {
                self.shortcuts = shortcuts::State::default();
                self.settings_tab = SettingsTab::About;
                return self.update(Message::Navigate(Navigation::Settings));
            }
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
            Message::WindowOpened(id) => return self.window_opened(id),
            Message::Window(event) => return self.update_window(event),
            #[cfg(target_os = "macos")]
            Message::NativeMenu(event) => return self.update_native_menu(event),
            Message::WindowShortcut(id, message) => {
                if self.windows.is_secondary(id) {
                    return match *message {
                        Message::DismissDialog if self.editor.pending.is_some() => {
                            self.update_editor(editor::Event::Cancel)
                        }
                        Message::DismissDialog
                            if self.windows.is_deployment(id)
                                && self
                                    .modules
                                    .deployment
                                    .has_settings_dialog(self.windows.connections == Some(id)) =>
                        {
                            self.update_module(modules::Event::Deployment(
                                crate::modules::deployment::Event::CloseDialog,
                            ))
                        }
                        Message::DismissDialog | Message::CloseActiveTab => {
                            self.update_window(windows::Event::Close(id))
                        }
                        Message::Navigate(Navigation::Settings) => self.open_settings(),
                        _ => Task::none(),
                    };
                }
                if self.window_id == Some(id) {
                    return self.update(*message);
                }
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
            Message::WindowResized(id) if self.windows.is_secondary(id) => {
                return window::is_maximized(id)
                    .map(move |value| Message::Window(windows::Event::Maximized(id, value)));
            }
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
                return Task::batch([
                    self.load_directory(),
                    self.refresh_project_changes(),
                    self.save(),
                    dialog,
                ]);
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
            Message::Navigate(Navigation::Settings) => return self.open_settings(),
            Message::Navigate(navigation) => {
                self.navigation = navigation;
                self.modules.pipelines.jobs.overview.popup = None;
                return self.refresh_pipeline_overviews(false);
            }
            Message::SelectSettingsTab(tab) => {
                self.shortcuts = shortcuts::State::default();
                let (connections, event) = match tab {
                    SettingsTab::Connections => {
                        (true, crate::modules::deployment::Event::OpenConnections)
                    }
                    SettingsTab::Deployment => {
                        (false, crate::modules::deployment::Event::OpenProfiles)
                    }
                    _ => {
                        self.settings_tab = tab;
                        return Task::none();
                    }
                };
                return Task::batch([
                    self.update_module(modules::Event::Deployment(event)),
                    self.open_deployment_settings(connections),
                ]);
            }
            Message::Shortcut(event) => return self.update_shortcuts(event),
            Message::Module(event) => return self.update_module(event),
            Message::Space(message) => return self.update_space(message),
            Message::ProjectAdd(message) => return self.update_project_add(message),
            Message::DismissDialog => {
                if self.modules.deployment.target.is_some()
                    || (self.navigation == Navigation::Settings
                        && self.modules.deployment.has_dialog())
                {
                    return self.update_module(modules::Event::Deployment(
                        crate::modules::deployment::Event::CloseDialog,
                    ));
                }
                if self.modules.deployment.menu.take().is_some() {
                    return Task::none();
                }
                if self.modules.deployment.activity_expanded {
                    return self.update_module(modules::Event::Deployment(
                        crate::modules::deployment::Event::DismissActivity,
                    ));
                }
                if self.repository_menu.take().is_some() {
                    return Task::none();
                }
                if self.tabs.menu.take().is_some() {
                    return Task::none();
                }
                if self.space_menu.take().is_some() {
                    return Task::none();
                }
                if self.search_everywhere.dialog.take().is_some() {
                    return Task::none();
                }
                if self.line_navigation.take().is_some() {
                    return Task::none();
                }
                if self.files.menu.take().is_some() {
                    return Task::none();
                }
                self.project_menu = None;
                if self.modules.git.git_actions.menu.is_some()
                    || self.modules.git.git_actions.bulk.menu.is_some()
                    || self.modules.git.git_actions.has_dialog()
                {
                    return self.update_git(git_actions::Message::Close);
                }
                if self.project_add.is_some() {
                    return self.update_project_add(project_add::Message::Cancel);
                }
                if self.space_draft.is_some() {
                    return self.update_space(spaces::Message::Cancel);
                }
                if !self.modules.git.branch_selection.selected.is_empty() {
                    return self.update_branch_selection(branch_selection::Event::Clear);
                }
                if self.auth_dialog.is_none()
                    && self.modules.pipelines.jobs.overview.popup.take().is_some()
                {
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
            Message::ToggleWorktreeFolder { repository, path } => {
                self.toggle_worktree_folder(repository, path)
            }
            Message::RepositoryLoaded(generation, request, result)
                if generation == self.generation && request == self.repo_request =>
            {
                self.loading = false;
                let reload_pipelines =
                    std::mem::take(&mut self.modules.pipelines.pipeline_reload_after_repository);
                match result {
                    Ok(mut repo) => {
                        if repo.repositories.len() <= 2 {
                            self.repository_menu = None;
                        }
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
                                self.modules
                                    .git
                                    .expanded_branch_folders
                                    .insert(format!("repo:{}", member.root.display()));
                                self.modules
                                    .git
                                    .expanded_branch_folders
                                    .insert(format!("{}:false", member.root.display()));
                            }
                        }
                        let changed_remote = self
                            .active_repository()
                            .and_then(|repo| repo.remote.as_ref())
                            != repo.active().and_then(|repo| repo.remote.as_ref());
                        if changed_remote {
                            self.reset_jobs();
                            self.modules.pipelines.pipelines = PipelineState::Idle;
                            self.account_username = None;
                            self.modules.pipelines.pipeline_request += 1;
                            self.modules.pipelines.pipeline_refreshing = false;
                        }
                        if self.modules.git.branch.as_ref().is_some_and(|key| {
                            repo.get(&key.root).is_none_or(|member| {
                                !member
                                    .branches
                                    .iter()
                                    .any(|branch| branch.reference == key.reference)
                            })
                        }) {
                            self.modules.git.branch = None;
                        }
                        if self.modules.git.branch.is_none() && repo.repositories.len() == 1 {
                            self.modules.git.branch =
                                repo.active().and_then(|member| {
                                    member.branches.iter().find(|branch| branch.current).map(
                                        |branch| BranchKey {
                                            root: member.root.clone(),
                                            reference: branch.reference.clone(),
                                        },
                                    )
                                });
                        }
                        self.modules.git.branch_selection.prune(&repo);
                        self.repository = Some(repo);
                        self.repository_error = None;
                        self.cache_repository();
                        self.sync_panels();
                        let pipeline_task = if (self.workspace.preferences.show_pipelines
                            || self.workspace.preferences.show_git)
                            && (reload_pipelines
                                || matches!(self.modules.pipelines.pipelines, PipelineState::Idle))
                        {
                            self.load_pipelines()
                        } else {
                            Task::none()
                        };
                        let history_task = if self.modules.git.query.is_empty() {
                            Task::none()
                        } else {
                            self.load_more_history()
                        };
                        return Task::batch([pipeline_task, history_task]);
                    }
                    Err(error) => {
                        self.repository_error = self.repository.is_none().then_some(error);
                        if error == Failure::NotRepository {
                            self.repository_menu = None;
                            if let Some(root) = self.workspace.directory() {
                                self.repository_cache.remove(root);
                            }
                            self.repository_error = Some(error);
                            self.reset_jobs();
                            self.repository = None;
                            self.modules.pipelines.pipelines = PipelineState::Idle;
                            self.account_username = None;
                            self.modules.pipelines.pipeline_request += 1;
                            self.modules.pipelines.pipeline_refreshing = false;
                        } else if self.repository.is_some() {
                            self.notifications.error(error_key(error), None);
                        }
                        self.sync_panels();
                    }
                }
            }
            Message::HistoryLoaded(generation, request, offset, result)
                if generation == self.generation
                    && request == self.repo_request
                    && self.modules.git.history_loading
                    && self
                        .repository
                        .as_ref()
                        .is_some_and(|repo| repo.commits.len() == offset) =>
            {
                self.modules.git.history_loading = false;
                match result {
                    Ok(page) => {
                        if let Some(repo) = self.repository.as_mut() {
                            repo.commits.extend(page.commits);
                            repo.next_history = page.next;
                        }
                        self.modules.git.history_error = None;
                        if !self.modules.git.query.is_empty() {
                            return self.load_more_history();
                        }
                    }
                    Err(error) => self.modules.git.history_error = Some(error),
                }
            }
            Message::HistoryScrolled { offset, remaining } => {
                self.modules.git.history_scroll_y = offset;
                if remaining <= 480. && self.modules.git.history_error.is_none() {
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
            Message::OpenWorkingChange(path) => return self.open_working_change(path),
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
                self.modules.git.branch_selection.selected.clear();
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
                self.modules.git.branch = Some(reference);
                self.close_commit_files();
                return self.load_repository();
            }
            Message::AllRepositories => {
                self.modules.git.branch_selection.selected.clear();
                self.modules.git.branch = None;
                self.close_commit_files();
                return self.load_repository();
            }
            Message::Search(query) => {
                self.modules.git.query = query;
                let scroll = self.reset_history_scroll();
                let history = if self.modules.git.query.is_empty() {
                    Task::none()
                } else {
                    self.load_more_history()
                };
                return Task::batch([scroll, history]);
            }
            Message::ToggleCommitFolder(path) => {
                if !self.modules.git.collapsed_commit_folders.remove(&path) {
                    self.modules.git.collapsed_commit_folders.insert(path);
                }
            }
            Message::ToggleBranchFolder(path) => {
                self.modules.git.branch_selection.selected.clear();
                if !self.modules.git.expanded_branch_folders.remove(&path) {
                    self.modules.git.expanded_branch_folders.insert(path);
                }
            }
            Message::SelectCommit(id) => return self.select_commit(id),
            Message::CloseCommitFiles => self.close_commit_files(),
            Message::ChangesLoaded(generation, request, result)
                if generation == self.generation && request == self.modules.git.commit_request =>
            {
                self.modules.git.changes_loading = false;
                match result {
                    Ok(files) => self.modules.git.changed_files = files,
                    Err(error) => self.modules.git.changes_error = Some(error),
                }
            }
            Message::RevealGitBranches => {
                if self.workspace.directory().is_none() {
                    return Task::none();
                }
                if !self.workspace.preferences.show_git {
                    return self.update(Message::TogglePanel(Panel::Module(
                        crate::modules::ModuleId::Git,
                    )));
                }
                self.modules.git.branch_attention_request =
                    self.modules.git.branch_attention_request.wrapping_add(1);
                let request = self.modules.git.branch_attention_request;
                self.modules.git.branch_attention = Some(request);
                return Task::perform(
                    async {
                        tokio::time::sleep(std::time::Duration::from_millis(700)).await;
                    },
                    move |_| Message::ClearBranchAttention(request),
                );
            }
            Message::ClearBranchAttention(request) => {
                if self.modules.git.branch_attention == Some(request) {
                    self.modules.git.branch_attention = None;
                }
            }
            Message::TogglePanel(panel) => {
                self.modules.git.branch_attention = None;
                self.modules.pipelines.jobs.overview.popup = None;
                let Panel::Module(id) = panel else {
                    return Task::none();
                };
                let open = !self.module_open(id);
                match id {
                    ModuleId::Git => self.workspace.preferences.show_git = open,
                    ModuleId::Pipelines => self.workspace.preferences.show_pipelines = open,
                    _ => return self.update_module(modules::Event::Toggle(id)),
                }
                if open {
                    self.hide_other_bottom_panels(id);
                }
                self.sync_panels();
                let pipelines = if (self.workspace.preferences.show_pipelines
                    || self.workspace.preferences.show_git)
                    && matches!(self.modules.pipelines.pipelines, PipelineState::Idle)
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
                self.modules.pipelines.jobs.overview.popup = None;
                self.panes
                    .resize(event.split, event.ratio.clamp(0.15, 0.85));
            }
            Message::ResizeGit(event) => self
                .modules
                .git
                .git_panes
                .resize(event.split, event.ratio.clamp(0.12, 0.88)),
            Message::RefreshPipelines => return self.request_pipelines(true),
            Message::PollPipelines => return self.load_pipelines(),
            Message::Jobs(event) => return self.update_jobs(event),
            Message::PipelinesLoaded(generation, request, result)
                if generation == self.generation
                    && request == self.modules.pipelines.pipeline_request =>
            {
                self.account_username = result.username;
                self.modules.pipelines.pipeline_refreshing = false;
                match result.runs {
                    Ok(runs) => {
                        if let Some(repo) = self.active_repository()
                            && let Some(remote) = &repo.remote
                        {
                            let root = repo.root.clone();
                            let remote = remote.clone();
                            self.modules.pipelines.pipeline_cache.insert(
                                root,
                                remote,
                                runs.clone(),
                            );
                        }
                        self.modules.pipelines.pipeline_poll_blocked = false;
                        self.modules.pipelines.pipelines = PipelineState::Ready(runs);
                    }
                    Err(error) => {
                        self.modules.pipelines.pipeline_poll_blocked = true;
                        if matches!(
                            error,
                            Failure::Unavailable | Failure::Read | Failure::RateLimited
                        ) && matches!(self.modules.pipelines.pipelines, PipelineState::Ready(_))
                        {
                            self.notifications.error(error_key(error), None);
                        } else {
                            self.modules.pipelines.pipeline_cache.clear();
                            self.modules.pipelines.pipelines = PipelineState::Failed(error);
                        }
                    }
                }
                if let Some(selected) = &mut self.modules.pipelines.jobs.selected
                    && let PipelineState::Ready(runs) = &self.modules.pipelines.pipelines
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
                self.modules.pipelines.pipelines = PipelineState::Idle;
                if let Some(remote) = self
                    .repository
                    .as_mut()
                    .and_then(RepositorySet::active_mut)
                    .and_then(|repo| repo.remote.as_mut())
                {
                    remote.provider = Some(provider);
                    self.modules.pipelines.pipeline_refreshing = false;
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
            Message::CopyCommitId(id) => {
                self.modules.git.copy_request += 1;
                return iced::clipboard::write(id.clone()).chain(Task::done(
                    Message::CommitIdCopied(self.modules.git.copy_request, id),
                ));
            }
            Message::CommitIdCopied(request, id) if request == self.modules.git.copy_request => {
                self.modules.git.copied_commit = Some(id);
                return Task::perform(
                    async {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    },
                    move |()| Message::ClearCopiedCommit(request),
                );
            }
            Message::ClearCopiedCommit(request) if request == self.modules.git.copy_request => {
                self.modules.git.copied_commit = None;
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
        self.repository_menu = None;
        if let Some(root) = self.project_directory.as_ref() {
            self.repository_cache.remember_folders(
                root,
                &self.modules.git.branch,
                &self.modules.git.expanded_branch_folders,
            );
        }
        self.tabs.menu = None;
        self.modules.git.branch_selection.selected.clear();
        self.modules.git.git_actions.bulk.menu = None;
        self.modules.git.branch_attention = None;
        self.line_navigation = None;
        self.modules.git.git_actions.menu = None;
        self.project_menu = None;
        self.space_menu = None;
        let directory = self.workspace.directory().cloned();
        self.modules
            .deployment
            .sync_context(self.workspace.selected_space, directory.as_deref());
        let same_project = self.project_directory == directory;
        self.project_directory = directory;
        self.cancel_auth();
        self.disconnecting = false;
        self.generation += 1;
        if !same_project {
            self.modules.terminal.reset();
            self.file_panel = self.workspace.preferences.file_panels[0];
            self.file_panel_visible = true;
            self.collapsed_worktree_folders.clear();
            self.reset_jobs();
            self.repository = None;
            self.account_username = None;
            self.modules.pipelines.pipelines = PipelineState::Idle;
        }
        self.repository_error = None;
        self.modules.git.history_loading = false;
        self.modules.git.history_error = None;
        self.modules.git.history_scroll_y = 0.;
        self.changes = changes::State::default();
        self.modules.git.branch = None;
        self.modules.git.query.clear();
        self.close_commit_files();
        self.modules.git.expanded_branch_folders = BTreeSet::from(["false".to_owned()]);
        let restored_git = !same_project && self.restore_cached_repository();
        if !restored_git
            && let Some(repos) = &self.repository
            && repos.repositories.len() > 1
        {
            for repo in &repos.repositories {
                self.modules
                    .git
                    .expanded_branch_folders
                    .insert(format!("repo:{}", repo.root.display()));
                self.modules
                    .git
                    .expanded_branch_folders
                    .insert(format!("{}:false", repo.root.display()));
            }
        }
        self.modules.git.collapsed_commit_folders.clear();
        self.modules.git.changed_files.clear();
        self.modules.git.changes_loading = false;
        self.modules.git.changes_error = None;
        self.modules.pipelines.pipeline_refreshing = false;
        self.modules.pipelines.pipeline_reload_after_repository = true;
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
            .modules
            .git
            .git_panes
            .iter()
            .any(|(_, panel)| matches!(panel, GitPanel::Files))
        {
            return;
        }
        let commit_pane = self
            .modules
            .git
            .git_panes
            .iter()
            .find(|(_, panel)| matches!(panel, GitPanel::Commits))
            .map(|(pane, _)| *pane);
        if let Some(pane) = commit_pane
            && let Some((_, split)) =
                self.modules
                    .git
                    .git_panes
                    .split(pane_grid::Axis::Vertical, pane, GitPanel::Files)
        {
            self.modules.git.git_panes.resize(split, 0.72);
        }
    }

    fn close_commit_files(&mut self) {
        self.modules.git.selected_commit = None;
        self.modules.git.changed_files.clear();
        self.modules.git.changes_loading = false;
        self.modules.git.changes_error = None;
        self.modules.git.commit_request += 1;
        let pane = self
            .modules
            .git
            .git_panes
            .iter()
            .find(|(_, panel)| matches!(panel, GitPanel::Files))
            .map(|(pane, _)| *pane);
        if let Some(pane) = pane {
            let _ = self.modules.git.git_panes.close(pane);
        }
    }

    pub(crate) fn has_remote(&self) -> bool {
        self.active_repository()
            .is_some_and(|repo| repo.remote.is_some())
    }

    pub(crate) fn panel_available(&self, id: ModuleId) -> bool {
        self.module_enabled(id)
            && match id {
                ModuleId::Terminal | ModuleId::Git => self.workspace.directory().is_some(),
                ModuleId::Pipelines => self.has_remote(),
                ModuleId::Deployment => false,
            }
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub(crate) fn menu_actions_enabled(&self) -> bool {
        self.loaded
            && self.editor.pending.is_none()
            && self.space_draft.is_none()
            && self.project_add.is_none()
            && self.auth_dialog.is_none()
            && !self.modules.git.git_actions.has_dialog()
            && self.line_navigation.is_none()
            && !self.modules.deployment.has_dialog()
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn main_window_close(&self) -> Option<Message> {
        self.window_id
            .map(|id| Message::Window(windows::Event::Close(id)))
    }

    fn hide_other_bottom_panels(&mut self, id: ModuleId) {
        self.workspace.preferences.show_git &= id == ModuleId::Git;
        self.workspace.preferences.show_pipelines &= id == ModuleId::Pipelines;
        self.modules.terminal.open &= id == ModuleId::Terminal;
    }

    fn sync_panels(&mut self) {
        let visible = [
            (ModuleId::Terminal, 0.68),
            (ModuleId::Git, 0.62),
            (ModuleId::Pipelines, 0.72),
        ]
        .into_iter()
        .find(|(id, _)| self.panel_available(*id) && self.module_open(*id));
        let panel = visible.map(|(id, _)| Panel::Module(id));
        if self.panes.iter().find_map(|(_, pane)| match pane {
            Panel::Module(_) => Some(*pane),
            Panel::Workspace => None,
        }) == panel
        {
            return;
        }
        let (mut panes, top) = pane_grid::State::new(Panel::Workspace);
        if let Some((id, ratio)) = visible
            && let Some((_, split)) =
                panes.split(pane_grid::Axis::Horizontal, top, Panel::Module(id))
        {
            panes.resize(split, ratio);
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

pub(crate) use crate::ui::presentation::error_key;

#[cfg(test)]
mod tests;

#[cfg(test)]
use crate::modules::git::git_panes;

#[cfg(test)]
use crate::domain::repository::Pipeline;
