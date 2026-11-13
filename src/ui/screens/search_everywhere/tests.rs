use super::*;
use crate::{
    app::search_everywhere::{Dialog, Scope},
    domain::workspace::{Project, Space},
    i18n::{I18n, Language},
};
use iced::{
    keyboard,
    widget::{button, container},
};
use std::{path::PathBuf, rc::Rc};

fn app() -> App {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.workspace.spaces = vec![Space {
        name: "Work".into(),
        projects: vec![Project {
            name: "Alpha".into(),
            directory: PathBuf::from("/tmp/alpha"),
            pinned: false,
        }],
        deployments: Vec::new(),
    }];
    app.workspace.select_project(0, 0);
    app.search_everywhere.dialog = Some(Dialog {
        scope: Scope::All,
        query: "ma".into(),
        rows: vec![Row::File {
            name: "main.rs".into(),
            directory: String::new(),
            path: PathBuf::from("/tmp/alpha/main.rs"),
        }],
        selection: 0,
    });
    app
}

fn key_press(named: keyboard::key::Named, code: keyboard::key::Code) -> iced::Event {
    iced::Event::Keyboard(keyboard::Event::KeyPressed {
        key: keyboard::Key::Named(named),
        modified_key: keyboard::Key::Named(named),
        physical_key: keyboard::key::Physical::Code(code),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::NONE,
        repeat: false,
        text: None,
    })
}

#[test]
fn popup_keys_map_to_search_events() {
    use keyboard::key::Named;
    let press = |named: Named| match key_press(named, keyboard::key::Code::Escape) {
        iced::Event::Keyboard(event) => event,
        _ => unreachable!(),
    };
    assert_eq!(
        key_event(&press(Named::ArrowDown)),
        Some(Event::Moved(true))
    );
    assert_eq!(key_event(&press(Named::ArrowUp)), Some(Event::Moved(false)));
    assert_eq!(key_event(&press(Named::Enter)), Some(Event::Selected));
    assert_eq!(key_event(&press(Named::Tab)), Some(Event::CycleScope));
    assert_eq!(key_event(&press(Named::Escape)), Some(Event::Cancel));
    // Typing keys stay with the input.
    assert_eq!(key_event(&press(Named::Backspace)), None);
}

#[test]
fn escape_and_arrows_reach_the_popup_with_a_focused_input() {
    for (named, code, expected) in [
        (
            keyboard::key::Named::Escape,
            keyboard::key::Code::Escape,
            Event::Cancel,
        ),
        (
            keyboard::key::Named::ArrowDown,
            keyboard::key::Code::ArrowDown,
            Event::Moved(true),
        ),
    ] {
        let app = app();
        let base = container(button("Background"))
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        let view = overlay(&app, base);
        let mut ui = iced_test::Simulator::with_size(iced::Settings::default(), (900., 700.), view);
        ui.click(iced::widget::Id::new(INPUT_ID)).unwrap();
        ui.simulate([key_press(named, code)]);
        let messages = ui.into_messages().collect::<Vec<_>>();
        assert_eq!(messages.len(), 1);
        assert!(
            matches!(&messages[0], Message::SearchEverywhere(event) if *event == expected),
            "expected {expected:?}, got {messages:?}"
        );
    }
}
