//! Host external module sessions and route their events.

use crate::{
    domain::modules::{InstalledModule, Manifest, ModuleRegistry, Placement, WidgetAction},
    i18n::Key,
    infrastructure::modules::{Error, install, package, protocol, registry, runtime},
};
use iced::{Subscription, Task};
use std::{
    collections::BTreeMap,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Arc,
};

/// Crash restarts allowed before the application disables a module.
const MAX_RESTARTS: u32 = 3;

#[derive(Default)]
pub(crate) struct State {
    pub registry: ModuleRegistry,
    /// Installation awaiting the permission confirmation.
    pub confirm: Option<Pending>,
    pub installing: bool,
    pub(crate) sessions: BTreeMap<String, SessionState>,
    /// Active module identifier per workspace zone.
    pub active: BTreeMap<Placement, String>,
    restarts: BTreeMap<String, u32>,
}

pub(crate) struct Pending {
    pub path: PathBuf,
    pub manifest: Manifest,
}

pub(crate) struct SessionState {
    pub request: u64,
    pub session: Option<Arc<runtime::Session>>,
    pub panel: Option<crate::domain::modules::WidgetNode>,
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Loaded(Result<ModuleRegistry, Error>),
    Pick,
    Picked(Option<PathBuf>),
    ManifestRead(Box<Result<(PathBuf, Manifest), Error>>),
    Confirm,
    Cancel,
    Installed(Result<(ModuleRegistry, String), Error>),
    Remove(String),
    Removed(String, Result<ModuleRegistry, Error>),
    SetEnabled(String, bool),
    Output(String, protocol::ModuleMessage),
    Widget(String, String, WidgetAction),
    SelectZone(Placement, String),
}

pub(crate) fn startup() -> Task<Event> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(registry::load)
                .await
                .map_err(|_| Error::Storage)?
        },
        Event::Loaded,
    )
}

async fn pick_package() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .add_filter("IWE module", &["iwe-module"])
        .pick_file()
        .await
        .map(|handle| handle.path().to_owned())
}

async fn read_manifest(path: PathBuf) -> Result<(PathBuf, Manifest), Error> {
    tokio::task::spawn_blocking(move || package::manifest(&path).map(|manifest| (path, manifest)))
        .await
        .map_err(|_| Error::Storage)?
}

async fn install_package(
    path: PathBuf,
    current: ModuleRegistry,
) -> Result<(ModuleRegistry, String), Error> {
    tokio::task::spawn_blocking(move || install::install(&path, &current))
        .await
        .map_err(|_| Error::Storage)?
}

async fn remove_module(id: String, current: ModuleRegistry) -> Result<ModuleRegistry, Error> {
    tokio::task::spawn_blocking(move || install::uninstall(&id, &current))
        .await
        .map_err(|_| Error::Storage)?
}

struct Stream {
    id: String,
    request: u64,
    session: Arc<runtime::Session>,
}

impl Hash for Stream {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.request.hash(state);
    }
}

pub(crate) fn message(event: Event) -> crate::app::Message {
    crate::app::Message::Module(crate::app::modules::Event::External(event))
}

