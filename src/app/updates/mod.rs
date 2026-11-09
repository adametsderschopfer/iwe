//! Coordinate release checks, downloads, and restart after workspace persistence.

use super::{App, Message};
use crate::{
    domain::update::Release,
    infrastructure::updates::{self, Error, Prepared},
};
use iced::{Task, window};
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub(crate) enum State {
    #[default]
    Idle,
    Checking,
    Current,
    Available(Release),
    Downloading,
    Ready(Arc<Prepared>),
    Saving(Arc<Prepared>),
    Installing(Arc<Prepared>),
    Failed(Error),
}
#[derive(Debug, Clone)]
pub(crate) enum Event {
    Check,
    Checked(Result<Option<Release>, Error>),
    Download,
    Downloaded(Result<Arc<Prepared>, Error>),
    Install,
    Started(Result<(), Error>),
}

impl App {
    pub(super) fn update_release(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Check
                if matches!(
                    self.updates,
                    State::Idle | State::Current | State::Available(_) | State::Failed(_)
                ) =>
            {
                self.updates = State::Checking;
                return Task::perform(updates::check(), |result| {
                    Message::Update(Event::Checked(result))
                });
            }
            Event::Checked(result) if matches!(self.updates, State::Checking) => {
                self.updates = match result {
                    Ok(Some(release)) => State::Available(release),
                    Ok(None) => State::Current,
                    Err(error) => State::Failed(error),
                };
            }
            Event::Download => {
                if let State::Available(release) = &self.updates {
                    let release = release.clone();
                    self.updates = State::Downloading;
                    return Task::perform(updates::download(release), |result| {
                        Message::Update(Event::Downloaded(result))
                    });
                }
            }
            Event::Downloaded(result) if matches!(self.updates, State::Downloading) => {
                self.updates = match result {
                    Ok(prepared) => State::Ready(prepared),
                    Err(error) => {
                        self.notifications.error(error_key(error), None);
                        State::Failed(error)
                    }
                };
            }
            Event::Install => {
                if let State::Ready(prepared) = &self.updates {
                    if self.save_blocked || self.window_id.is_none() {
                        self.updates = State::Failed(Error::Install);
                    } else {
                        self.updates = State::Saving(Arc::clone(prepared));
                        return self.save();
                    }
                }
            }
            Event::Started(result) if matches!(self.updates, State::Installing(_)) => {
                if result.is_ok()
                    && let Some(id) = self.window_id
                {
                    return window::close(id);
                }
                self.notifications.error(error_key(Error::Install), None);
                self.updates = State::Failed(Error::Install);
            }
            _ => {}
        }
        Task::none()
    }

    pub(super) fn restart_after_save(&mut self, success: bool) -> Task<Message> {
        let State::Saving(prepared) = &self.updates else {
            return Task::none();
        };
        if !success {
            self.updates = State::Failed(Error::Install);
            return Task::none();
        }
        let prepared = Arc::clone(prepared);
        self.updates = State::Installing(Arc::clone(&prepared));
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || updates::start_install(&prepared))
                    .await
                    .unwrap_or(Err(Error::Install))
            },
            |result| Message::Update(Event::Started(result)),
        )
    }
}

#[cfg(test)]
mod tests;

pub(crate) fn error_key(error: Error) -> crate::i18n::Key {
    use crate::i18n::Key;
    match error {
        Error::Invalid => Key::UpdateInvalid,
        Error::Install => Key::UpdateInstallFailed,
        Error::Unsupported => Key::UpdateUnsupported,
        Error::Network => Key::UpdateCheckFailed,
    }
}
