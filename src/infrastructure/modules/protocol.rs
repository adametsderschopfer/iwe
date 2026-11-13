//! Define the line-delimited JSON protocol between the host and module processes.

use crate::domain::modules::{API_VERSION, WidgetAction, WidgetNode};

/// Largest accepted protocol line.
pub(crate) const MAX_LINE: usize = 1024 * 1024;
/// Largest accepted notification text.
const MAX_NOTIFY_TEXT: usize = 8192;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub(crate) enum HostMessage {
    Initialize {
        api: u32,
        app_version: String,
        language: String,
        install_path: String,
    },
    Event {
        id: String,
        action: WidgetAction,
    },
    Shutdown,
}

impl HostMessage {
    /// First message the host sends to a started module process.
    pub(crate) fn initialize(language: &str, install_path: &str) -> Self {
        Self::Initialize {
            api: API_VERSION,
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            language: language.to_owned(),
            install_path: install_path.to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum NotifyLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ModuleMessage {
    Panel(WidgetNode),
    Notify {
        level: NotifyLevel,
        message: String,
    },
    /// The module process ended its output stream.
    Exited,
}

/// Serialize a host message as one protocol line without the newline.
pub(crate) fn encode(message: &HostMessage) -> String {
    serde_json::to_string(message).unwrap_or_default()
}

/// Parse one module output line. Invalid lines are ignored.
pub(crate) fn parse(line: &str) -> Option<ModuleMessage> {
    if line.len() > MAX_LINE {
        return None;
    }
    match serde_json::from_str::<ModuleWire>(line).ok()? {
        ModuleWire::Panel { content } => {
            content.is_valid().then_some(ModuleMessage::Panel(content))
        }
        ModuleWire::Notify { level, message } => (message.chars().count() <= MAX_NOTIFY_TEXT)
            .then_some(ModuleMessage::Notify { level, message }),
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum ModuleWire {
    Panel { content: WidgetNode },
    Notify { level: NotifyLevel, message: String },
}

#[cfg(test)]
mod tests;
