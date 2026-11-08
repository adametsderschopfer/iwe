use super::*;
use crate::i18n::Language;

#[test]
fn greeting_fits_default_and_minimum_windows_in_every_language() {
    for (language, greeting) in [
        (Language::English, "Hello World!"),
        (Language::Russian, "Привет, мир!"),
        (Language::Spanish, "¡Hola, mundo!"),
    ] {
        let app = application(Rc::new(I18n::new(language)));
        let (state, _) = app.boot();
        let settings = Program::window(&app).expect("The application must open a window");

        assert_eq!(Program::title(&app, &state, window::Id::unique()), "IWE");
        for size in [settings.size, settings.min_size.unwrap()] {
            let mut ui =
                iced_test::Simulator::with_size(iced::Settings::default(), size, state.view());
            let bounds = ui
                .find(greeting)
                .expect("The translated greeting must be present")
                .visible_bounds()
                .expect("The greeting must be visible");
            assert!(bounds.x >= 0.0 && bounds.y >= 0.0);
            assert!(bounds.x + bounds.width <= size.width);
            assert!(bounds.y + bounds.height <= size.height);
            assert!((bounds.center_x() - size.width / 2.0).abs() < 1.0);
            assert!((bounds.center_y() - size.height / 2.0).abs() < 1.0);
        }
    }
}

#[test]
fn window_keeps_native_controls_and_platform_transparency() {
    let app = application(Rc::new(I18n::new(Language::English)));
    let settings = Program::window(&app).expect("The application must open a window");
    assert!(settings.decorations);
    assert!(settings.exit_on_close_request);
    assert_eq!(settings.transparent, cfg!(target_os = "macos"));
}

#[test]
fn background_stays_opaque_until_native_glass_is_ready() {
    let app = application(Rc::new(I18n::new(Language::English)));
    let (state, _) = app.boot();
    for theme in Theme::ALL {
        let style = Program::style(&app, &state, theme);
        assert_eq!(style.background_color.a, 1.0);
        assert_eq!(style.text_color.a, 1.0);
    }
}

#[test]
fn native_glass_result_controls_transparency_without_hiding_the_greeting() {
    let app = application(Rc::new(I18n::new(Language::English)));
    let (mut state, _) = app.boot();
    for (ready, alpha) in [(false, 1.0), (true, 0.0), (false, 1.0)] {
        let _ = state.update(Message::GlassConfigured(ready));
        let style = Program::style(&app, &state, &Theme::Dark);
        assert_eq!(style.background_color.a, alpha);
        assert_eq!(style.text_color.a, 1.0);
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            (400.0, 280.0),
            state.view(),
        );
        assert!(ui.find("Hello World!").unwrap().visible_bounds().is_some());
    }
}
