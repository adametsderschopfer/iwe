use super::*;
use iced::widget::{button, row, scrollable, text};

#[derive(Clone, Debug, PartialEq)]
enum Message {
    Click(usize),
    Action(usize),
    Move(usize, usize),
    Scroll(f32),
}

fn view() -> Element<'static, Message> {
    let rows = Column::with_children((0..8).map(|index| {
        row![
            button(text(index))
                .width(Length::Fill)
                .height(40)
                .on_press(Message::Click(index)),
            button("×")
                .width(32)
                .height(40)
                .on_press(Message::Action(index)),
        ]
        .spacing(4)
        .into()
    }))
    .spacing(4);
    scrollable(column(rows, 36., Message::Move, Message::Scroll))
        .height(188)
        .width(240)
        .into()
}

fn drag(ui: &mut iced_test::Simulator<Message>, from: Point, to: Point) {
    ui.point_at(from);
    ui.simulate([Event::Mouse(mouse::Event::ButtonPressed(
        mouse::Button::Left,
    ))]);
    ui.point_at(to);
    ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position: to })]);
}

#[test]
fn dropping_rows_moves_in_both_directions_without_clicking() {
    for (from, to, expected) in [
        (
            Point::new(100., 20.),
            Point::new(100., 120.),
            Message::Move(0, 2),
        ),
        (
            Point::new(100., 108.),
            Point::new(100., 2.),
            Message::Move(2, 0),
        ),
    ] {
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (280., 240.), view());
        drag(&mut ui, from, to);
        ui.simulate([Event::Mouse(mouse::Event::ButtonReleased(
            mouse::Button::Left,
        ))]);
        assert_eq!(ui.into_messages().collect::<Vec<_>>(), [expected]);
    }
}

#[test]
fn clicks_and_trailing_actions_keep_their_original_behavior() {
    for (position, expected) in [
        (Point::new(100., 20.), Message::Click(0)),
        (Point::new(222., 20.), Message::Action(0)),
    ] {
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (280., 240.), view());
        ui.point_at(position);
        ui.simulate([
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ]);
        assert_eq!(ui.into_messages().collect::<Vec<_>>(), [expected]);
    }
}

#[test]
fn drops_outside_the_viewport_and_unchanged_positions_do_nothing() {
    for destination in [
        Point::new(100., 32.),
        Point::new(100., 210.),
        Point::new(260., 108.),
    ] {
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (280., 240.), view());
        drag(&mut ui, Point::new(100., 20.), destination);
        ui.simulate([Event::Mouse(mouse::Event::ButtonReleased(
            mouse::Button::Left,
        ))]);
        assert!(ui.into_messages().next().is_none());
    }
}

#[test]
fn losing_focus_cancels_dragging() {
    let mut ui = iced_test::Simulator::with_size(iced::Settings::default(), (280., 240.), view());
    drag(&mut ui, Point::new(100., 20.), Point::new(100., 120.));
    ui.simulate([
        Event::Window(window::Event::Unfocused),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ]);
    assert!(ui.into_messages().next().is_none());
}

#[test]
fn dragging_at_the_visible_edge_requests_scrolling() {
    let mut ui = iced_test::Simulator::with_size(iced::Settings::default(), (280., 240.), view());
    drag(&mut ui, Point::new(100., 20.), Point::new(100., 183.));
    ui.simulate([Event::Window(window::Event::RedrawRequested(
        Instant::now() + Duration::from_millis(32),
    ))]);
    assert!(
        ui.into_messages()
            .any(|message| matches!(message, Message::Scroll(offset) if offset > 0.))
    );
}

#[test]
fn scrolled_rows_keep_small_pointer_movements_as_clicks() {
    for grouped in [false, true] {
        let content = if grouped {
            let groups = Column::with_children((0..4).map(|group| {
                Column::new()
                    .push(
                        button(text(format!("Space {group}")))
                            .height(38)
                            .width(Length::Fill)
                            .on_press(Message::Action(group)),
                    )
                    .push(column(
                        Column::with_children((0..4).map(|index| {
                            button(text(format!("Project {}", group * 4 + index)))
                                .height(40)
                                .width(Length::Fill)
                                .on_press(Message::Click(group * 4 + index))
                                .into()
                        })),
                        0.,
                        Message::Move,
                        Message::Scroll,
                    ))
                    .into()
            }));
            scrollable(super::groups(groups, 38., Message::Move, Message::Scroll))
                .width(240)
                .height(188)
                .into()
        } else {
            view()
        };
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (280., 240.), content);
        ui.point_at(Point::new(100., 90.));
        ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0., y: -1000. },
        })]);
        let label = if grouped { "Project 15" } else { "7" };
        let center = ui.find(label).unwrap().visible_bounds().unwrap().center();
        drag(&mut ui, center, Point::new(center.x + 1., center.y + 1.));
        ui.simulate([Event::Mouse(mouse::Event::ButtonReleased(
            mouse::Button::Left,
        ))]);
        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            [Message::Click(if grouped { 15 } else { 7 })]
        );
    }
}
