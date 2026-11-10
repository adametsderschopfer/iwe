use super::{
    transfer::{self, Operation, Output, Request},
    transport::{self, Entry, Progress, Secret},
};
use crate::{
    domain::deployment::{
        Authentication, Connection, Connections, Error, Mapping, PrivateKey, Profile, Protocol,
    },
    i18n::Key,
};
use iced::{Subscription, Task};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Field {
    Name,
    Host,
    Port,
    Username,
    ProfileName,
    Root,
    KeyName,
    KeyPath,
    DraftName,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ProfileTab {
    #[default]
    Connection,
    Mappings,
    Exclusions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NameAction {
    NewConnection,
    NewProfile,
    DuplicateConnection,
    DuplicateProfile,
    RenameConnection,
    RenameProfile,
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    SaveText {
        profile: String,
        token: u64,
        path: String,
        expected: std::sync::Arc<String>,
        text: std::sync::Arc<String>,
    },
    Toggle,
    ToggleActivity,
    DismissActivity,
    Settings,
    SpaceSettings(usize),
    OpenConnections,
    OpenProfiles,
    ShowConnections,
    BeginName(NameAction),
    ConfirmName,
    NewProtocol(Protocol),
    DiscardChanges,
    RevertConnection,
    RevertProfile,
    ProfileTab(ProfileTab),
    TestProfile,
    OpenKeys,
    EditKey(String),
    NewConnection,
    EditConnection(String),
    Input(Field, String),
    Password(Secret),
    Protocol(Protocol),
    Authentication(Authentication),
    SaveConnection,
    ConnectionSaved(u64, Connection, Result<(), Error>),
    DeleteConnection,
    Test,
    NewKey,
    CloseDialog,
    CloseSettings,
    PickKey,
    KeyPicked(u64, Option<PathBuf>),
    AddKey,
    DeleteKey(String),
    NewProfile,
    EditProfile(String),
    ProfileConnection(String),
    AddMapping,
    MappingLocal(usize, String),
    MappingRemote(usize, String),
    RemoveMapping(usize),
    AddExclusion,
    Exclusion(usize, String),
    RemoveExclusion(usize),
    DefaultProfile(bool),
    SaveProfile,
    DeleteProfile,
    SelectProfile(String),
    BrowseRoot,
    StopBrowsing,
    UseRoot(String),
    Folder(String),
    Select(Entry),
    Menu(Entry, iced::Point),
    DismissMenu,
    Refresh,
    Run(Operation),
    CurrentFile(bool),
    Finished(u64, Result<Output, Error>),
    TrustHost,
    ConfirmDownload,
    Cancel,
    Retry,
    Tick,
    Resize(f32, bool),
}

pub(crate) enum Effect {
    EditorSaved(u64, Result<(), Error>),
    None,
    Save,
    Settings,
    Connections,
    Preview(Output),
    CompareRequested {
        profile: String,
        label: String,
        path: PathBuf,
        remote: String,
    },
    Transferred(String),
}

#[derive(Default)]
pub(crate) struct State {
    pub open: bool,
    pub activity_visible: bool,
    pub activity_expanded: bool,
    pub operation_label: Option<Key>,
    pub operation_path: String,
    pub profile_tab: ProfileTab,
    pub keys_open: bool,
    pub key_editing: Option<String>,
    pub naming: Option<NameAction>,
    pub name_value: String,
    pub new_protocol: Protocol,
    pub pending: Option<Box<Event>>,
    pub connection: Option<Connection>,
    pub secret: Secret,
    pub secret_changed: bool,
    pub port: String,
    pub key_dialog: bool,
    pub key_error: Option<Error>,
    key_request: u64,
    pub key_name: String,
    pub key_path: String,
    pub profile: Option<Profile>,
    pub selected_profile: Option<String>,
    pub tree: BTreeMap<String, Vec<Entry>>,
    pub expanded: BTreeSet<String>,
    pub selected: Option<Entry>,
    pub menu: Option<Entry>,
    pub menu_point: iced::Point,
    pub browsing: bool,
    pub error: Option<Error>,
    pub status: Option<Key>,
    pub busy: bool,
    pub saving: bool,
    pub progress: Progress,
    pub operation_name: String,
    current: Option<Request>,
    failed: Option<Request>,
    queue: VecDeque<Request>,
    request: u64,
    credential_request: u64,
    context: Option<(usize, PathBuf)>,
}

fn identifier() -> Result<String, Error> {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).map_err(|_| Error::Configuration)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

impl State {
    pub(crate) fn comparison_error(&self, profile: &str, path: &Path) -> Option<Error> {
        let matches = |request: &&Request| {
            request.profile.id == profile
                && matches!(&request.operation, Operation::Compare(local) if local == path)
        };
        if self
            .current
            .iter()
            .chain(self.queue.iter())
            .any(|request| matches(&request))
        {
            return None;
        }
        if self.failed.iter().any(|request| matches(&request)) {
            return self.error.clone().or(Some(Error::Cancelled));
        }
        Some(Error::Cancelled)
    }

    pub(crate) fn queued(&self) -> usize {
        self.queue.len()
    }

    pub(crate) fn has_dialog(&self) -> bool {
        self.key_dialog || self.keys_open || self.naming.is_some() || self.pending.is_some()
    }

    fn close_key_dialog(&mut self) {
        self.key_request += 1;
        self.key_dialog = false;
        self.key_editing = None;
        self.key_name.clear();
        self.key_path.clear();
        self.key_error = None;
    }

    fn close_connection(&mut self) -> Task<Event> {
        self.connection = None;
        self.secret = Secret::default();
        self.secret_changed = false;
        self.queue
            .retain(|request| !matches!(request.operation, Operation::Test));
        if self
            .current
            .as_ref()
            .is_some_and(|request| matches!(request.operation, Operation::Test))
        {
            self.progress.cancel.cancel();
            self.request += 1;
            self.busy = false;
            self.current = None;
        }
        if self
            .failed
            .as_ref()
            .is_some_and(|request| matches!(request.operation, Operation::Test))
        {
            self.failed = None;
        }
        self.error = None;
        if !self.busy
            && let Some(request) = self.queue.pop_front()
        {
            return self.start(request);
        }
        Task::none()
    }

    pub(crate) fn suspend(&mut self) {
        self.progress.cancel.cancel();
        let request = self.request + 1;
        let credential_request = self.credential_request + 1;
        let key_request = self.key_request + 1;
        *self = Self::default();
        self.request = request;
        self.credential_request = credential_request;
        self.key_request = key_request;
    }

    pub(crate) fn sync_context(&mut self, space: Option<usize>, project: Option<&Path>) {
        let context = space.zip(project.map(Path::to_path_buf));
        if self.context == context {
            return;
        }
        self.progress.cancel.cancel();
        self.request += 1;
        self.context = context;
        self.activity_visible = false;
        self.activity_expanded = false;
        self.busy = false;
        self.current = None;
        self.failed = None;
        self.queue.clear();
        self.tree.clear();
        self.expanded.clear();
        self.selected = None;
        self.selected_profile = None;
        self.profile = None;
        self.browsing = false;
        self.error = None;
        self.status = None;
        self.menu = None;
    }

    pub(crate) fn active_profile<'a>(
        &self,
        profiles: &'a [Profile],
        project: Option<&Path>,
    ) -> Option<&'a Profile> {
        let project = project?;
        profiles
            .iter()
            .find(|p| p.project == project && Some(&p.id) == self.selected_profile.as_ref())
            .or_else(|| profiles.iter().find(|p| p.project == project && p.default))
            .or_else(|| profiles.iter().find(|p| p.project == project))
    }

    pub(crate) fn forget_editor_save(&mut self, token: u64) {
        if self.failed.as_ref().is_some_and(|request| matches!(request.operation, Operation::SaveText { token: saved, .. } if saved == token)) {
            self.failed = None;
            self.error = None;
            self.activity_visible = false;
            self.activity_expanded = false;
        }
    }

    pub(crate) fn subscription(&self) -> Subscription<Event> {
        if self.busy {
            iced::time::every(std::time::Duration::from_millis(150)).map(|_| Event::Tick)
        } else {
            Subscription::none()
        }
    }

    fn start(&mut self, request: Request) -> Task<Event> {
        if self.busy {
            if self.queue.len() < 32 {
                self.queue.push_back(request);
            } else {
                self.error = Some(Error::TooLarge);
            }
            return Task::none();
        }
        self.request += 1;
        let id = self.request;
        self.busy = true;
        self.error = None;
        self.status = None;
        self.operation_name = if request.profile.name.is_empty() {
            request.connection.name.clone()
        } else {
            request.profile.name.clone()
        };
        self.progress = Progress::default();
        let progress = self.progress.clone();
        self.activity_visible = true;
        self.operation_label = Some(match &request.operation {
            Operation::Test => Key::DeploymentTest,
            Operation::List(_) => Key::DeploymentRefresh,
            Operation::Preview(_) => Key::DeploymentLoadingRemote,
            Operation::Compare(_) => Key::DeploymentCompare,
            Operation::Upload(_) | Operation::SaveText { .. } => Key::DeploymentUpload,
            Operation::Download(_, _) => Key::DeploymentDownload,
        });
        self.operation_path = match &request.operation {
            Operation::Compare(path) | Operation::Upload(path) => {
                path.to_string_lossy().into_owned()
            }
            Operation::List(path) | Operation::Preview(path) | Operation::Download(path, _) => {
                path.clone()
            }
            Operation::SaveText { path, .. } => path.clone(),
            Operation::Test => String::new(),
        };
        self.current = Some(request.clone());
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || transfer::execute(request, &progress))
                    .await
                    .unwrap_or(Err(Error::Connection))
            },
            move |result| Event::Finished(id, result),
        )
    }

    fn request(
        &self,
        settings: &Connections,
        profiles: &[Profile],
        project: Option<&Path>,
        operation: Operation,
    ) -> Result<Request, Error> {
        let browsing = self.browsing && matches!(operation, Operation::List(_));
        let profile = if browsing {
            self.profile.as_ref()
        } else {
            self.active_profile(profiles, project)
        }
        .ok_or(Error::Configuration)?
        .clone();
        let connection = settings
            .connections
            .iter()
            .find(|c| c.id == profile.connection)
            .ok_or(Error::Configuration)?
            .clone();
        let mut profile = profile;
        if browsing {
            profile.root = "/".to_owned();
        }
        Ok(Request {
            profile,
            connection,
            settings: settings.clone(),
            operation,
            secret: None,
        })
    }

    pub(crate) fn update(
        &mut self,
        event: Event,
        settings: &mut Connections,
        profiles: &mut Vec<Profile>,
        project: Option<&Path>,
    ) -> (Task<Event>, Effect) {
        let mut effect = Effect::None;
        if self.guard_selection(&event, settings, profiles) {
            self.pending = Some(Box::new(event));
            return (Task::none(), effect);
        }
        match event {
            Event::OpenConnections if self.connection.is_none() => {
                if let Some(connection) = settings.connections.first() {
                    return self.update(
                        Event::EditConnection(connection.id.clone()),
                        settings,
                        profiles,
                        project,
                    );
                }
            }
            Event::OpenProfiles if self.profile.is_none() => {
                if let Some(profile) = self
                    .active_profile(profiles, project)
                    .or_else(|| profiles.first())
                {
                    return self.update(
                        Event::EditProfile(profile.id.clone()),
                        settings,
                        profiles,
                        project,
                    );
                }
            }
            Event::ShowConnections => return (Task::none(), Effect::Connections),
            Event::BeginName(action) => {
                self.name_value = match action {
                    NameAction::NewConnection | NameAction::NewProfile => String::new(),
                    NameAction::DuplicateConnection | NameAction::RenameConnection => self
                        .connection
                        .as_ref()
                        .map(|c| c.name.clone())
                        .unwrap_or_default(),
                    NameAction::DuplicateProfile | NameAction::RenameProfile => self
                        .profile
                        .as_ref()
                        .map(|p| p.name.clone())
                        .unwrap_or_default(),
                };
                self.new_protocol = Protocol::Sftp;
                self.naming = Some(action);
            }
            Event::NewProtocol(protocol) => self.new_protocol = protocol,
            Event::ConfirmName => return self.confirm_name(settings, profiles, project),
            Event::DiscardChanges => {
                if let Some(event) = self.pending.take() {
                    match event.as_ref() {
                        Event::EditConnection(_) | Event::BeginName(NameAction::NewConnection) => {
                            let cleanup = self.close_connection();
                            let (next, effect) = self.update(*event, settings, profiles, project);
                            return (Task::batch([cleanup, next]), effect);
                        }
                        _ => self.profile = None,
                    }
                    return self.update(*event, settings, profiles, project);
                }
            }
            Event::RevertConnection if !self.saving => {
                let id = self.connection.as_ref().map(|c| c.id.clone());
                let cleanup = self.close_connection();
                if let Some(id) = id {
                    let (select, _) =
                        self.update(Event::EditConnection(id), settings, profiles, project);
                    return (Task::batch([cleanup, select]), effect);
                }
            }
            Event::RevertProfile => {
                let id = self.profile.take().map(|p| p.id);
                if let Some(id) = id {
                    return self.update(Event::EditProfile(id), settings, profiles, project);
                }
            }
            Event::ProfileTab(tab) => self.profile_tab = tab,
            Event::OpenKeys => self.keys_open = true,
            Event::EditKey(id) => {
                if let Some(key) = settings.keys.iter().find(|k| k.id == id) {
                    self.key_request += 1;
                    self.key_dialog = true;
                    self.key_editing = Some(id);
                    self.key_name = key.name.clone();
                    self.key_path = key.path.to_string_lossy().into_owned();
                    self.key_error = None;
                }
            }
            Event::TestProfile => {
                if let Some(profile) = &self.profile
                    && let Some(connection) = settings
                        .connections
                        .iter()
                        .find(|c| c.id == profile.connection)
                {
                    let request = Request {
                        connection: connection.clone(),
                        settings: settings.clone(),
                        profile: profile.clone(),
                        operation: Operation::Test,
                        secret: None,
                    };
                    return (self.start(request), effect);
                }
                self.error = Some(Error::Configuration);
            }
            Event::ToggleActivity => self.activity_expanded = !self.activity_expanded,
            Event::DismissActivity => {
                self.activity_expanded = false;
                if !self.busy {
                    self.activity_visible = false;
                }
            }
            Event::Toggle => {
                self.open = !self.open;
                if self.open && self.tree.is_empty() {
                    return self.update(Event::Refresh, settings, profiles, project);
                }
            }
            Event::Settings => return (Task::none(), Effect::Settings),
            Event::NewConnection => match identifier() {
                Ok(id) => {
                    self.connection = Some(Connection {
                        id,
                        port: 22,
                        ..Connection::default()
                    });
                    self.port = "22".to_owned();
                    self.secret = Secret::default();
                    self.secret_changed = false;
                    self.error = None;
                }
                Err(error) => self.error = Some(error),
            },
            Event::EditConnection(id) => {
                if self.connection.as_ref().is_some_and(|c| c.id == id) {
                    return (Task::none(), effect);
                }
                self.connection = settings.connections.iter().find(|c| c.id == id).cloned();
                self.port = self
                    .connection
                    .as_ref()
                    .map(|c| c.port.to_string())
                    .unwrap_or_default();
                self.secret = Secret::default();
                self.secret_changed = false;
                self.error = None;
            }
            Event::Input(field, value) => match field {
                Field::DraftName => self.name_value = value,
                Field::KeyName => self.key_name = value,
                Field::KeyPath => self.key_path = value,
                Field::ProfileName => {
                    if let Some(profile) = &mut self.profile {
                        profile.name = value;
                    }
                }
                Field::Root => {
                    if let Some(profile) = &mut self.profile {
                        profile.root = value;
                    }
                }
                _ => {
                    if let Some(connection) = &mut self.connection {
                        match field {
                            Field::Name => connection.name = value,
                            Field::Host => {
                                if connection.host != value {
                                    connection.trusted_host_key = None;
                                }
                                connection.host = value;
                            }
                            Field::Port => {
                                self.port = value.clone();
                                connection.port = value.parse().unwrap_or(0);
                                connection.trusted_host_key = None;
                            }
                            Field::Username => connection.username = value,
                            _ => {}
                        }
                    }
                }
            },
            Event::Password(secret) => {
                self.secret = secret;
                self.secret_changed = true;
            }
            Event::Protocol(protocol) => {
                if let Some(connection) = &mut self.connection {
                    connection.protocol = protocol;
                    connection.port = protocol.port();
                    self.port = connection.port.to_string();
                    if protocol != Protocol::Sftp {
                        connection.authentication = Authentication::Password;
                    }
                    connection.trusted_host_key = None;
                }
            }
            Event::Authentication(authentication) => {
                if let Some(connection) = &mut self.connection {
                    connection.authentication = authentication;
                    self.secret = Secret::default();
                    self.secret_changed = false;
                }
            }
            Event::SaveConnection if !self.saving => {
                if let Some(connection) = self.connection.clone() {
                    let validation = connection.validate().and_then(|()| {
                        if let Authentication::Key(id) = &connection.authentication
                            && !settings.keys.iter().any(|key| &key.id == id)
                        {
                            return Err(Error::Configuration);
                        }
                        Ok(())
                    });
                    if let Err(error) = validation {
                        self.error = Some(error);
                    } else {
                        self.credential_request += 1;
                        let id = self.credential_request;
                        let secret = self.secret.clone();
                        let changed = self.secret_changed;
                        self.saving = true;
                        let result_connection = connection.clone();
                        return (
                            Task::perform(
                                async move {
                                    if changed {
                                        tokio::task::spawn_blocking(move || {
                                            transport::store_secret(
                                                transport::secret_id(&connection),
                                                &secret,
                                            )
                                        })
                                        .await
                                        .unwrap_or(Err(Error::Credentials))
                                    } else {
                                        Ok(())
                                    }
                                },
                                move |result| {
                                    Event::ConnectionSaved(id, result_connection.clone(), result)
                                },
                            ),
                            Effect::None,
                        );
                    }
                }
            }
            Event::ConnectionSaved(id, connection, result) if id == self.credential_request => {
                self.saving = false;
                match result {
                    Ok(()) => {
                        self.connection = Some(connection.clone());
                        if let Some(existing) = settings
                            .connections
                            .iter_mut()
                            .find(|c| c.id == connection.id)
                        {
                            *existing = connection;
                        } else {
                            settings.connections.push(connection);
                        }
                        self.secret = Secret::default();
                        self.secret_changed = false;
                        self.status = Some(Key::DeploymentSaved);
                        self.error = None;
                        effect = Effect::Save;
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            Event::DeleteConnection if !self.saving && !self.busy => {
                if let Some(connection) = self.connection.take() {
                    settings.connections.retain(|c| c.id != connection.id);
                    self.secret = Secret::default();
                    effect = Effect::Save;
                }
            }
            Event::NewKey => {
                self.key_request += 1;
                self.key_dialog = true;
                self.key_name.clear();
                self.key_path.clear();
                self.key_error = None;
            }
            Event::CloseDialog => {
                if self.pending.take().is_some() || self.naming.take().is_some() {
                    self.name_value.clear();
                } else if self.key_dialog {
                    self.close_key_dialog();
                } else if self.keys_open {
                    self.keys_open = false;
                } else if !self.saving {
                    return (self.close_connection(), effect);
                }
            }
            Event::CloseSettings => {
                self.close_key_dialog();
                self.keys_open = false;
                self.naming = None;
                self.pending = None;
                let (cleanup, effect) =
                    self.update(Event::StopBrowsing, settings, profiles, project);
                let connection = if self.saving {
                    Task::none()
                } else {
                    self.close_connection()
                };
                return (Task::batch([cleanup, connection]), effect);
            }
            Event::PickKey if self.key_dialog => {
                self.key_request += 1;
                let request = self.key_request;
                return (
                    Task::perform(
                        async {
                            rfd::AsyncFileDialog::new()
                                .pick_file()
                                .await
                                .map(|file| file.path().to_path_buf())
                        },
                        move |path| Event::KeyPicked(request, path),
                    ),
                    effect,
                );
            }
            Event::KeyPicked(request, Some(path))
                if self.key_dialog && request == self.key_request =>
            {
                self.key_path = path.to_string_lossy().into_owned();
                self.key_error = None;
            }
            Event::AddKey if self.key_dialog => {
                if self.key_name.trim().is_empty() || !Path::new(&self.key_path).is_absolute() {
                    self.key_error = Some(Error::Configuration);
                } else {
                    match self.key_editing.clone().map_or_else(identifier, Ok) {
                        Ok(id) => {
                            settings.keys.retain(|key| key.id != id);
                            settings.keys.push(PrivateKey {
                                id: id.clone(),
                                name: self.key_name.trim().to_owned(),
                                path: PathBuf::from(&self.key_path),
                            });
                            if !self.keys_open
                                && let Some(connection) = &mut self.connection
                            {
                                connection.authentication = Authentication::Key(id);
                                self.secret = Secret::default();
                                self.secret_changed = false;
                            }
                            self.close_key_dialog();
                            effect = Effect::Save;
                        }
                        Err(error) => self.key_error = Some(error),
                    }
                }
            }
            Event::DeleteKey(id) => {
                if settings
                    .connections
                    .iter()
                    .any(|c| c.authentication == Authentication::Key(id.clone()))
                {
                    self.error = Some(Error::Configuration);
                } else {
                    settings.keys.retain(|key| key.id != id);
                    effect = Effect::Save;
                }
            }
            Event::Test => {
                if let Some(connection) = self.connection.clone() {
                    let request = Request {
                        connection,
                        settings: settings.clone(),
                        profile: Profile::default(),
                        operation: Operation::Test,
                        secret: self.secret_changed.then(|| self.secret.clone()),
                    };
                    return (self.start(request), effect);
                }
            }
            Event::NewProfile => {
                self.profile_tab = ProfileTab::Connection;
                if let (Some(project), Ok(id)) = (project, identifier()) {
                    self.profile = Some(Profile {
                        id,
                        project: project.to_owned(),
                        root: "/".to_owned(),
                        connection: settings
                            .connections
                            .first()
                            .map(|c| c.id.clone())
                            .unwrap_or_default(),
                        mappings: vec![Mapping::default()],
                        default: !profiles.iter().any(|p| p.project == project),
                        ..Profile::default()
                    });
                    self.error = None;
                }
            }
            Event::EditProfile(id) => {
                if self.profile.as_ref().is_some_and(|p| p.id == id) {
                    return (Task::none(), effect);
                }
                let _ = self.update(Event::StopBrowsing, settings, profiles, project);
                self.profile_tab = ProfileTab::Connection;
                self.profile = profiles.iter().find(|p| p.id == id).cloned();
                self.error = None;
            }
            Event::ProfileConnection(id) => {
                if let Some(profile) = &mut self.profile {
                    profile.connection = id;
                }
            }
            Event::AddMapping => {
                if let Some(profile) = &mut self.profile {
                    profile.mappings.push(Mapping::default());
                }
            }
            Event::MappingLocal(index, value) => {
                if let Some(mapping) = self
                    .profile
                    .as_mut()
                    .and_then(|p| p.mappings.get_mut(index))
                {
                    mapping.local = PathBuf::from(value);
                }
            }
            Event::MappingRemote(index, value) => {
                if let Some(mapping) = self
                    .profile
                    .as_mut()
                    .and_then(|p| p.mappings.get_mut(index))
                {
                    mapping.remote = value;
                }
            }
            Event::RemoveMapping(index) => {
                if let Some(profile) = &mut self.profile
                    && index < profile.mappings.len()
                {
                    profile.mappings.remove(index);
                }
            }
            Event::AddExclusion => {
                if let Some(profile) = &mut self.profile {
                    profile.excluded.push(PathBuf::new());
                }
            }
            Event::Exclusion(index, value) => {
                if let Some(path) = self
                    .profile
                    .as_mut()
                    .and_then(|p| p.excluded.get_mut(index))
                {
                    *path = PathBuf::from(value);
                }
            }
            Event::RemoveExclusion(index) => {
                if let Some(profile) = &mut self.profile
                    && index < profile.excluded.len()
                {
                    profile.excluded.remove(index);
                }
            }
            Event::DefaultProfile(value) => {
                if let Some(profile) = &mut self.profile {
                    profile.default = value;
                }
            }
            Event::SaveProfile => {
                if let Some(mut profile) = self.profile.clone() {
                    profile.excluded.retain(|p| !p.as_os_str().is_empty());
                    match profile.validate() {
                        Ok(())
                            if settings
                                .connections
                                .iter()
                                .any(|c| c.id == profile.connection) =>
                        {
                            if profile.default {
                                for existing in
                                    profiles.iter_mut().filter(|p| p.project == profile.project)
                                {
                                    existing.default = false;
                                }
                            }
                            profiles.retain(|p| p.id != profile.id);
                            profiles.push(profile.clone());
                            self.profile = Some(profile);
                            self.status = Some(Key::DeploymentSaved);
                            self.error = None;
                            self.browsing = false;
                            self.tree.clear();
                            self.expanded.clear();
                            effect = Effect::Save;
                        }
                        Err(error) => self.error = Some(error),
                        _ => self.error = Some(Error::Configuration),
                    }
                }
            }
            Event::DeleteProfile if !self.busy => {
                if let Some(profile) = self.profile.take() {
                    profiles.retain(|p| p.id != profile.id);
                    self.tree.clear();
                    self.expanded.clear();
                    effect = Effect::Save;
                }
            }
            Event::SelectProfile(id) if !self.busy => {
                self.selected_profile = Some(id.clone());
                self.tree.clear();
                self.expanded.clear();
                self.selected = None;
                self.browsing = false;
                if let Some(selected) = profiles.iter().find(|p| p.id == id).cloned() {
                    for p in profiles
                        .iter_mut()
                        .filter(|p| p.project == selected.project)
                    {
                        p.default = p.id == id;
                    }
                }
                let (task, _) = self.update(Event::Refresh, settings, profiles, project);
                return (task, Effect::Save);
            }
            Event::BrowseRoot => {
                self.browsing = true;
                self.tree.clear();
                self.expanded.clear();
                return self.update(
                    Event::Run(Operation::List("/".to_owned())),
                    settings,
                    profiles,
                    project,
                );
            }
            Event::StopBrowsing => {
                if self.browsing {
                    self.progress.cancel.cancel();
                    self.request += 1;
                    self.busy = false;
                    self.current = None;
                    self.failed = None;
                    self.queue.clear();
                    self.tree.clear();
                    self.expanded.clear();
                    self.selected = None;
                    self.menu = None;
                    self.browsing = false;
                }
            }
            Event::UseRoot(root) => {
                if let Some(profile) = &mut self.profile {
                    profile.root = root;
                }
                self.browsing = false;
                self.tree.clear();
                self.expanded.clear();
            }
            Event::Select(entry) => self.selected = Some(entry),
            Event::Menu(entry, point) => {
                self.selected = Some(entry.clone());
                self.menu = Some(entry);
                self.menu_point = point;
            }
            Event::DismissMenu => self.menu = None,
            Event::Folder(path) => {
                if self.expanded.remove(&path) {
                    return (Task::none(), effect);
                }
                self.expanded.insert(path.clone());
                if !self.tree.contains_key(&path) {
                    return self.update(
                        Event::Run(Operation::List(path)),
                        settings,
                        profiles,
                        project,
                    );
                }
            }
            Event::Refresh => {
                let root = if self.browsing {
                    Some("/".to_owned())
                } else {
                    self.active_profile(profiles, project)
                        .map(|p| p.root.clone())
                };
                if let Some(root) = root {
                    return self.update(
                        Event::Run(Operation::List(root)),
                        settings,
                        profiles,
                        project,
                    );
                }
            }
            Event::SaveText {
                profile,
                token,
                path,
                expected,
                text,
            } => {
                if self.busy {
                    return (
                        Task::none(),
                        Effect::EditorSaved(token, Err(Error::Connection)),
                    );
                }
                let Some(profile) = profiles
                    .iter()
                    .find(|candidate| {
                        candidate.id == profile && Some(candidate.project.as_path()) == project
                    })
                    .cloned()
                else {
                    return (
                        Task::none(),
                        Effect::EditorSaved(token, Err(Error::Configuration)),
                    );
                };
                let Some(connection) = settings
                    .connections
                    .iter()
                    .find(|connection| connection.id == profile.connection)
                    .cloned()
                else {
                    return (
                        Task::none(),
                        Effect::EditorSaved(token, Err(Error::Configuration)),
                    );
                };
                return (
                    self.start(Request {
                        connection,
                        settings: settings.clone(),
                        profile,
                        operation: Operation::SaveText {
                            token,
                            path,
                            expected,
                            text,
                        },
                        secret: None,
                    }),
                    Effect::None,
                );
            }
            Event::Run(operation) => {
                self.menu = None;
                match self.request(settings, profiles, project, operation) {
                    Ok(request) => {
                        if let Operation::Compare(path) = &request.operation {
                            let remote = match request.profile.to_remote(path) {
                                Ok(remote) => remote,
                                Err(error) => {
                                    self.error = Some(error);
                                    return (Task::none(), Effect::Settings);
                                }
                            };
                            effect = Effect::CompareRequested {
                                profile: request.profile.id.clone(),
                                label: request.profile.name.clone(),
                                path: path.clone(),
                                remote,
                            };
                            if self
                                .current
                                .iter()
                                .chain(self.queue.iter())
                                .any(|existing| {
                                    existing.profile.id == request.profile.id
                                        && existing.operation == request.operation
                                })
                            {
                                return (Task::none(), effect);
                            }
                            if self.busy && self.queue.len() >= 32 {
                                self.error = Some(Error::TooLarge);
                                self.activity_expanded = true;
                                return (Task::none(), Effect::None);
                            }
                        }
                        return (self.start(request), effect);
                    }
                    Err(error) => {
                        self.error = Some(error);
                        return (Task::none(), Effect::Settings);
                    }
                }
            }
            Event::Finished(id, result) if id == self.request => {
                self.busy = false;
                match result {
                    Ok(output) => {
                        self.error = None;
                        self.failed = None;
                        self.status = Some(Key::DeploymentComplete);
                        match output {
                            Output::SavedText(token) => effect = Effect::EditorSaved(token, Ok(())),
                            Output::Connected => self.status = Some(Key::DeploymentConnected),
                            Output::Transferred => {
                                self.status = Some(Key::DeploymentComplete);
                                effect = Effect::Transferred(self.operation_name.clone());
                            }
                            Output::Listed(path, entries) => {
                                self.expanded.insert(path.clone());
                                self.tree.insert(path, entries);
                            }
                            output @ Output::Preview {
                                previous: Some(_), ..
                            } => {
                                self.status = Some(Key::DeploymentComparisonReady);
                                effect = Effect::Preview(output);
                            }
                            output @ Output::Preview { .. } => effect = Effect::Preview(output),
                        }
                        self.current = None;
                        if let Some(next) = self.queue.pop_front() {
                            return (self.start(next), effect);
                        }
                    }
                    Err(error) => {
                        self.activity_expanded = true;
                        self.error = Some(error.clone());
                        if let Some(Request {
                            operation: Operation::SaveText { token, .. },
                            ..
                        }) = &self.current
                        {
                            effect = Effect::EditorSaved(*token, Err(error.clone()));
                        }
                        self.failed = self.current.take();
                        self.queue.clear();
                    }
                }
            }
            Event::TrustHost => {
                if let (Some(Error::HostUnknown(key)), Some(mut request)) =
                    (self.error.clone(), self.failed.take())
                {
                    request.connection.trusted_host_key = Some(key.clone());
                    if let Some(connection) = settings.connections.iter_mut().find(|c| {
                        c.id == request.connection.id
                            && c.host == request.connection.host
                            && c.port == request.connection.port
                    }) {
                        connection.trusted_host_key = Some(key.clone());
                        effect = Effect::Save;
                    }
                    if let Some(connection) = self.connection.as_mut().filter(|c| {
                        c.id == request.connection.id
                            && c.host == request.connection.host
                            && c.port == request.connection.port
                    }) {
                        connection.trusted_host_key = Some(key);
                    }
                    return (self.start(request), effect);
                }
            }
            Event::ConfirmDownload => {
                if self.error == Some(Error::Overwrite)
                    && let Some(mut request) = self.failed.take()
                    && let Operation::Download(_, overwrite) = &mut request.operation
                {
                    *overwrite = true;
                    return (self.start(request), effect);
                }
            }
            Event::Cancel => {
                self.progress.cancel.cancel();
                self.queue.clear();
                self.failed = None;
                self.error = None;
            }
            Event::Retry => {
                if let Some(request) = self.failed.take() {
                    return (self.start(request), effect);
                }
            }
            _ => {}
        }
        (Task::none(), effect)
    }
}

impl Drop for State {
    fn drop(&mut self) {
        self.progress.cancel.cancel();
    }
}

#[cfg(test)]
mod tests;

mod settings;
