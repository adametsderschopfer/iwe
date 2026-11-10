use super::*;
use crate::i18n::{I18n, Language};
use std::rc::Rc;

fn app() -> App {
    App::new(Rc::new(I18n::new(Language::English)))
}

#[test]
fn automatic_checks_throttle_focus_and_retry_failures_after_fifteen_minutes() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10_000);
    let mut schedule = Schedule::default();
    assert!(schedule.due(Trigger::Periodic, now));
    schedule.last_attempt = Some(now);
    assert!(!schedule.due(Trigger::Focused, now + Duration::from_secs(899)));
    assert!(schedule.due(Trigger::Focused, now + RETRY_INTERVAL));
    assert!(!schedule.due(Trigger::Periodic, now + RETRY_INTERVAL));
    assert!(schedule.due(Trigger::Periodic, now + CHECK_INTERVAL));
    schedule.retry = true;
    assert!(!schedule.due(Trigger::Periodic, now + Duration::from_secs(899)));
    assert!(schedule.due(Trigger::Periodic, now + RETRY_INTERVAL));
    assert!(schedule.due(Trigger::Periodic, now - Duration::from_secs(1)));
}

#[test]
fn manual_checks_bypass_cooldown_and_automatic_checks_preserve_available_updates() {
    let mut app = app();
    assert!(app.update_release(Event::Check).units() > 0);
    let _ = app.update_release(Event::Checked(Ok(None)));
    assert_eq!(
        app.update_release(Event::Automatic(Trigger::Focused))
            .units(),
        0
    );
    assert!(app.update_release(Event::Check).units() > 0);
    let _ = app.update_release(Event::Checked(Ok(Some(release()))));
    app.update_schedule.last_attempt = None;
    assert_eq!(
        app.update_release(Event::Automatic(Trigger::Periodic))
            .units(),
        0
    );
    assert!(matches!(app.updates, State::Available(_)));
}
fn release() -> Release {
    Release {
        version: "0.1.11".into(),
        tag: "0.1.11".into(),
        asset: "test".into(),
        size: 1,
        sha256: String::new(),
        manifest: vec![],
        signature: vec![],
    }
}

#[test]
fn checks_ignore_duplicate_requests_and_obsolete_completions() {
    let mut app = app();
    assert!(app.update_release(Event::Check).units() > 0);
    assert_eq!(app.update_release(Event::Check).units(), 0);
    let _ = app.update_release(Event::Checked(Ok(Some(release()))));
    assert!(matches!(app.updates, State::Available(_)));
    let _ = app.update_release(Event::Checked(Ok(None)));
    assert!(matches!(app.updates, State::Available(_)));
    assert!(app.update_release(Event::Download).units() > 0);
    assert_eq!(app.update_release(Event::Download).units(), 0);
    let _ = app.update_release(Event::Downloaded(Err(Error::Invalid)));
    assert!(matches!(app.updates, State::Failed(Error::Invalid)));
}

#[test]
fn update_actions_are_visible_in_all_languages_and_do_not_close_the_window_when_downloading() {
    use iced::widget::Id;
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let mut app = App::new(Rc::new(I18n::new(language)));
        app.loaded = true;
        app.updates = State::Available(release());
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (1080., 720.), app.view());
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(std::path::PathBuf::from) {
            for (theme, suffix) in [(iced::Theme::Dark, "dark"), (iced::Theme::Light, "light")] {
                ui.snapshot(&theme)
                    .unwrap()
                    .matches_image(output.join(format!("update-sidebar-{language:?}-{suffix}")))
                    .unwrap();
            }
        }
        let update = ui
            .find(app.i18n.text(crate::i18n::Key::DownloadUpdate))
            .unwrap()
            .visible_bounds()
            .unwrap();
        let settings = ui
            .find(app.i18n.text(crate::i18n::Key::Settings))
            .unwrap()
            .visible_bounds()
            .unwrap();
        assert_eq!(update.x, settings.x);
        let update_row = ui
            .find(Id::new("sidebar-update"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        let settings_row = ui
            .find(Id::new("nav-settings"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        assert_eq!(update_row.height, settings_row.height);
        ui.click(Id::new("sidebar-update")).unwrap();
        let messages: Vec<_> = ui.into_messages().collect();
        assert!(matches!(
            messages.as_slice(),
            [Message::Update(Event::Download)]
        ));
        for message in messages {
            let _ = app.update(message);
        }
        assert!(matches!(app.updates, State::Downloading));
        app.navigation = super::super::Navigation::Settings;
        app.settings_tab = super::super::SettingsTab::About;
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (1080., 720.), app.view());
        assert!(ui.find(app.i18n.text(crate::i18n::Key::Updates)).is_ok());
    }
}

#[test]
fn failed_handoff_keeps_the_app_open_and_reports_the_failure() {
    let mut app = app();
    app.updates = State::Installing(Arc::new(Prepared::fixture()));
    let task = app.update(Message::Update(Event::InstallFailed(Error::Install)));
    assert_eq!(task.units(), 0);
    assert!(matches!(app.updates, State::Failed(Error::Install)));
    assert!(!app.notifications.items.is_empty());
}

#[test]
fn installing_waits_for_the_latest_workspace_save_and_preserves_failed_saves() {
    use crate::domain::Failure;
    for success in [true, false] {
        let mut app = app();
        app.window_id = Some(iced::window::Id::unique());
        app.updates = State::Ready(Arc::new(Prepared::fixture()));
        // A save already in flight must finish before the latest snapshot is written.
        app.saving = true;
        let _ = app.update(Message::Update(Event::Install));
        assert!(matches!(app.updates, State::Saving(_)));
        assert!(app.save_pending);
        let first = app.update(Message::Saved(Ok(())));
        assert!(first.units() > 0);
        assert!(matches!(app.updates, State::Saving(_)));
        assert!(app.saving);
        assert!(!app.save_pending);
        let next = app.update(Message::Saved(if success {
            Ok(())
        } else {
            Err(Failure::Storage)
        }));
        if success {
            assert!(matches!(app.updates, State::Installing(_)));
            assert!(next.units() > 0);
        } else {
            assert!(matches!(app.updates, State::Failed(Error::Install)));
            assert_eq!(next.units(), 0);
        }
    }
}
