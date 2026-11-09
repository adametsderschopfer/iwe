use super::*;
use crate::i18n::{I18n, Language};
use std::rc::Rc;

fn app() -> App {
    App::new(Rc::new(I18n::new(Language::English)))
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

#[tokio::test]
async fn confirmed_handoff_exits_the_event_loop_and_failed_handoff_keeps_the_app_open() {
    use iced::futures::StreamExt;
    use iced_test::runtime::{Action, task};
    for success in [true, false] {
        let mut app = app();
        app.updates = State::Installing(Arc::new(Prepared::fixture()));
        let result = if success { Ok(()) } else { Err(Error::Install) };
        let task = app.update_release(Event::Started(result));
        if success {
            let mut stream = task::into_stream(task).unwrap();
            assert!(matches!(stream.next().await, Some(Action::Exit)));
        } else {
            assert_eq!(task.units(), 0);
            assert!(matches!(app.updates, State::Failed(Error::Install)));
        }
    }
}
