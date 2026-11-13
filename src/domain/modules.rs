//! Describe external module packages and declarative panels without operating-system access.

use std::collections::{BTreeMap, BTreeSet};

/// Protocol version the application speaks with external modules.
pub(crate) const API_VERSION: u32 = 1;

/// Longest allowed widget tree depth.
pub(crate) const MAX_WIDGET_DEPTH: usize = 32;
/// Largest allowed widget tree node count.
pub(crate) const MAX_WIDGET_NODES: usize = 512;
/// Longest allowed widget text.
pub(crate) const MAX_WIDGET_TEXT: usize = 8192;
/// Longest allowed widget identifier.
pub(crate) const MAX_WIDGET_ID: usize = 128;

/// Operations an external module declares in its manifest.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Permission {
    FilesystemRead,
    FilesystemWrite,
    Network,
    Process,
}

/// Workspace zone where an external module panel appears.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Placement {
    Header,
    Top,
    Left,
    Right,
    Bottom,
}

impl Placement {
    pub(crate) const ALL: [Placement; 5] = [
        Placement::Header,
        Placement::Top,
        Placement::Left,
        Placement::Right,
        Placement::Bottom,
    ];
}

/// Localized manifest text. English is required; other languages fall back to it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct LocalizedText {
    pub en: String,
    pub ru: Option<String>,
    pub es: Option<String>,
}

impl LocalizedText {
    pub(crate) fn resolve(&self, language: &str) -> &str {
        match language {
            "ru" => self.ru.as_deref().unwrap_or(&self.en),
            "es" => self.es.as_deref().unwrap_or(&self.en),
            _ => &self.en,
        }
    }
}

/// Package description embedded in every external module package.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Manifest {
    pub id: String,
    pub name: LocalizedText,
    pub description: LocalizedText,
    pub version: semver::Version,
    pub api: u32,
    pub min_app_version: semver::Version,
    /// Executable file names under `binaries/<target>/` per platform target.
    pub entry: BTreeMap<String, String>,
    pub permissions: BTreeSet<Permission>,
    pub placement: Placement,
}

/// Platform identifier of the current build.
pub(crate) fn current_target() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "macos-aarch64"
    }
    #[cfg(all(target_os = "macos", not(target_arch = "aarch64")))]
    {
        "macos-x86_64"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux-x86_64"
    }
    #[cfg(all(target_os = "linux", not(target_arch = "x86_64")))]
    {
        "linux-aarch64"
    }
    #[cfg(target_os = "windows")]
    {
        "windows-x86_64"
    }
}

impl Manifest {
    /// Check field limits and identifier syntax. Reject reserved identifiers.
    pub(crate) fn is_valid(&self, reserved: &[&str]) -> bool {
        valid_id(&self.id)
            && !reserved.contains(&self.id.as_str())
            && self.api == API_VERSION
            && !self.name.en.trim().is_empty()
            && self.name.en.chars().count() <= MAX_WIDGET_TEXT
            && self.description.en.chars().count() <= MAX_WIDGET_TEXT
            && !self.entry.is_empty()
            && self.entry.len() <= 16
            && self
                .entry
                .values()
                .all(|name| valid_component(name) && name.len() <= MAX_WIDGET_ID)
            && self
                .entry
                .keys()
                .all(|target| valid_component(target) && target.len() <= MAX_WIDGET_ID)
    }

    /// Executable file name for the current platform.
    pub(crate) fn entry_name(&self) -> Option<&str> {
        self.entry.get(current_target()).map(String::as_str)
    }
}

/// Stable module identifiers bundled with the application.
pub(crate) const SYSTEM_MODULE_IDS: [&str; 4] = ["git", "pipelines", "terminal", "deployment"];

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.chars().next().is_some_and(char::is_alphanumeric)
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn valid_component(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with(['.', '/'])
        && !value.contains(['/', '\\', ':'])
        && !value.contains("..")
}

/// Interaction an interface widget reports to an external module.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub(crate) enum WidgetAction {
    Click,
    Toggle { value: bool },
    Change { value: String },
}

/// Declarative panel description an external module sends to the host.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub(crate) enum WidgetNode {
    Column {
        children: Vec<WidgetNode>,
        #[serde(default)]
        spacing: u8,
    },
    Row {
        children: Vec<WidgetNode>,
        #[serde(default)]
        spacing: u8,
    },
    Text {
        content: String,
        #[serde(default)]
        style: TextStyle,
    },
    Button {
        id: String,
        label: String,
        #[serde(default)]
        style: ButtonStyle,
        #[serde(default)]
        disabled: bool,
    },
    TextInput {
        id: String,
        #[serde(default)]
        value: String,
        #[serde(default)]
        placeholder: String,
        #[serde(default)]
        disabled: bool,
    },
    Checkbox {
        id: String,
        label: String,
        checked: bool,
    },
    Toggle {
        id: String,
        checked: bool,
        #[serde(default)]
        disabled: bool,
    },
    Progress {
        value: f32,
    },
    Separator,
    Scroll {
        child: Box<WidgetNode>,
    },
    Spacer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum TextStyle {
    #[default]
    Body,
    Heading,
    Caption,
    Muted,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ButtonStyle {
    #[default]
    Secondary,
    Primary,
    Danger,
}

impl WidgetNode {
    /// Reject oversized or malformed panel descriptions.
    pub(crate) fn is_valid(&self) -> bool {
        let mut nodes = 0_usize;
        self.valid(MAX_WIDGET_DEPTH, &mut nodes)
    }

    fn valid(&self, depth: usize, nodes: &mut usize) -> bool {
        if depth == 0 || *nodes == MAX_WIDGET_NODES {
            return false;
        }
        *nodes += 1;
        let text = |value: &str| value.chars().count() <= MAX_WIDGET_TEXT;
        let identifier = |value: &str| !value.is_empty() && value.len() <= MAX_WIDGET_ID;
        match self {
            Self::Column { children, .. } | Self::Row { children, .. } => {
                children.iter().all(|child| child.valid(depth - 1, nodes))
            }
            Self::Text { content, .. } => text(content),
            Self::Button { id, label, .. } => identifier(id) && text(label),
            Self::TextInput {
                id,
                value,
                placeholder,
                ..
            } => identifier(id) && text(value) && text(placeholder),
            Self::Checkbox { id, label, .. } => identifier(id) && text(label),
            Self::Toggle { id, .. } => identifier(id),
            Self::Progress { value } => value.is_finite() && (0.0..=1.0).contains(value),
            Self::Separator | Self::Spacer => true,
            Self::Scroll { child } => child.valid(depth - 1, nodes),
        }
    }
}

/// Registry record for one installed external module.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstalledModule {
    pub manifest: Manifest,
    /// Installation time in UTC milliseconds.
    pub installed_at: u64,
}

/// Persisted collection of installed external modules.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct ModuleRegistry {
    pub modules: Vec<InstalledModule>,
}

impl ModuleRegistry {
    pub(crate) fn get(&self, id: &str) -> Option<&InstalledModule> {
        self.modules.iter().find(|module| module.manifest.id == id)
    }
}

#[cfg(test)]
mod tests;
