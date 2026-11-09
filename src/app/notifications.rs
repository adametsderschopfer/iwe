//! Keep transient operation results outside persisted workspace state.

use crate::i18n::Key;
use iced::{Subscription, time::Instant};
use std::{collections::VecDeque, time::Duration};

const MAX_VISIBLE: usize = 3;
const LIFETIME: Duration = Duration::from_secs(5);

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
    expires: Instant,
    paused: Option<Instant>,
}

#[derive(Default)]
pub(crate) struct State {
    pub items: VecDeque<Notification>,
    next_id: u64,
    now: Option<Instant>,
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Tick(Instant),
    Dismiss(u64),
    Hover(u64, bool),
}

impl State {
    pub(crate) fn success(&mut self, message: Key, context: Option<String>) {
        self.push(Kind::Success, message, context, Instant::now());
    }

    pub(crate) fn error(&mut self, message: Key, context: Option<String>) {
        self.push(Kind::Error, message, context, Instant::now());
    }

    fn push(&mut self, kind: Kind, message: Key, context: Option<String>, now: Instant) {
        self.now = Some(now);
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
            expires: now + LIFETIME,
            paused: None,
        });
    }

    pub(crate) fn subscription(&self) -> Subscription<Event> {
        if self.items.iter().any(|item| item.paused.is_none()) {
            iced::time::every(Duration::from_millis(33)).map(Event::Tick)
        } else {
            Subscription::none()
        }
    }

    pub(crate) fn update(&mut self, event: Event) {
        self.update_at(event, Instant::now());
    }

    pub(crate) fn remaining(&self, item: &Notification) -> f32 {
        let now = item.paused.or(self.now).unwrap_or_else(Instant::now);
        (item.expires.saturating_duration_since(now).as_secs_f32() / LIFETIME.as_secs_f32())
            .clamp(0., 1.)
    }

    fn update_at(&mut self, event: Event, now: Instant) {
        self.now = Some(match &event { Event::Tick(instant) => *instant, _ => now });
        match event {
            Event::Tick(now) => {
                self.items
                    .retain(|item| item.paused.is_some() || now < item.expires);
            }
            Event::Dismiss(id) => self.items.retain(|item| item.id != id),
            Event::Hover(id, hovered) => {
                if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
                    if hovered {
                        item.paused.get_or_insert(now);
                    } else if let Some(start) = item.paused.take() {
                        item.expires += now.saturating_duration_since(start);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
