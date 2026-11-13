//! Verify manifest validation, target selection, and panel description limits.

use super::{
    ButtonStyle, InstalledModule, LocalizedText, Manifest, ModuleRegistry, Permission, Placement,
    SYSTEM_MODULE_IDS, TextStyle, WidgetAction, WidgetNode, current_target,
};
use std::collections::{BTreeMap, BTreeSet};

fn manifest(id: &str) -> Manifest {
    let mut entry = BTreeMap::new();
    entry.insert(current_target().to_owned(), "module-bin".to_owned());
    Manifest {
        id: id.to_owned(),
        name: LocalizedText {
            en: "Sample".to_owned(),
            ru: None,
            es: None,
        },
        description: LocalizedText {
            en: "Sample module".to_owned(),
            ru: None,
            es: None,
        },
        version: semver::Version::new(1, 0, 0),
        api: 1,
        min_app_version: semver::Version::new(1, 0, 0),
        entry,
        permissions: BTreeSet::from([Permission::Network]),
        placement: Placement::Bottom,
    }
}

#[test]
fn accepts_valid_manifest() {
    assert!(manifest("sample").is_valid(&SYSTEM_MODULE_IDS));
    assert_eq!(manifest("sample").entry_name(), Some("module-bin"));
}

#[test]
fn rejects_reserved_and_malformed_identifiers() {
    assert!(!manifest("git").is_valid(&SYSTEM_MODULE_IDS));
    assert!(!manifest("").is_valid(&SYSTEM_MODULE_IDS));
    assert!(!manifest("Sample").is_valid(&SYSTEM_MODULE_IDS));
    assert!(!manifest("a".repeat(65).as_str()).is_valid(&SYSTEM_MODULE_IDS));
}

#[test]
fn rejects_unsupported_api_version() {
    let mut module = manifest("sample");
    module.api = 2;
    assert!(!module.is_valid(&SYSTEM_MODULE_IDS));
}

#[test]
fn rejects_entry_names_that_escape_the_package() {
    let mut module = manifest("sample");
    module
        .entry
        .insert(current_target().to_owned(), "../escape".to_owned());
    assert!(!module.is_valid(&SYSTEM_MODULE_IDS));
    let mut slashes = module;
    slashes.entry.clear();
    assert!(!slashes.is_valid(&SYSTEM_MODULE_IDS));
}

#[test]
fn localized_text_falls_back_to_english() {
    let text = LocalizedText {
        en: "Sample".to_owned(),
        ru: Some("Пример".to_owned()),
        es: None,
    };
    assert_eq!(text.resolve("ru"), "Пример");
    assert_eq!(text.resolve("es"), "Sample");
    assert_eq!(text.resolve("en"), "Sample");
}

#[test]
fn widget_tree_round_trips_through_json() {
    let panel = WidgetNode::Column {
        children: vec![
            WidgetNode::Text {
                content: "Hello".to_owned(),
                style: TextStyle::Heading,
            },
            WidgetNode::Button {
                id: "click".to_owned(),
                label: "Press".to_owned(),
                style: ButtonStyle::Primary,
                disabled: false,
            },
        ],
        spacing: 8,
    };
    let json = serde_json::to_string(&panel).expect("serialize panel");
    let parsed: WidgetNode = serde_json::from_str(&json).expect("parse panel");
    assert_eq!(parsed, panel);
    assert!(panel.is_valid());
}

#[test]
fn widget_tree_rejects_excessive_depth_and_bad_values() {
    let mut deep = WidgetNode::Separator;
    for _ in 0..64 {
        deep = WidgetNode::Scroll {
            child: Box::new(deep),
        };
    }
    assert!(!deep.is_valid());

    let oversized = WidgetNode::Text {
        content: "a".repeat(9000),
        style: TextStyle::Body,
    };
    assert!(!oversized.is_valid());

    let progress = WidgetNode::Progress { value: 1.5 };
    assert!(!progress.is_valid());
}

#[test]
fn widget_actions_serialize_with_tags() {
    let actions = [
        WidgetAction::Click,
        WidgetAction::Toggle { value: true },
        WidgetAction::Change {
            value: "text".to_owned(),
        },
    ];
    assert_eq!(
        serde_json::to_string(&actions[0]).expect("serialize click"),
        r#"{"type":"click"}"#
    );
    assert_eq!(
        serde_json::to_string(&actions[1]).expect("serialize toggle"),
        r#"{"type":"toggle","value":true}"#
    );
    assert_eq!(
        serde_json::to_string(&actions[2]).expect("serialize change"),
        r#"{"type":"change","value":"text"}"#
    );
}

#[test]
fn manifest_round_trips_through_json() {
    let module = manifest("sample");
    let json = serde_json::to_string(&module).expect("serialize manifest");
    let parsed: Manifest = serde_json::from_str(&json).expect("parse manifest");
    assert_eq!(parsed, module);
    assert!(json.contains("\"minAppVersion\""));
}

#[test]
fn registry_finds_installed_module_by_identifier() {
    let mut registry = ModuleRegistry::default();
    assert!(registry.get("sample").is_none());
    registry.modules.push(InstalledModule {
        manifest: manifest("sample"),
        installed_at: 42,
    });
    assert!(registry.get("sample").is_some());
    assert!(registry.get("other").is_none());
}
