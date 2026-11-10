//! Store unique source and tab shortcuts independently of keyboard frameworks.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) enum Command {
    GoToLine,
    Copy,
    SelectAll,
    CloseTab,
    ReopenTab,
    DeploymentCompare,
    DeploymentUpload,
}

impl Command {
    pub(crate) const ALL: [Self; 7] = [
        Self::GoToLine,
        Self::Copy,
        Self::SelectAll,
        Self::CloseTab,
        Self::ReopenTab,
        Self::DeploymentCompare,
        Self::DeploymentUpload,
    ];

    pub(crate) fn default_binding(self) -> Binding {
        Binding {
            key: match self {
                Self::GoToLine if cfg!(target_os = "macos") => "L",
                Self::GoToLine => "G",
                Self::Copy | Self::DeploymentCompare => "C",
                Self::DeploymentUpload => "V",
                Self::SelectAll => "A",
                Self::CloseTab => "W",
                Self::ReopenTab => "T",
            }
            .to_owned(),
            control: !cfg!(target_os = "macos"),
            super_key: cfg!(target_os = "macos"),
            alt: matches!(self, Self::DeploymentCompare | Self::DeploymentUpload),
            shift: self == Self::ReopenTab,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Binding {
    pub key: String,
    pub control: bool,
    pub super_key: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Binding {
    pub(crate) fn valid(&self) -> bool {
        let key = self.key.as_bytes();
        let supported = (key.len() == 1
            && (key[0].is_ascii_uppercase() || key[0].is_ascii_digit()))
            || self
                .key
                .strip_prefix('F')
                .and_then(|n| n.parse::<u8>().ok())
                .is_some_and(|n| (1..=12).contains(&n) && self.key == format!("F{n}"));
        supported && (self.control || self.super_key || self.alt)
    }

    pub(crate) fn reserved(&self) -> bool {
        if self.key == "Comma"
            && !self.alt
            && !self.shift
            && self.control != cfg!(target_os = "macos")
            && self.super_key == cfg!(target_os = "macos")
        {
            return true;
        }
        cfg!(target_os = "macos")
            && self.super_key
            && !self.control
            && !self.shift
            && (self.key == "H" || (!self.alt && self.key == "Q"))
    }

    pub(crate) fn label(&self) -> String {
        let mut parts = Vec::new();
        if self.control {
            parts.push("Ctrl");
        }
        if self.super_key {
            parts.push(if cfg!(target_os = "macos") {
                "Command"
            } else {
                "Super"
            });
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        parts.push(&self.key);
        parts.join(" + ")
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct Shortcuts(BTreeMap<Command, Binding>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rejection {
    Invalid,
    Conflict,
}

impl Shortcuts {
    pub(crate) fn binding(&self, command: Command) -> Binding {
        self.0
            .get(&command)
            .cloned()
            .unwrap_or_else(|| command.default_binding())
    }

    pub(crate) fn assign(&mut self, command: Command, binding: Binding) -> Result<(), Rejection> {
        if binding.reserved() {
            return Err(Rejection::Conflict);
        }
        if !binding.valid() {
            return Err(Rejection::Invalid);
        }
        if Command::ALL
            .into_iter()
            .any(|other| other != command && self.binding(other) == binding)
        {
            return Err(Rejection::Conflict);
        }
        self.0.insert(command, binding);
        Ok(())
    }

    pub(crate) fn normalize(&mut self) {
        let original = std::mem::take(&mut self.0);
        // Retain only complete, unique effective assignments, including default bindings.
        self.0 = original
            .into_iter()
            .filter(|(_, binding)| binding.valid() && !binding.reserved())
            .collect();
        loop {
            let conflict = Command::ALL.into_iter().find(|command| {
                self.0.contains_key(command)
                    && Command::ALL.into_iter().any(|other| {
                        other != *command && self.binding(other) == self.binding(*command)
                    })
            });
            let Some(command) = conflict else {
                break;
            };
            self.0.remove(&command);
        }
    }
}

#[cfg(test)]
mod tests;
