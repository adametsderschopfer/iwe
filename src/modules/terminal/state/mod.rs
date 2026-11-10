//! Coordinate independent terminal tabs for the selected project.

use super::session::{MAX_INPUT, Output, Session};
use iced::{Subscription, Task};
use std::{
    hash::{Hash, Hasher},
    path::Path,
    sync::Arc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    Starting,
    Running,
    Exited,
    Failed,
}

#[derive(Default)]
pub(crate) struct State {
    pub open: bool,
    pub tabs: Vec<Tab>,
    pub active: Option<u64>,
    next_request: u64,
    next_number: u64,
}

pub(crate) struct Tab {
    pub request: u64,
    pub number: u64,
    pub status: Status,
    pub parser: vt100::Parser<Replies>,
    pub revision: u64,
    session: Option<Arc<Session>>,
}

impl Tab {
    fn new(request: u64, number: u64) -> Self {
        Self {
            request,
            number,
            status: Status::Starting,
            parser: vt100::Parser::new_with_callbacks(24, 80, 2000, Replies::default()),
            revision: 0,
            session: None,
        }
    }

    pub(crate) fn subscription(&self) -> Subscription<Event> {
        let Some(session) = &self.session else {
            return Subscription::none();
        };
        Subscription::run_with(
            Stream {
                request: self.request,
                session: Arc::clone(session),
            },
            |data| {
                iced::futures::stream::unfold(
                    (data.request, Some(Arc::clone(&data.session))),
                    |(request, session)| async move {
                        let session = session?;
                        let output = session.next().await;
                        let finished = output
                            .iter()
                            .any(|event| matches!(event, Output::Closed | Output::Failed));
                        Some((
                            Event::Output(request, output),
                            (request, (!finished).then_some(session)),
                        ))
                    },
                )
            },
        )
    }

    pub(crate) fn running(&self) -> bool {
        self.session.is_some()
    }

    fn fail(&mut self) {
        self.session = None;
        self.status = Status::Failed;
    }

    fn send(&mut self, bytes: Vec<u8>) {
        if let Some(session) = &self.session {
            if session.input(bytes).is_err() {
                self.fail();
            } else {
                if self.parser.screen().scrollback() != 0 {
                    self.revision = self.revision.wrapping_add(1);
                }
                self.parser.screen_mut().set_scrollback(0);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Start,
    Started(u64, Result<Arc<Session>, ()>),
    Close(u64),
    Select(u64),
    Hide,
    PasteRejected,
    Focus(u64),
    Output(u64, Vec<Output>),
    Input(u64, Vec<u8>),
    Resize(u64, u16, u16),
    Scroll(u64, i32),
    Paste(u64),
    Pasted(u64, Option<String>),
    Copy(String),
}

impl State {
    pub(crate) fn active_tab(&self) -> Option<&Tab> {
        self.tabs
            .iter()
            .find(|tab| Some(tab.request) == self.active)
    }

    pub(crate) fn subscription(&self) -> Subscription<Event> {
        Subscription::batch(self.tabs.iter().map(Tab::subscription))
    }

    pub(crate) fn reset(&mut self) {
        let next_request = self.next_request;
        *self = Self {
            next_request,
            ..Self::default()
        };
    }

    pub(crate) fn update(&mut self, event: Event, directory: Option<&Path>) -> Task<Event> {
        match event {
            Event::Start => {
                let Some(directory) = directory.map(Path::to_owned) else {
                    return Task::none();
                };
                self.next_request = self.next_request.wrapping_add(1);
                self.next_number = self.next_number.wrapping_add(1);
                let request = self.next_request;
                self.tabs.push(Tab::new(request, self.next_number));
                self.active = Some(request);
                self.open = true;
                let start = Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || Session::start(&directory, 24, 80))
                            .await
                            .unwrap_or(Err(()))
                    },
                    move |result| Event::Started(request, result),
                );
                return Task::batch([start, iced::widget::operation::snap_to_end("terminal-tabs")]);
            }
            Event::Hide => self.open = false,
            Event::Select(request) => {
                if self.tabs.iter().any(|tab| tab.request == request) {
                    self.active = Some(request);
                }
            }
            Event::Close(request) => {
                let Some(index) = self.tabs.iter().position(|tab| tab.request == request) else {
                    return Task::none();
                };
                self.tabs.remove(index);
                if self.tabs.is_empty() {
                    self.active = None;
                    self.open = false;
                } else if self.active == Some(request) {
                    self.active = Some(self.tabs[index.min(self.tabs.len() - 1)].request);
                }
            }
            Event::Copy(text) => return iced::clipboard::write(text),
            event => {
                let request = match &event {
                    Event::Started(request, _)
                    | Event::Output(request, _)
                    | Event::Input(request, _)
                    | Event::Resize(request, _, _)
                    | Event::Scroll(request, _)
                    | Event::Paste(request)
                    | Event::Pasted(request, _) => *request,
                    _ => return Task::none(),
                };
                if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.request == request) {
                    return tab.update(event);
                }
            }
        }
        Task::none()
    }
}

