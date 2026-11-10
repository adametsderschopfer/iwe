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
        revision: 0,
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
fn terminal_screen_revisions_track_resize_and_real_scrollback_changes() {
    let mut terminal = TerminalState::default();
    let _ = terminal.update(Event::Start, Some(std::path::Path::new("project")));
    let request = terminal.active.unwrap();
    let revision = terminal.active_tab().unwrap().revision;
    let _ = terminal.update(Event::Resize(request, 24, 80), None);
    let _ = terminal.update(Event::Scroll(request, 0), None);
    assert_eq!(terminal.active_tab().unwrap().revision, revision);
    let _ = terminal.update(Event::Resize(request, 3, 12), None);
    assert_ne!(terminal.active_tab().unwrap().revision, revision);
    terminal.tabs[0].parser.process(b"1\r\n2\r\n3\r\n4");
    let revision = terminal.active_tab().unwrap().revision;
    let _ = terminal.update(Event::Scroll(request, 2), None);
    assert!(terminal.active_tab().unwrap().parser.screen().scrollback() > 0);
    assert_ne!(terminal.active_tab().unwrap().revision, revision);
    let revision = terminal.active_tab().unwrap().revision;
    let _ = terminal.update(Event::Hide, None);
    assert_eq!(terminal.active_tab().unwrap().revision, revision);
}

#[tokio::test]
async fn terminal_redraw_reuses_geometry_and_invalidates_each_visual_change() {
    use iced::widget::canvas::Program;
    use iced_test::core::renderer::Headless;
    let renderer = Renderer::new(theme::font(), 16.into(), Some("tiny-skia"))
        .await
        .unwrap();
    let mut parser = vt100::Parser::new(3, 12, 0);
    parser.process(b"cached screen");
    let mut screen = Screen {
        screen: parser.screen(),
        request: 1,
        revision: 0,
        size: 14.,
        interactive: true,
        running: true,
    };
    let mut state = State::default();
    let mut bounds = Rectangle::with_size(Size::new(240., 120.));
    for _ in 0..100 {
        screen.draw(
            &state,
            &renderer,
            &Theme::Dark,
            bounds,
            mouse::Cursor::Unavailable,
        );
    }
    assert_eq!(state.paints.get(), 1);
    for change in 0..10 {
        let mut appearance = Theme::Dark;
        match change {
            0 => screen.revision += 1,
            1 => screen.request += 1,
            2 => state.focused = true,
            3 => {
                state.anchor = Some((0, 0));
                state.selection = Some((0, 3));
            }
            4 => screen.interactive = false,
            5 => screen.size = 16.,
            6 => appearance = Theme::Light,
            7 => {}
            8 => bounds.width += 40.,
            9 => {
                screen.interactive = true;
                screen.running = false;
                state.selection = None;
            }
            _ => unreachable!(),
        }
        let before = state.paints.get();
        screen.draw(
            &state,
            &renderer,
            &appearance,
            bounds,
            mouse::Cursor::Unavailable,
        );
        assert_eq!(state.paints.get(), before + 1, "change {change}");
        screen.draw(
            &state,
            &renderer,
            &appearance,
            bounds,
            mouse::Cursor::Unavailable,
        );
        assert_eq!(state.paints.get(), before + 1);
    }
}

#[test]
fn terminal_canvas_sends_keys_only_after_a_focus_click() {
    let parser = vt100::Parser::new(24, 80, 0);
    let screen = Screen {
        screen: parser.screen(),
        request: 42,
        revision: 0,
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
