use super::*;
use iced::{Color, widget::span};
use std::borrow::Cow;

#[test]
fn unselected_spans_keep_borrowed_text_and_existing_emphasis() {
    for range in [None, Some(2..2), Some(20..25)] {
        let spans = vec![
            span("let ").color(Color::WHITE),
            span("value").color(Color::BLACK).background(Color::WHITE),
        ];
        let result = highlight_spans(spans, range, Color::BLACK);
        assert_eq!(result.len(), 2);
        assert!(
            result
                .iter()
                .all(|span| matches!(span.text, Cow::Borrowed(_)))
        );
        assert_eq!(result[0].text, "let ");
        assert_eq!(result[1].text, "value");
        assert_eq!(result[0].color, Some(Color::WHITE));
        assert_eq!(result[1].color, Some(Color::BLACK));
        assert_eq!(
            result[1].highlight.as_ref().unwrap().background,
            Color::WHITE.into()
        );
    }
}

#[test]
fn selection_splits_unicode_spans_without_losing_syntax_colors() {
    let spans = vec![
        span("aé中").color(Color::WHITE),
        span("🙂z").color(Color::BLACK),
    ];
    let result = highlight_spans(spans, Some(1..10), Color::WHITE);
    assert_eq!(
        result
            .iter()
            .map(|span| span.text.as_ref())
            .collect::<Vec<_>>(),
        ["a", "é中", "🙂", "z"]
    );
    assert!(result[0].highlight.is_none());
    assert!(result[1].highlight.is_some());
    assert!(result[2].highlight.is_some());
    assert!(result[3].highlight.is_none());
    assert_eq!(result[1].color, Some(Color::WHITE));
    assert_eq!(result[2].color, Some(Color::BLACK));
}

#[tokio::test]
async fn pointer_layout_waits_for_selection_and_tracks_unicode_boundaries() {
    use iced::advanced::clipboard;
    use iced_test::core::renderer::Headless;
    let renderer = iced_test::renderer::Renderer::new(theme::font(), 16.into(), Some("tiny-skia"))
        .await
        .unwrap();
    let document = Document::plain("aé中🙂z".to_owned());
    let mut widget = Selectable {
        shortcuts: crate::domain::shortcuts::Shortcuts::default(),
        content: rich_text::<(), Message, _, _>([span(document.text.as_str())])
            .font(theme::code_font())
            .size(16)
            .line_height(1.65)
            .into(),
        value: document.text.clone(),
        document: &document,
        selection: Selection::default(),
        lane: Lane::Unified,
        row: 0,
        id: 7,
        size: 16.,
    };
    let mut tree = Tree::new(&widget as &dyn Widget<Message, Theme, Renderer>);
    let node = widget.layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, Size::new(400., 40.)),
    );
    let viewport = Rectangle::with_size(Size::new(400., 40.));
    let cursor = mouse::Cursor::Available(Point::new(1., 8.));
    let mut messages = Vec::new();
    for event in [
        Event::Window(iced::window::Event::RedrawRequested(
            iced::time::Instant::now(),
        )),
        Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0., y: -24. },
        }),
        Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(1., 8.),
        }),
    ] {
        widget.update(
            &mut tree,
            &event,
            Layout::new(&node),
            cursor,
            &renderer,
            &mut clipboard::Null,
            &mut Shell::new(&mut messages),
            &viewport,
        );
        assert_eq!(
            tree.state
                .downcast_ref::<text::paragraph::Plain<TextParagraph>>()
                .raw()
                .min_bounds(),
            Size::ZERO
        );
        assert!(messages.is_empty());
    }
    widget.update(
        &mut tree,
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Layout::new(&node),
        cursor,
        &renderer,
        &mut clipboard::Null,
        &mut Shell::new(&mut messages),
        &viewport,
    );
    let Message::CodeSelection(
        7,
        action @ Action::Start(Lane::Unified, Position { row: 0, byte: 0 }),
    ) = messages.pop().unwrap()
    else {
        panic!("Clicking the source must begin selection");
    };
    widget.selection.update(action, &document);
    for x in 1..100 {
        let point = Point::new(x as f32, 8.);
        widget.update(
            &mut tree,
            &Event::Mouse(mouse::Event::CursorMoved { position: point }),
            Layout::new(&node),
            mouse::Cursor::Available(point),
            &renderer,
            &mut clipboard::Null,
            &mut Shell::new(&mut messages),
            &viewport,
        );
        let Message::CodeSelection(7, Action::Extend(Lane::Unified, position)) =
            messages.pop().unwrap()
        else {
            panic!("Dragging must extend selection");
        };
        assert!(document.text.is_char_boundary(position.byte));
        widget
            .selection
            .update(Action::Extend(Lane::Unified, position), &document);
    }
    assert_eq!(
        widget.selection.copy(&document).as_deref(),
        Some(document.text.as_str())
    );
}
