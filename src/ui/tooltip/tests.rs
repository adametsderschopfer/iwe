use super::*;
use iced::{
    Event, Point, Rectangle, Renderer, Size, Vector,
    advanced::{Layout, Shell, layout, mouse, widget::Tree},
    widget::{button, container, text},
};
use iced_test::core::{clipboard, renderer::Headless};

fn hint_bounds(
    renderer: &Renderer,
    position: Position,
    origin: Point,
) -> (Rectangle, Rectangle, Point) {
    let viewport = Rectangle::with_size(Size::new(480., 320.));
    let mut element: Element<'_, ()> = view(
        button("Project").on_press(()).width(100).height(40),
        text("/projects/company/backend").size(12),
        position,
    )
    .into();
    let mut tree = Tree::new(&element);
    let node = element
        .as_widget_mut()
        .layout(
            &mut tree,
            renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        )
        .move_to(origin);
    let control = node.bounds();
    let pointer = control.center();
    let cursor = mouse::Cursor::Available(pointer);
    let mut messages = Vec::new();
    element.as_widget_mut().update(
        &mut tree,
        &Event::Mouse(mouse::Event::CursorMoved { position: pointer }),
        Layout::new(&node),
        cursor,
        renderer,
        &mut clipboard::Null,
        &mut Shell::new(&mut messages),
        &viewport,
    );
    let hint = element
        .as_widget_mut()
        .overlay(
            &mut tree,
            Layout::new(&node),
            renderer,
            &viewport,
            Vector::ZERO,
        )
        .expect("hover opens a hint")
        .as_overlay_mut()
        .layout(renderer, viewport.size())
        .children()[0]
        .bounds();
    for event in [
        mouse::Event::ButtonPressed(mouse::Button::Left),
        mouse::Event::ButtonReleased(mouse::Button::Left),
    ] {
        element.as_widget_mut().update(
            &mut tree,
            &Event::Mouse(event),
            Layout::new(&node),
            cursor,
            renderer,
            &mut clipboard::Null,
            &mut Shell::new(&mut messages),
            &viewport,
        );
    }
    assert_eq!(messages, vec![()], "hover must preserve button actions");
    (control, hint, pointer)
}

#[tokio::test]
async fn hover_hints_keep_a_visible_gap_in_every_direction() {
    let renderer = Renderer::new(iced::Font::DEFAULT, 16.into(), Some("tiny-skia"))
        .await
        .unwrap();
    for position in [
        Position::Top,
        Position::Bottom,
        Position::Left,
        Position::Right,
        Position::FollowCursor,
    ] {
        let (control, hint, pointer) = hint_bounds(&renderer, position, Point::new(190., 140.));
        let gap = match position {
            Position::Top => control.y - hint.y - hint.height,
            Position::Bottom => hint.y - control.y - control.height,
            Position::Left => control.x - hint.x - hint.width,
            Position::Right => hint.x - control.x - control.width,
            Position::FollowCursor => pointer.y - hint.y - hint.height,
        };
        assert!((gap - 8.).abs() < 0.1, "{position:?}: gap={gap}");
    }
}

#[tokio::test]
async fn hover_hints_remain_inside_the_window_at_its_edges() {
    let renderer = Renderer::new(iced::Font::DEFAULT, 16.into(), Some("tiny-skia"))
        .await
        .unwrap();
    for (position, origin) in [
        (Position::Top, Point::ORIGIN),
        (Position::Left, Point::ORIGIN),
        (Position::Bottom, Point::new(380., 280.)),
        (Position::Right, Point::new(380., 280.)),
        (Position::FollowCursor, Point::ORIGIN),
    ] {
        let (_, hint, _) = hint_bounds(&renderer, position, origin);
        assert!(hint.x >= 0. && hint.y >= 0., "{position:?}: {hint:?}");
        assert!(hint.x + hint.width <= 480. && hint.y + hint.height <= 320.);
    }
}

#[test]
fn tooltip_snapshot() {
    let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(std::path::PathBuf::from) else {
        return;
    };
    let element: Element<'_, ()> = container(view(
        button(text("iwe").size(16))
            .width(220)
            .height(48)
            .style(super::super::theme::action(true))
            .on_press(()),
        text("/Users/v_adamets/projects/iwe").size(12),
        Position::Right,
    ))
    .padding(60)
    .into();
    let mut ui = iced_test::Simulator::with_size(iced::Settings::default(), (620., 180.), element);
    ui.point_at((100., 80.));
    ui.simulate([Event::Mouse(mouse::Event::CursorMoved {
        position: Point::new(100., 80.),
    })]);
    for (theme, name) in [(iced::Theme::Dark, "dark"), (iced::Theme::Light, "light")] {
        ui.snapshot(&theme)
            .unwrap()
            .matches_image(output.join(format!("tooltip-gap-{name}")))
            .unwrap();
    }
}
