//! Keep editor drafts and small settings dialogs independent of persisted connections.

use super::*;
use crate::domain::deployment::safe_text;

impl State {
    pub(crate) fn connection_dirty(&self, settings: &Connections) -> bool {
        self.connection.as_ref().is_some_and(|draft| {
            self.secret_changed
                || settings.connections.iter().find(|c| c.id == draft.id) != Some(draft)
        })
    }

    pub(crate) fn profile_dirty(&self, profiles: &[Profile]) -> bool {
        self.profile
            .as_ref()
            .is_some_and(|draft| profiles.iter().find(|p| p.id == draft.id) != Some(draft))
    }

    pub(super) fn guard_selection(
        &self,
        event: &Event,
        settings: &Connections,
        profiles: &[Profile],
    ) -> bool {
        match event {
            Event::EditConnection(id) => {
                self.connection.as_ref().is_some_and(|c| &c.id != id)
                    && self.connection_dirty(settings)
            }
            Event::EditProfile(id) => {
                self.profile.as_ref().is_some_and(|p| &p.id != id) && self.profile_dirty(profiles)
            }
            Event::BeginName(NameAction::NewConnection) => self.connection_dirty(settings),
            Event::BeginName(NameAction::NewProfile) => self.profile_dirty(profiles),
            _ => false,
        }
    }

    pub(super) fn confirm_name(
        &mut self,
        settings: &mut Connections,
        profiles: &mut Vec<Profile>,
        project: Option<&Path>,
    ) -> (Task<Event>, Effect) {
        let name = self.name_value.trim().to_owned();
        if name.is_empty() || !safe_text(&name) {
            return (Task::none(), Effect::None);
        }
        let Some(action) = self.naming.take() else {
            return (Task::none(), Effect::None);
        };
        let result = match action {
            NameAction::NewConnection => {
                let (task, effect) = self.update(Event::NewConnection, settings, profiles, project);
                if let Some(connection) = &mut self.connection {
                    connection.name = name;
                    connection.protocol = self.new_protocol;
                    connection.port = self.new_protocol.port();
                    self.port = connection.port.to_string();
                }
                (task, effect)
            }
            NameAction::NewProfile => {
                let (task, effect) = self.update(Event::NewProfile, settings, profiles, project);
                if let Some(profile) = &mut self.profile {
                    profile.name = name;
                }
                (task, effect)
            }
            NameAction::RenameConnection | NameAction::DuplicateConnection => {
                if let Some(connection) = &mut self.connection {
                    if action == NameAction::DuplicateConnection {
                        match identifier() {
                            Ok(id) => connection.id = id,
                            Err(error) => {
                                self.error = Some(error);
                                return (Task::none(), Effect::None);
                            }
                        }
                        self.secret = Secret::default();
                        self.secret_changed = false;
                    }
                    connection.name = name;
                }
                (Task::none(), Effect::None)
            }
            NameAction::RenameProfile | NameAction::DuplicateProfile => {
                if let Some(profile) = &mut self.profile {
                    if action == NameAction::DuplicateProfile {
                        match identifier() {
                            Ok(id) => profile.id = id,
                            Err(error) => {
                                self.error = Some(error);
                                return (Task::none(), Effect::None);
                            }
                        }
                        profile.default = false;
                    }
                    profile.name = name;
                }
                (Task::none(), Effect::None)
            }
        };
        self.name_value.clear();
        self.error = None;
        self.status = None;
        result
    }
}
