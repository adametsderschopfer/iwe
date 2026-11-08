//! Format embedded translations without global mutable state or runtime file access.

use fluent_bundle::{FluentBundle, FluentResource};
use unic_langid::{LanguageIdentifier, langid};

pub(crate) use fluent_bundle::FluentArgs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Language {
    English,
    Russian,
    Spanish,
}

impl Language {
    pub(crate) fn select(explicit: Option<&str>, system: Option<&str>) -> Self {
        let locale = explicit
            .filter(|value| !value.trim().is_empty())
            .or(system)
            .unwrap_or("en");
        let primary = locale.trim().split(['-', '_', '.', '@']).next();

        match primary.map(str::to_ascii_lowercase).as_deref() {
            Some("ru") => Self::Russian,
            Some("es") => Self::Spanish,
            _ => Self::English,
        }
    }

    fn identifier(self) -> LanguageIdentifier {
        match self {
            Self::English => langid!("en"),
            Self::Russian => langid!("ru"),
            Self::Spanish => langid!("es"),
        }
    }

    fn source(self) -> &'static str {
        match self {
            Self::English => include_str!("../../locales/en.ftl"),
            Self::Russian => include_str!("../../locales/ru.ftl"),
            Self::Spanish => include_str!("../../locales/es.ftl"),
        }
    }
}

macro_rules! message_keys {
    ($($variant:ident => $id:literal),+ $(,)?) => {
        #[derive(Debug, Clone, Copy)]
        pub(crate) enum Key { $($variant),+ }

        impl Key {
            #[cfg(test)]
            const ALL: &'static [Self] = &[$(Self::$variant),+];

            fn id(self) -> &'static str {
                match self { $(Self::$variant => $id),+ }
            }
        }
    };
}

message_keys! {
    AppTitle => "app-title",
    Greeting => "greeting",
    MessageUnavailable => "message-unavailable",
    GraphicsUnavailable => "graphics-unavailable",
    StartFailed => "start-failed",
    GraphicsTrying => "graphics-trying",
    UnsupportedOs => "unsupported-os",
    GraphicsInitializationFailed => "graphics-initialization-failed",
    ApplicationFailed => "application-failed",
}

pub(crate) struct I18n {
    selected: Option<FluentBundle<FluentResource>>,
    english: Option<FluentBundle<FluentResource>>,
}

impl I18n {
    pub(crate) fn new(language: Language) -> Self {
        Self {
            selected: bundle(language, language.source()),
            english: bundle(Language::English, Language::English.source()),
        }
    }

    pub(crate) fn text(&self, key: Key) -> String {
        self.format(key, &FluentArgs::new())
    }

    pub(crate) fn format(&self, key: Key, args: &FluentArgs<'_>) -> String {
        self.resolve(key, args)
            .or_else(|| self.resolve(Key::MessageUnavailable, &FluentArgs::new()))
            .unwrap_or_default()
    }

    fn resolve(&self, key: Key, args: &FluentArgs<'_>) -> Option<String> {
        self.selected
            .as_ref()
            .and_then(|bundle| format(bundle, key, args))
            .or_else(|| {
                self.english
                    .as_ref()
                    .and_then(|bundle| format(bundle, key, args))
            })
    }
}

fn bundle(language: Language, source: &str) -> Option<FluentBundle<FluentResource>> {
    let resource = FluentResource::try_new(source.to_owned()).ok()?;
    let mut bundle = FluentBundle::new(vec![language.identifier()]);
    bundle.add_resource(resource).ok()?;
    Some(bundle)
}

fn format(
    bundle: &FluentBundle<FluentResource>,
    key: Key,
    args: &FluentArgs<'_>,
) -> Option<String> {
    let pattern = bundle.get_message(key.id())?.value()?;
    let mut errors = Vec::new();
    let text = bundle.format_pattern(pattern, Some(args), &mut errors);
    (errors.is_empty() && !text.trim().is_empty()).then(|| text.into_owned())
}

#[cfg(test)]
mod tests;
