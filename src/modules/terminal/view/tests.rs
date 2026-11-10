use super::*;
use iced::keyboard::{Key, Modifiers, key::Named};

#[test]
fn terminal_input_supports_unicode_interrupts_and_cursor_modes() {
    assert_eq!(
        input::physical_control(keyboard::key::Physical::Code(keyboard::key::Code::KeyC)),
        Some(vec![3])
    );
    assert_eq!(
        input::encode(
            &Key::Character("ж".into()),
            Some("Ж"),
            Modifiers::SHIFT,
            false
        ),
        Some("Ж".as_bytes().to_vec())
    );
    assert_eq!(
        input::encode(&Key::Character("c".into()), None, Modifiers::CTRL, false),
        Some(vec![3])
    );
    assert_eq!(
        input::encode(&Key::Named(Named::ArrowUp), None, Modifiers::empty(), false),
        Some(b"\x1b[A".to_vec())
    );
    assert_eq!(
        input::encode(&Key::Named(Named::ArrowUp), None, Modifiers::empty(), true),
        Some(b"\x1bOA".to_vec())
    );
    assert_eq!(
        input::encode(&Key::Named(Named::Tab), None, Modifiers::SHIFT, false),
        Some(b"\x1b[Z".to_vec())
    );
    assert!(input::encode(&Key::Character("w".into()), None, Modifiers::LOGO, false).is_none());
}

#[test]
fn escape_sequences_update_screen_colors_cursor_and_scrollback() {
    let mut parser = vt100::Parser::new(3, 12, 20);
    parser.process(b"\x1b[31mred\x1b[0m\r\nline2\r\nline3\r\nline4");
    assert!(parser.screen().contents().contains("line4"));
    parser.screen_mut().set_scrollback(20);
    assert!(parser.screen().contents().contains("red"));
    assert_eq!(
        parser.screen().cell(0, 0).unwrap().fgcolor(),
        vt100::Color::Idx(1)
    );
    parser.screen_mut().set_scrollback(0);
    parser.process(b"\x1b[2J\x1b[Hnew");
    assert_eq!(parser.screen().contents(), "new");
    assert_eq!(parser.screen().cursor_position(), (0, 3));
}

#[test]
fn selection_copies_cells_without_escape_sequences_or_wide_continuations() {
    let mut parser = vt100::Parser::new(3, 12, 0);
    parser.process("\x1b[32mab界\r\ncd".as_bytes());
    let screen = Screen {
        screen: parser.screen(),
        request: 1,
        size: 14.,
        interactive: true,
        running: true,
    };
    let state = State {
        anchor: Some((0, 1)),
        selection: Some((1, 1)),
        ..State::default()
    };
    assert_eq!(screen.selected(&state), "b界\ncd");
}

#[test]
fn terminal_canvas_sends_keys_only_after_a_focus_click() {
    let parser = vt100::Parser::new(24, 80, 0);
    let screen = Screen {
        screen: parser.screen(),
        request: 42,
        size: 14.,
        interactive: true,
        running: true,
    };
    let content = container(
        canvas::Canvas::new(screen)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .id("input-screen")
    .width(Length::Fill)
    .height(Length::Fill);
    let mut ui = iced_test::Simulator::with_size(iced::Settings::default(), (800., 500.), content);
    ui.tap_key(keyboard::Key::Character("x".into()));
    ui.click(iced::widget::Id::new("input-screen")).unwrap();
    ui.tap_key(keyboard::Key::Character("y".into()));
    let events = ui.into_messages().collect::<Vec<_>>();
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Input(_, bytes) if bytes == b"x"))
    );
    assert!(events.iter().any(|event| matches!(event, Event::Focus(42))));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Input(42, bytes) if bytes == b"y"))
    );
}