impl crate::app::App {
    pub(super) fn update_external(&mut self, event: Event) -> Task<crate::app::Message> {
        match event {
            Event::Loaded(Ok(modules)) => {
                self.modules.external.registry = modules;
                for module in self.modules.external.registry.modules.clone() {
                    if self.external_enabled(&module.manifest.id) {
                        self.external_start(&module.manifest);
                    }
                }
                self.external_sync_zones();
            }
            Event::Loaded(Err(_)) => {
                self.notifications
                    .error(Key::ModuleRegistryLoadFailed, None);
            }
            Event::Pick => {
                self.modules.external.installing = true;
                return Task::perform(pick_package(), |path| message(Event::Picked(path)));
            }
            Event::Picked(Some(path)) => {
                return Task::perform(read_manifest(path), |result| {
                    message(Event::ManifestRead(Box::new(result)))
                });
            }
            Event::Picked(None) => self.modules.external.installing = false,
            Event::ManifestRead(boxed) => match *boxed {
                Ok((path, manifest)) => {
                    self.modules.external.installing = false;
                    self.modules.external.confirm = Some(Pending { path, manifest });
                }
                Err(_) => {
                    self.modules.external.installing = false;
                    self.notifications.error(Key::ModuleInstallFailed, None);
                }
            },
            Event::Confirm => {
                let Some(pending) = self.modules.external.confirm.take() else {
                    return Task::none();
                };
                self.modules.external.installing = true;
                let current = self.modules.external.registry.clone();
                return Task::perform(install_package(pending.path, current), |result| {
                    message(Event::Installed(result))
                });
            }
            Event::Cancel => {
                self.modules.external.confirm = None;
                self.modules.external.installing = false;
            }
            Event::Installed(Ok((modules, id))) => {
                self.modules.external.installing = false;
                self.modules.external.registry = modules;
                self.workspace.preferences.disabled_modules.remove(&id);
                self.external_sync_zones();
                let module = self.modules.external.registry.get(&id).cloned();
                if let Some(module) = &module {
                    self.external_start(&module.manifest);
                }
                let name = module
                    .map(|module| {
                        module
                            .manifest
                            .name
                            .resolve(self.i18n.language().code())
                            .to_owned()
                    })
                    .unwrap_or_default();
                self.notifications.success(Key::ModuleInstalled, Some(name));
                return self.save();
            }
            Event::Installed(Err(error)) => {
                self.modules.external.installing = false;
                self.notifications.error(external_error_key(error), None);
            }
            Event::Remove(id) => {
                self.external_stop(&id);
                let current = self.modules.external.registry.clone();
                return Task::perform(remove_module(id.clone(), current), move |result| {
                    message(Event::Removed(id.clone(), result))
                });
            }
            Event::Removed(id, Ok(modules)) => {
                self.modules.external.registry = modules;
                self.modules.external.sessions.remove(&id);
                self.modules.external.restarts.remove(&id);
                self.workspace.preferences.disabled_modules.remove(&id);
                self.external_sync_zones();
                return self.save();
            }
            Event::Removed(_, Err(_)) => {
                self.notifications.error(Key::ModuleRemoveFailed, None);
            }
            Event::SetEnabled(id, enabled) => {
                if self.external_enabled(&id) == enabled {
                    return Task::none();
                }
                if enabled {
                    self.workspace.preferences.disabled_modules.remove(&id);
                    if let Some(module) = self.modules.external.registry.get(&id) {
                        let manifest = module.manifest.clone();
                        self.external_start(&manifest);
                    }
                } else {
                    self.workspace
                        .preferences
                        .disabled_modules
                        .insert(id.clone());
                    self.external_stop(&id);
                }
                self.external_sync_zones();
                return self.save();
            }
            Event::Output(id, output) => return self.external_output(id, output),
            Event::Widget(module, widget, action) => {
                if let Some(state) = self.modules.external.sessions.get(&module)
                    && let Some(session) = &state.session
                {
                    session.send(&protocol::HostMessage::Event { id: widget, action });
                }
            }
            Event::SelectZone(zone, id) => {
                self.modules.external.active.insert(zone, id);
            }
        }
        Task::none()
    }

    pub(crate) fn external_subscription(&self) -> Subscription<crate::app::Message> {
        Subscription::batch(
            self.modules
                .external
                .sessions
                .iter()
                .filter_map(|(id, state)| {
                    let session = state.session.as_ref()?;
                    Some(Subscription::run_with(
                        Stream {
                            id: id.clone(),
                            request: state.request,
                            session: Arc::clone(session),
                        },
                        |data| {
                            iced::futures::stream::unfold(
                                (data.id.clone(), Some(Arc::clone(&data.session))),
                                |(id, session)| async move {
                                    let session = session?;
                                    let output = session.next().await?;
                                    Some((
                                        message(Event::Output(id.clone(), output)),
                                        (id, Some(session)),
                                    ))
                                },
                            )
                        },
                    ))
                }),
        )
    }

    pub(crate) fn external_enabled(&self, id: &str) -> bool {
        !self.workspace.preferences.disabled_modules.contains(id)
    }

