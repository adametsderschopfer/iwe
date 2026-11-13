use super::*;
use crate::{
    app::{App, Message, SettingsTab, spaces},
    i18n::{I18n, Language},
};
use iced::{Point, window};
use std::rc::Rc;

fn app() -> App {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.window_id = Some(window::Id::unique());
    app.loaded = true;
    app
}

#[test]
fn header_menus_open_exclusively_and_close_on_other_messages() {
    let mut app = app();
    let first = Point::new(12., 40.);
    let _ = app.update(Message::OpenMenuBar(Menu::File, first));
    assert_eq!(app.menu_bar, Some((Menu::File, first)));
    let second = Point::new(120., 40.);
    let _ = app.update(Message::OpenMenuBar(Menu::View, second));
    assert_eq!(app.menu_bar, Some((Menu::View, second)));
    let _ = app.update(Message::CloseMenuBar);
    assert_eq!(app.menu_bar, None);
    let _ = app.update(Message::OpenMenuBar(Menu::Help, first));
    let _ = app.update(Message::Refresh);
    assert_eq!(app.menu_bar, None);
}

#[test]
fn opening_a_header_menu_closes_anchored_menus() {
    let mut app = app();
    app.space_menu = Some((0, Point::new(4., 4.)));
    let _ = app.update(Message::OpenMenuBar(Menu::File, Point::new(12., 40.)));
    assert!(app.space_menu.is_none());
    assert_eq!(app.menu_bar, Some((Menu::File, Point::new(12., 40.))));
}

#[test]
fn about_action_opens_the_settings_window_on_the_about_tab() {
    let mut app = app();
    let _ = app.update(Message::OpenMenuBar(Menu::Help, Point::new(1., 1.)));
    let _ = app.update(Message::MenuAbout);
    assert_eq!(app.menu_bar, None);
    assert_eq!(app.settings_tab, SettingsTab::About);
    assert!(app.windows.settings.is_some());
}

#[test]
fn menu_actions_track_dialogs_like_the_native_menu() {
    let mut app = app();
    app.workspace.create_space("Work".into(), vec![]);
    assert!(app.menu_actions_enabled());
    let _ = app.update_space(spaces::Message::New);
    assert!(!app.menu_actions_enabled());
}

#[test]
fn header_menu_titles_open_their_menus() {
    let mut app = app();
    let id = app.window_id.unwrap();
    let mut ui = iced_test::Simulator::with_size(
        iced::Settings::default(),
        (1280., 800.),
        app.view_window(id),
    );
    ui.click(iced::widget::Id::new("menu-bar-file")).unwrap();
    assert!(
        ui.into_messages()
            .any(|message| matches!(message, Message::OpenMenuBar(Menu::File, _)))
    );
}
