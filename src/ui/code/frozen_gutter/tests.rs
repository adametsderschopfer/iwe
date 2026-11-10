use super::*;
use iced::{
    mouse as input,
    widget::{button, row, scrollable, space, text},
};

#[derive(Debug, Clone)]
enum Message {
    Click,
    Scrolled(f32),
}

#[test]
fn horizontally_scrolled_source_cannot_receive_clicks_under_the_fixed_gutter() {
    let content = row![
        space().width(60),
        button(text("source text"))
            .on_press(Message::Click)
            .width(540)
            .height(30),
    ]
    .width(600)
    .height(30);
    let view = scrollable(view(content, 60.))
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::default(),
        ))
        .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().x))
        .width(200)
        .height(50);
    let mut ui = iced_test::Simulator::with_size(iced::Settings::default(), (200., 50.), view);
    ui.point_at(iced::Point::new(100., 15.));
    ui.simulate([Event::Mouse(input::Event::WheelScrolled {
        delta: input::ScrollDelta::Pixels { x: -120., y: 0. },
    })]);
    ui.point_at(iced::Point::new(20., 15.));
    ui.simulate([
        Event::Mouse(input::Event::ButtonPressed(input::Button::Left)),
        Event::Mouse(input::Event::ButtonReleased(input::Button::Left)),
    ]);
    ui.point_at(iced::Point::new(100., 15.));
    ui.simulate([
        Event::Mouse(input::Event::ButtonPressed(input::Button::Left)),
        Event::Mouse(input::Event::ButtonReleased(input::Button::Left)),
    ]);
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        messages
            .iter()
            .any(|message| matches!(message, Message::Scrolled(x) if *x > 60.))
    );
    assert_eq!(
        messages
            .iter()
            .filter(|message| matches!(message, Message::Click))
            .count(),
        1
    );
}

#[tokio::test]
async fn horizontal_scrolling_keeps_gutter_pixels_fixed_while_source_moves() {
    use iced::{
        Color, Font, Size,
        futures::StreamExt,
        widget::{container, operation},
    };
    use iced_test::{
        core::renderer::Headless,
        runtime::{Action, UserInterface, task, user_interface},
    };

    let mut renderer =
        iced_test::renderer::Renderer::new(Font::MONOSPACE, 16.into(), Some("tiny-skia"))
            .await
            .unwrap();
    for gutter_width in [35., 65., 91., 115.] {
        let row =
            container(
                row![
            text("123").font(Font::MONOSPACE).width(gutter_width),
            text("Start: abcdefghijklmnopqrstuvwxyz 0123456789 another different source segment")
                .font(Font::MONOSPACE).wrapping(text::Wrapping::None),
        ]
                .height(30)
                .width(900),
            )
            .style(crate::ui::theme::diff_row(Some(
                crate::ui::code::LineChange::Added,
            )));
        let element: Element<'_, ()> = scrollable(view(row, gutter_width))
            .id("frozen-source")
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::default(),
            ))
            .width(240)
            .height(70)
            .into();
        let mut ui = UserInterface::build(
            element,
            Size::new(240., 70.),
            user_interface::Cache::default(),
            &mut renderer,
        );
        let style = renderer::Style {
            text_color: Color::WHITE,
        };
        ui.draw(
            &mut renderer,
            &Theme::Dark,
            &style,
            mouse::Cursor::Unavailable,
        );
        let before = renderer.screenshot(Size::new(240, 70), 1., Color::BLACK);
        let task: iced::Task<()> = operation::scroll_to(
            "frozen-source",
            operation::AbsoluteOffset { x: 120., y: 0. },
        );
        let mut actions = task::into_stream(task).unwrap();
        while let Some(action) = actions.next().await {
            if let Action::Widget(mut operation) = action {
                ui.operate(&renderer, operation.as_mut());
            }
        }
        ui.draw(
            &mut renderer,
            &Theme::Dark,
            &style,
            mouse::Cursor::Unavailable,
        );
        let after = renderer.screenshot(Size::new(240, 70), 1., Color::BLACK);
        for y in 0..30 {
            let start = y * 240 * 4;
            let end = start + gutter_width as usize * 4;
            assert_eq!(
                &before[start..end],
                &after[start..end],
                "The gutter must remain at its viewport position"
            );
        }
        assert!(
            (0..30).any(|y| {
                let start = (y * 240 + gutter_width as usize) * 4;
                let end = (y * 240 + 230) * 4;
                before[start..end] != after[start..end]
            }),
            "The source must move independently of its gutter"
        );
    }
}
