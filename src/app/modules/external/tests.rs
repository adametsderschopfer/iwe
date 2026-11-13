//! Verify zone activation and enable state for external modules.

use super::*;
use crate::{
    app::App,
    i18n::{I18n, Language},
};
use std::rc::Rc;

fn app() -> App {
    App::new(Rc::new(I18n::new(Language::English)))
}

fn installed(id: &str, placement: Placement) -> InstalledModule {
    use std::collections::{BTreeMap, BTreeSet};
    let mut entry = BTreeMap::new();
    entry.insert(
        crate::domain::modules::current_target().to_owned(),
        "module-bin".to_owned(),
    );
    InstalledModule {
        manifest: Manifest {
            id: id.to_owned(),
            name: crate::domain::modules::LocalizedText {
                en: format!("{id} module"),
                ru: None,
                es: None,
            },
            description: crate::domain::modules::LocalizedText {
                en: "Test module".to_owned(),
                ru: None,
                es: None,
            },
            version: semver::Version::new(1, 0, 0),
            api: 1,
            min_app_version: semver::Version::new(1, 0, 0),
            entry,
            permissions: BTreeSet::new(),
            placement,
        },
        installed_at: 0,
    }
}

#[test]
fn zone_activation_follows_enabled_modules() {
    let mut app = app();
    app.modules
        .external
        .registry
        .modules
        .push(installed("alpha", Placement::Bottom));
    app.modules
        .external
        .registry
        .modules
        .push(installed("beta", Placement::Bottom));
    app.modules
        .external
        .registry
        .modules
        .push(installed("side", Placement::Left));
    app.external_sync_zones();
    assert_eq!(
        app.modules.external.active.get(&Placement::Bottom),
        Some(&"alpha".to_owned())
    );
    assert_eq!(
        app.modules.external.active.get(&Placement::Left),
        Some(&"side".to_owned())
    );

    app.workspace
        .preferences
        .disabled_modules
        .insert("alpha".to_owned());
    app.external_sync_zones();
    assert_eq!(
        app.modules.external.active.get(&Placement::Bottom),
        Some(&"beta".to_owned())
    );

    app.workspace
        .preferences
        .disabled_modules
        .insert("beta".to_owned());
    app.external_sync_zones();
    assert!(!app.modules.external.active.contains_key(&Placement::Bottom));
    assert_eq!(
        app.modules.external.active.get(&Placement::Left),
        Some(&"side".to_owned())
    );
}

#[test]
fn zone_selection_prefers_user_choice() {
    let mut app = app();
    app.modules
        .external
        .registry
        .modules
        .push(installed("alpha", Placement::Top));
    app.modules
        .external
        .registry
        .modules
        .push(installed("beta", Placement::Top));
    app.external_sync_zones();
    let _ = app.update_external(Event::SelectZone(Placement::Top, "beta".to_owned()));
    assert_eq!(
        app.modules.external.active.get(&Placement::Top),
        Some(&"beta".to_owned())
    );
    // Sync keeps an explicit choice that is still enabled.
    app.external_sync_zones();
    assert_eq!(
        app.modules.external.active.get(&Placement::Top),
        Some(&"beta".to_owned())
    );
}

#[test]
fn set_enabled_updates_preferences() {
    let mut app = app();
    app.modules
        .external
        .registry
        .modules
        .push(installed("alpha", Placement::Right));
    assert!(app.external_enabled("alpha"));
    let _ = app.update_external(Event::SetEnabled("alpha".to_owned(), false));
    assert!(!app.external_enabled("alpha"));
    assert!(app.modules.external.sessions.is_empty());
    let _ = app.update_external(Event::SetEnabled("alpha".to_owned(), true));
    assert!(app.external_enabled("alpha"));
}

#[test]
fn zone_modules_filter_by_placement_and_state() {
    let mut app = app();
    app.modules
        .external
        .registry
        .modules
        .push(installed("alpha", Placement::Bottom));
    app.modules
        .external
        .registry
        .modules
        .push(installed("hidden", Placement::Bottom));
    app.modules
        .external
        .registry
        .modules
        .push(installed("side", Placement::Left));
    app.workspace
        .preferences
        .disabled_modules
        .insert("hidden".to_owned());
    let bottom = app.external_zone_modules(Placement::Bottom);
    assert_eq!(bottom.len(), 1);
    assert_eq!(bottom[0].manifest.id, "alpha");
    let left = app.external_zone_modules(Placement::Left);
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].manifest.id, "side");
}
