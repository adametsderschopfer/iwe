use super::*;

#[test]
fn all_results_expire_after_five_seconds() {
    let now = Instant::now();
    let mut state = State::default();
    state.push(Kind::Success, Key::GitUpdateDone, Some("main".into()), now);
    state.push(Kind::Error, Key::StorageSaveFailed, None, now);
    state.update_at(Event::Tick(now + Duration::from_secs(4)), now);
    assert_eq!(state.items.len(), 2);
    assert!((state.remaining(&state.items[0]) - 0.2).abs() < 0.001);
    state.update_at(Event::Tick(now + Duration::from_secs(5)), now);
    assert!(state.items.is_empty());
}

#[test]
fn hovering_pauses_only_the_hovered_notification_and_its_progress() {
    let now = Instant::now();
    let mut state = State::default();
    state.push(Kind::Success, Key::GitFetchDone, None, now);
    let id = state.items[0].id;
    state.update_at(Event::Hover(id, true), now + Duration::from_secs(4));
    state.update_at(Event::Tick(now + Duration::from_secs(10)), now);
    assert_eq!(state.items.len(), 1);
    assert!((state.remaining(&state.items[0]) - 0.2).abs() < 0.001);
    state.push(
        Kind::Success,
        Key::GitUpdateDone,
        None,
        now + Duration::from_secs(15),
    );
    state.update_at(Event::Tick(now + Duration::from_secs(20)), now);
    assert_eq!(state.items.len(), 1);
    assert_eq!(state.items[0].id, id);
    assert!((state.remaining(&state.items[0]) - 0.2).abs() < 0.001);
    state.update_at(Event::Hover(id, false), now + Duration::from_secs(34));
    assert!((state.remaining(&state.items[0]) - 0.2).abs() < 0.001);
    state.update_at(Event::Tick(now + Duration::from_secs(35)), now);
    assert!(state.items.is_empty());
}

#[test]
fn repeated_errors_coalesce_and_old_dismissals_cannot_close_new_results() {
    let now = Instant::now();
    let mut state = State::default();
    for context in ["one", "two", "three", "four"] {
        state.push(Kind::Error, Key::OpenFailed, Some(context.into()), now);
    }
    assert_eq!(state.items.len(), 3);
    assert_eq!(state.items[0].context.as_deref(), Some("two"));
    let old_id = state.items.back().unwrap().id;
    state.push(Kind::Error, Key::OpenFailed, Some("four".into()), now);
    assert_eq!(state.items.len(), 3);
    state.update_at(Event::Dismiss(old_id), now);
    assert_eq!(state.items.len(), 3);
    state.update_at(Event::Hover(state.items[0].id, true), now);
    let ids: Vec<_> = state.items.iter().map(|item| item.id).collect();
    for id in ids {
        state.update_at(Event::Dismiss(id), now);
    }
    assert!(state.items.is_empty());
}

#[test]
fn notification_cards_fit_all_languages_and_preserve_workspace_interactions() {
    use crate::{
        app::{App, Message, Navigation},
        i18n::{I18n, Language},
        ui::theme,
    };
    use iced::{Theme, mouse, widget::Id};
    use std::{path::PathBuf, rc::Rc};
    let settings = || iced::Settings {
        default_font: theme::font(),
        ..Default::default()
    };
    for (language, suffix) in [
        (Language::English, "en"),
        (Language::Russian, "ru"),
        (Language::Spanish, "es"),
    ] {
        let mut app = App::new(Rc::new(I18n::new(language)));
        app.loaded = true;
        app.workspace
            .create_space("Work".into(), vec![PathBuf::from("/project")]);
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        let before = ui
            .find(Id::new("project-files-panel"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        drop(ui);
        app.notifications
            .success(Key::GitFetchDone, Some("project".into()));
        app.notifications.success(
            Key::GitUpdateDone,
            Some(format!("feature/{}", "длинное-название-ветки-".repeat(8))),
        );
        app.notifications
            .error(Key::GitOperationFailed, Some("main".into()));
        for (visual_theme, appearance) in [(Theme::Dark, "dark"), (Theme::Light, "light")] {
            let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
            assert_eq!(
                before,
                ui.find(Id::new("project-files-panel"))
                    .unwrap()
                    .visible_bounds()
                    .unwrap()
            );
            let bounds = ui
                .find(Id::new("notifications"))
                .unwrap()
                .visible_bounds()
                .unwrap();
            assert_eq!(bounds.width, theme::NOTIFICATION_WIDTH);
            assert_eq!(
                bounds.x + bounds.width,
                crate::app::tests::content_width(&app, 1080.) - theme::NOTIFICATION_INSET
            );
            assert_eq!(bounds.y + bounds.height, 720. - theme::NOTIFICATION_INSET);
            assert!(bounds.y > 0.);
            for key in [
                Key::GitFetchDone,
                Key::GitUpdateDone,
                Key::GitOperationFailed,
            ] {
                let label = ui
                    .find(app.i18n.text(key))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                assert!(bounds.contains(label.center()));
            }
            if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
                ui.snapshot(&visual_theme)
                    .unwrap()
                    .matches_image(output.join(format!("notifications-{suffix}-{appearance}")))
                    .unwrap();
            }
            ui.point_at((bounds.x + 30., bounds.y + bounds.height - 4.));
            ui.simulate([
                iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ]);
            let messages: Vec<_> = ui.into_messages().collect();
            assert!(
                messages
                    .iter()
                    .all(|message| matches!(message, Message::Notification(Event::Hover(_, _))))
            );
            assert!(
                messages
                    .iter()
                    .any(|message| matches!(message, Message::Notification(Event::Hover(_, true))))
            );
        }
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        ui.click(Id::new("notification-close-3")).unwrap();
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        assert_eq!(app.notifications.items.len(), 2);
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        ui.click(Id::new("nav-settings")).unwrap();
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        assert_eq!(app.navigation, Navigation::Settings);
    }
}
