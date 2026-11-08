use super::*;
use fluent_syntax::ast::Entry;
use std::collections::BTreeSet;

const LANGUAGES: [Language; 3] = [Language::English, Language::Russian, Language::Spanish];
#[test]
fn locale_selection_handles_overrides_regions_and_fallbacks() {
    for (explicit, system, expected) in [
        (Some("es-MX"), Some("ru-RU"), Language::Spanish),
        (Some(" RU_ru.UTF-8 "), None, Language::Russian),
        (Some("EN-us"), Some("es"), Language::English),
        (None, Some("ru-RU"), Language::Russian),
        (None, Some("es_ES.UTF-8"), Language::Spanish),
        (Some("  "), Some("ru"), Language::Russian),
        (Some("de-DE"), Some("ru"), Language::English),
        (None, Some("C"), Language::English),
        (None, Some("russian"), Language::English),
        (None, None, Language::English),
    ] {
        assert_eq!(Language::select(explicit, system), expected);
    }
}

#[test]
fn all_catalogs_have_the_complete_key_set_and_resolve_without_errors() {
    let expected: BTreeSet<_> = Key::ALL.iter().map(|key| key.id()).collect();
    for language in LANGUAGES {
        let source = language.source();
        let resource = FluentResource::try_new(source.to_owned()).expect("Catalog must parse");
        let keys: Vec<_> = resource
            .entries()
            .filter_map(|entry| match entry {
                Entry::Message(message) => Some(message.id.name),
                _ => None,
            })
            .collect();
        assert_eq!(
            keys.len(),
            expected.len(),
            "Missing, duplicate, or extra keys: {language:?}"
        );
        assert_eq!(keys.into_iter().collect::<BTreeSet<_>>(), expected);
        let bundle = bundle(language, source).expect("Catalog must have no duplicate identifiers");

        for &key in Key::ALL {
            let mut args = FluentArgs::new();
            match key {
                Key::UnchangedLines => {
                    args.set("start", 1);
                    args.set("end", 42);
                }
                Key::GraphicsTrying => {
                    args.set("backend", "Metal");
                }
                Key::StartFailed | Key::GraphicsInitializationFailed | Key::ApplicationFailed => {
                    args.set("error", "E_TEST");
                }
                _ => {}
            }
            let value =
                format(&bundle, key, &args).expect("Translation must resolve without fallback");
            assert!(!value.trim().is_empty());
            if matches!(key, Key::GraphicsTrying) {
                assert!(value.contains("Metal"));
            }
            if matches!(
                key,
                Key::StartFailed | Key::GraphicsInitializationFailed | Key::ApplicationFailed
            ) {
                assert!(value.contains("E_TEST"));
            }
        }
    }
}

#[test]
fn missing_or_invalid_translations_fall_back_to_english() {
    for source in [
        "app-title = Título",
        "greeting = { $missing }",
        "greeting = {",
    ] {
        let i18n = I18n {
            selected: bundle(Language::Spanish, source),
            english: bundle(Language::English, Language::English.source()),
        };
        assert_eq!(i18n.text(Key::Greeting), "What are you working on?");
    }
}

#[test]
fn unavailable_catalogs_do_not_expose_internal_keys() {
    let i18n = I18n {
        selected: None,
        english: None,
    };
    assert!(i18n.text(Key::Greeting).is_empty());
}

#[test]
fn missing_message_uses_nontechnical_text_in_the_selected_language() {
    for (language, expected) in [
        (Language::English, "This text is unavailable."),
        (Language::Russian, "Этот текст недоступен."),
        (Language::Spanish, "Este texto no está disponible."),
    ] {
        let source: String = language
            .source()
            .lines()
            .filter(|line| !line.starts_with("greeting ="))
            .map(|line| format!("{line}\n"))
            .collect();
        let i18n = I18n {
            selected: bundle(language, &source),
            english: None,
        };
        assert_eq!(i18n.text(Key::Greeting), expected);
    }
}

#[test]
fn separate_localizers_do_not_change_each_others_language() {
    let russian = I18n::new(Language::Russian);
    let spanish = I18n::new(Language::Spanish);
    assert_eq!(russian.text(Key::Greeting), "С чего начнём?");
    assert_eq!(spanish.text(Key::Greeting), "¿En qué vas a trabajar?");
    assert_eq!(russian.text(Key::Greeting), "С чего начнём?");
}
