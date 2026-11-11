use super::*;
use iced::widget::{button, column, text_input};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Close,
    Edit(String),
    Background,
}

#[test]
fn escape_closes_dialogs_and_context_menus_with_focused_inputs() {
    for modal in [false, true] {
        let base = iced::widget::container(button("Background").on_press(Message::Background))
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        let contents = || {
            column![
                text_input("Name", "Draft")
                    .id("input")
                    .on_input(Message::Edit)
            ]
            .into()
        };
        let view = if modal {
            crate::ui::modal::overlay(base, contents(), 300., Message::Close)
        } else {
            overlay(
                base,
                Point::new(20., 20.),
                Size::new(300., 80.),
                contents,
                Message::Close,
            )
        };
        let mut ui = iced_test::Simulator::with_size(iced::Settings::default(), (520., 400.), view);
        ui.click(iced::widget::Id::new("input")).unwrap();
        ui.simulate([Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(keyboard::key::Named::Escape),
            modified_key: keyboard::Key::Named(keyboard::key::Named::Escape),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::Escape),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::NONE,
            repeat: false,
            text: None,
        })]);
        assert_eq!(ui.into_messages().collect::<Vec<_>>(), [Message::Close]);
    }
}