impl Tab {
    fn update(&mut self, event: Event) -> Task<Event> {
        let state = self;
        match event {
            Event::Started(request, result)
                if request == state.request && state.status == Status::Starting =>
            {
                match result {
                    Ok(session) => {
                        let (rows, cols) = state.parser.screen().size();
                        if session.resize(rows, cols).is_err() {
                            state.fail();
                        } else {
                            state.session = Some(session);
                            state.status = Status::Running;
                        }
                    }
                    Err(()) => state.fail(),
                }
            }
            Event::Output(request, events) if request == state.request && state.running() => {
                if !events.is_empty() {
                    state.revision = state.revision.wrapping_add(1);
                }
                for event in events {
                    match event {
                        Output::Bytes(bytes) => state.parser.process(&bytes),
                        Output::Closed => {
                            state.session = None;
                            state.status = Status::Exited;
                            break;
                        }
                        Output::Failed => {
                            state.fail();
                            break;
                        }
                    }
                }
                let replies = std::mem::take(&mut state.parser.callbacks_mut().0);
                if !replies.is_empty() {
                    state.send(replies);
                }
            }
            Event::Input(request, bytes) if request == state.request => state.send(bytes),
            Event::Resize(request, rows, cols) if request == state.request => {
                let (rows, cols) = (rows.clamp(1, 200), cols.clamp(1, 400));
                if state.parser.screen().size() != (rows, cols) {
                    state.revision = state.revision.wrapping_add(1);
                    state.parser.screen_mut().set_size(rows, cols);
                    if let Some(session) = &state.session
                        && session.resize(rows, cols).is_err()
                    {
                        state.fail();
                    }
                }
            }
            Event::Scroll(request, lines) if request == state.request => {
                let previous = state.parser.screen().scrollback();
                let offset = state
                    .parser
                    .screen()
                    .scrollback()
                    .saturating_add_signed(lines as isize);
                state.parser.screen_mut().set_scrollback(offset);
                if state.parser.screen().scrollback() != previous {
                    state.revision = state.revision.wrapping_add(1);
                }
            }
            Event::Paste(request) if request == state.request && state.running() => {
                return iced::clipboard::read().map(move |text| Event::Pasted(request, text));
            }
            Event::Pasted(request, Some(text)) if request == state.request && state.running() => {
                if text.len() <= MAX_INPUT - 12 {
                    // Remove escape characters before applying the shell's paste protocol.
                    let text = text
                        .replace(['\0', '\u{1b}'], "")
                        .replace("\r\n", "\n")
                        .replace('\n', "\r");
                    let bytes = if state.parser.screen().bracketed_paste() {
                        format!("\u{1b}[200~{text}\u{1b}[201~").into_bytes()
                    } else {
                        text.into_bytes()
                    };
                    state.send(bytes);
                } else {
                    return Task::done(Event::PasteRejected);
                }
            }
            _ => {}
        }
        Task::none()
    }
}

#[derive(Default)]
pub(crate) struct Replies(Vec<u8>);

struct Stream {
    request: u64,
    session: Arc<Session>,
}

impl Hash for Stream {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.request.hash(state);
    }
}

impl vt100::Callbacks for Replies {
    fn unhandled_csi(
        &mut self,
        screen: &mut vt100::Screen,
        i1: Option<u8>,
        i2: Option<u8>,
        params: &[&[u16]],
        c: char,
    ) {
        if i1.is_some() || i2.is_some() || self.0.len() > 4096 {
            return;
        }
        match (
            c,
            params.first().and_then(|p| p.first()).copied().unwrap_or(0),
        ) {
            ('n', 5) => self.0.extend_from_slice(b"\x1b[0n"),
            ('n', 6) => {
                let (row, col) = screen.cursor_position();
                self.0
                    .extend_from_slice(format!("\x1b[{};{}R", row + 1, col + 1).as_bytes());
            }
            ('c', 0) => self.0.extend_from_slice(b"\x1b[?1;2c"),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;
