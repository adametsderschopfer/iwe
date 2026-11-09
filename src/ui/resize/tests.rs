use super::*;
use iced::{
    Point,
    widget::{button, container, row, space, text},
};

#[derive(Clone, Debug, PartialEq)]
enum Message {
    Resize(f32, bool),
    Click,
}

fn view(enabled: bool) -> Element<'static, Message> {
    horizontal(
        row![
            // The left button models a scrollbar or an action beside the divider.
            button(text("left"))
                .on_press(Message::Click)
                .width(100)
                .height(100),
            container(space()).width(1),
            button(text("right"))
                .on_press(Message::Click)
                .width(Length::Fill)
                .height(100),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into(),
        enabled.then_some(100.),
        100.,
        80. ..=240.,
        Message::Resize,
    )
}

fn move_to(ui: &mut iced_test::Simulator<'_, Message>, position: Point) {
    ui.point_at(position);
    ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position })]);
}

#[test]
fn drag_tracks_outside_the_panel_and_commits_only_on_release() {
    let mut ui =
        iced_test::Simulator::with_size(iced::Settings::default(), (400., 150.), view(true));
    ui.point_at((100.5, 60.));
    ui.simulate([Event::Mouse(mouse::Event::ButtonPressed(
        mouse::Button::Left,
    ))]);
    move_to(&mut ui, Point::new(180.5, 200.));
    ui.simulate([Event::Mouse(mouse::Event::ButtonReleased(
        mouse::Button::Left,
    ))]);
    move_to(&mut ui, Point::new(120., 60.));
    assert_eq!(
        ui.into_messages().collect::<Vec<_>>(),
        vec![Message::Resize(180., false), Message::Resize(180., true)]
    );
}

#[test]
fn drag_clamps_width_and_finishes_when_the_window_loses_focus() {
    for ending in [
        Event::Window(window::Event::Unfocused),
        Event::Mouse(mouse::Event::CursorLeft),
    ] {
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (400., 150.), view(true));
        ui.point_at((100., 60.));
        ui.simulate([Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Left,
        ))]);
        move_to(&mut ui, Point::new(900., 60.));
        move_to(&mut ui, Point::new(-100., 60.));
        ui.simulate([ending]);
        move_to(&mut ui, Point::new(200., 60.));
        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            vec![
                Message::Resize(240., false),
                Message::Resize(80., false),
                Message::Resize(80., true)
            ]
        );
    }
}

#[test]
fn clicks_beside_the_divider_and_hidden_panels_do_not_start_resizing() {
    for (enabled, x) in [(true, 97.), (false, 103.)] {
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (400., 150.), view(enabled));
        ui.point_at((x, 60.));
        ui.simulate([
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ]);
        move_to(&mut ui, Point::new(180., 60.));
        assert_eq!(ui.into_messages().collect::<Vec<_>>(), vec![Message::Click]);
    }
}