    /// Modules placed in one zone and currently enabled.
    pub(crate) fn external_zone_modules(&self, zone: Placement) -> Vec<&InstalledModule> {
        self.modules
            .external
            .registry
            .modules
            .iter()
            .filter(|module| {
                module.manifest.placement == zone && self.external_enabled(&module.manifest.id)
            })
            .collect()
    }

    fn external_start(&mut self, manifest: &Manifest) {
        if self.modules.external.sessions.contains_key(&manifest.id) {
            return;
        }
        let Ok(executable) = install::executable(manifest) else {
            self.notifications.error(Key::ModuleStartFailed, None);
            return;
        };
        let directory = crate::infrastructure::modules::module_directory(
            &manifest.id,
            &manifest.version.to_string(),
        );
        let install_path = directory
            .ok()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default();
        let initialize =
            protocol::HostMessage::initialize(self.i18n.language().code(), &install_path);
        match runtime::Session::start(&executable, &initialize) {
            Ok(session) => {
                self.modules.external.sessions.insert(
                    manifest.id.clone(),
                    SessionState {
                        request: 0,
                        session: Some(session),
                        panel: None,
                    },
                );
            }
            Err(()) => self.notifications.error(Key::ModuleStartFailed, None),
        }
    }

    fn external_stop(&mut self, id: &str) {
        if let Some(state) = self.modules.external.sessions.get_mut(id) {
            if let Some(session) = state.session.take() {
                session.shutdown();
            }
            state.panel = None;
            state.request += 1;
        }
    }

    fn external_output(
        &mut self,
        id: String,
        output: protocol::ModuleMessage,
    ) -> Task<crate::app::Message> {
        match output {
            protocol::ModuleMessage::Panel(panel) => {
                if let Some(state) = self.modules.external.sessions.get_mut(&id) {
                    state.panel = Some(panel);
                }
            }
            protocol::ModuleMessage::Notify { level, message } => match level {
                protocol::NotifyLevel::Error => {
                    self.notifications.error(Key::ModuleMessage, Some(message))
                }
                protocol::NotifyLevel::Info | protocol::NotifyLevel::Warning => self
                    .notifications
                    .success(Key::ModuleMessage, Some(message)),
            },
            protocol::ModuleMessage::Exited => {
                if let Some(state) = self.modules.external.sessions.get_mut(&id) {
                    state.session = None;
                }
                if !self.external_enabled(&id) {
                    return Task::none();
                }
                let restarts = self
                    .modules
                    .external
                    .restarts
                    .entry(id.clone())
                    .or_insert(0);
                if *restarts < MAX_RESTARTS {
                    *restarts += 1;
                    let manifest = self
                        .modules
                        .external
                        .registry
                        .get(&id)
                        .map(|module| module.manifest.clone());
                    if let Some(manifest) = &manifest {
                        self.external_start(manifest);
                    }
                } else {
                    if let Some(state) = self.modules.external.sessions.get_mut(&id) {
                        state.panel = None;
                    }
                    self.workspace
                        .preferences
                        .disabled_modules
                        .insert(id.clone());
                    self.external_sync_zones();
                    self.notifications.error(Key::ModuleCrashed, None);
                    return self.save();
                }
            }
        }
        Task::none()
    }

    fn external_sync_zones(&mut self) {
        for zone in Placement::ALL {
            let modules = self.external_zone_modules(zone);
            let active = self.modules.external.active.get(&zone);
            let valid =
                active.is_some_and(|id| modules.iter().any(|module| &module.manifest.id == id));
            if valid {
                continue;
            }
            match modules.first() {
                Some(module) => {
                    self.modules
                        .external
                        .active
                        .insert(zone, module.manifest.id.clone());
                }
                None => {
                    self.modules.external.active.remove(&zone);
                }
            }
        }
    }
}

fn external_error_key(error: Error) -> Key {
    match error {
        Error::TooLarge => Key::ModulePackageTooLarge,
        Error::UnsupportedTarget => Key::ModuleUnsupportedTarget,
        Error::Incompatible => Key::ModuleIncompatible,
        Error::OlderVersion => Key::ModuleOlderVersion,
        Error::Invalid | Error::Storage => Key::ModuleInstallFailed,
    }
}

#[cfg(test)]
mod tests;
