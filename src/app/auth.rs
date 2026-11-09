//! Bind authorization tasks to the selected provider and discard stale results.

use super::{App, Message as AppMessage, PipelineState, error_key};
use crate::{
    domain::{
        Failure,
        auth::{Account, AuthPrompt, Secret},
    },
    i18n::Key,
    infrastructure::{auth::Pending, pipelines},
};
use iced::Task;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stage {
    Starting,
    Waiting,
    Token,
    Connecting,
    Failed,
}

pub(crate) struct Dialog {
    pub account: Account,
    pub prompt: Option<AuthPrompt>,
    pub token: Secret,
    pub stage: Stage,
    pub error: Option<Failure>,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Started(u64, Result<Pending, Failure>),
    Finished(u64, Result<String, Failure>),
    TokenChanged(Secret),
    SubmitToken,
    Cancel,
    CopyCode,
    Disconnect,
    Disconnected(u64, u64, Result<(), Failure>),
}

impl App {
    fn account(&self) -> Result<Account, Failure> {
        Account::from_remote(
            self.active_repository()
                .and_then(|repo| repo.remote.as_ref())
                .ok_or(Failure::Read)?,
        )
    }

    pub(super) fn cancel_auth(&mut self) {
        if let Some(cancel) = self.auth_cancel.take() {
            cancel.cancel();
        }
        self.auth_request += 1;
        self.auth_dialog = None;
    }

    pub(super) fn begin_auth(&mut self) -> Task<AppMessage> {
        if self.disconnecting {
            return Task::none();
        }
        let Ok(account) = self.account() else {
            return Task::none();
        };
        self.cancel_auth();
        let browser = account.browser_login();
        self.auth_dialog = Some(Dialog {
            account: account.clone(),
            prompt: None,
            token: Secret::default(),
            stage: if browser {
                Stage::Starting
            } else {
                Stage::Token
            },
            error: None,
        });
        let cancel = CancellationToken::new();
        self.auth_cancel = Some(cancel.clone());
        if !browser {
            return Task::none();
        }
        let request = self.auth_request;
        let service = self.auth_service.clone();
        Task::perform(
            async move {
                tokio::select! { biased; _ = cancel.cancelled() => Err(Failure::AuthCancelled), result = service.start(account) => result }
            },
            move |result| AppMessage::Auth(Message::Started(request, result)),
        )
    }

    pub(super) fn update_auth(&mut self, message: Message) -> Task<AppMessage> {
        match message {
            Message::Cancel if self.auth_dialog.is_some() => self.cancel_auth(),
            Message::Started(request, result) if request == self.auth_request => {
                let Some(dialog) = self.auth_dialog.as_mut() else {
                    return Task::none();
                };
                match result {
                    Ok(pending) => {
                        let prompt = pending.prompt();
                        let url = prompt.url.clone();
                        dialog.prompt = Some(prompt);
                        dialog.stage = Stage::Waiting;
                        let account = dialog.account.clone();
                        let service = self.auth_service.clone();
                        let cancel = self.auth_cancel.clone().unwrap_or_default();
                        let browser_message = self.i18n.text(Key::BrowserReturn);
                        return Task::batch([
                            Task::perform(pipelines::open_url(url), AppMessage::Opened),
                            Task::perform(
                                async move {
                                    service
                                        .finish(account, pending, cancel, browser_message)
                                        .await
                                },
                                move |result| AppMessage::Auth(Message::Finished(request, result)),
                            ),
                        ]);
                    }
                    Err(error) => {
                        dialog.stage = Stage::Failed;
                        dialog.error = Some(error);
                    }
                }
            }
            Message::Finished(request, result) if request == self.auth_request => match result {
                Ok(username) => {
                    self.notifications
                        .success(Key::SignedIn, Some(username.clone()));
                    self.auth_cancel = None;
                    self.auth_dialog = None;
                    self.auth_request += 1;
                    self.account_username = Some(username);
                    self.pipeline_refreshing = false;
                    return Task::batch([self.load_pipelines(), self.reload_jobs_after_sign_in()]);
                }
                Err(error) => {
                    if let Some(dialog) = &mut self.auth_dialog {
                        dialog.token = Secret::default();
                        dialog.stage = if dialog.account.browser_login() {
                            Stage::Failed
                        } else {
                            Stage::Token
                        };
                        dialog.error = Some(error);
                    }
                }
            },
            Message::TokenChanged(token) => {
                if let Some(dialog) = &mut self.auth_dialog
                    && dialog.stage == Stage::Token
                    && token.expose().len() <= 8192
                {
                    dialog.token = token;
                }
            }
            Message::SubmitToken => {
                let Some(dialog) = self
                    .auth_dialog
                    .as_mut()
                    .filter(|dialog| dialog.stage == Stage::Token && !dialog.token.is_empty())
                else {
                    return Task::none();
                };
                let token = std::mem::take(&mut dialog.token);
                dialog.stage = Stage::Connecting;
                dialog.error = None;
                let account = dialog.account.clone();
                let service = self.auth_service.clone();
                let cancel = self.auth_cancel.clone().unwrap_or_default();
                let request = self.auth_request;
                return Task::perform(
                    async move { service.connect_token(account, token, cancel).await },
                    move |result| AppMessage::Auth(Message::Finished(request, result)),
                );
            }
            Message::CopyCode => {
                if let Some(code) = self
                    .auth_dialog
                    .as_ref()
                    .and_then(|dialog| dialog.prompt.as_ref())
                    .and_then(|prompt| prompt.code.as_ref())
                {
                    return iced::clipboard::write(code.clone());
                }
            }
            Message::Disconnect if !self.disconnecting => {
                let Ok(account) = self.account() else {
                    return Task::none();
                };
                self.cancel_auth();
                self.reset_jobs();
                self.disconnecting = true;
                self.pipeline_request += 1;
                self.pipeline_refreshing = false;
                self.pipelines = PipelineState::Loading;
                let (generation, request) = (self.generation, self.auth_request);
                let service = self.auth_service.clone();
                return Task::perform(
                    async move { service.disconnect(account).await },
                    move |result| {
                        AppMessage::Auth(Message::Disconnected(generation, request, result))
                    },
                );
            }
            Message::Disconnected(generation, request, result)
                if generation == self.generation && request == self.auth_request =>
            {
                self.disconnecting = false;
                match result {
                    Ok(()) => {
                        self.notifications
                            .success(Key::SignedOut, self.account_username.clone());
                        self.account_username = None;
                        return self.load_pipelines();
                    }
                    Err(error) => {
                        self.notifications.error(error_key(error), None);
                        self.pipelines = PipelineState::Failed(error);
                    }
                }
            }
            _ => {}
        }
        Task::none()
    }
}
