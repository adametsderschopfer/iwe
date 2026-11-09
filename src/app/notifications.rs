//! Keep transient operation results outside persisted workspace state.

use crate::i18n::Key;
use iced::{Subscription, time::Instant};
use std::{collections::VecDeque, time::Duration};

const MAX_VISIBLE: usize = 3;
const SUCCESS_LIFETIME: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Success,
    Error,
}

pub(crate) struct Notification {
    pub id: u64,
    pub kind: Kind,
    pub message: Key,
    pub context: Option<String>,
    created: Instant,
    expires: Option<Instant>,
}

#[derive(Default)]
pub(crate) struct State {
    pub items: VecDeque<Notification>,
    next_id: u64,
    hovered: Option<Instant>,
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Tick(Instant),
    Dismiss(u64),
    Hover(bool),
}

impl State {
    pub(crate) fn success(&mut self, message: Key, context: Option<String>) {
        self.push(Kind::Success, message, context, Instant::now());
    }

    pub(crate) fn error(&mut self, message: Key, context: Option<String>) {
        self.push(Kind::Error, message, context, Instant::now());
    }

    fn push(&mut self, kind: Kind, message: Key, context: Option<String>, now: Instant) {
        self.items
            .retain(|item| item.kind != kind || item.message != message || item.context != context);
        while self.items.len() >= MAX_VISIBLE {
            self.items.pop_front();
        }
        self.next_id += 1;
        self.items.push_back(Notification {
            id: self.next_id,
            kind,
            message,
            context,
            created: now,
            expires: (kind == Kind::Success).then_some(now + SUCCESS_LIFETIME),
        });
    }

    pub(crate) fn subscription(&self) -> Subscription<Event> {
        if self.hovered.is_none() && self.items.iter().any(|item| item.expires.is_some()) {
            iced::time::every(Duration::from_millis(500)).map(Event::Tick)
        } else {
            Subscription::none()
        }
    }

    pub(crate) fn update(&mut self, event: Event) {
        self.update_at(event, Instant::now());
    }

    fn update_at(&mut self, event: Event, now: Instant) {
        match event {
            Event::Tick(now) if self.hovered.is_none() => {
                self.items
                    .retain(|item| item.expires.is_none_or(|end| now < end));
            }
            Event::Dismiss(id) => self.items.retain(|item| item.id != id),
            Event::Hover(true) => {
                self.hovered.get_or_insert(now);
            }
            Event::Hover(false) => {
                if let Some(start) = self.hovered.take() {
                    for item in &mut self.items {
                        if let Some(end) = &mut item.expires {
                            *end += now.saturating_duration_since(start.max(item.created));
                        }
                    }
                }
            }
            _ => {}
        }
        if self.items.is_empty() {
            self.hovered = None;
        }
    }
}

#[cfg(test)]
mod tests;
