//! Keep command-click selection separate from ordinary controls and outside clicks.

use crate::app::{
    Message,
    branch_selection::{Event as SelectionEvent, Target},
    git_actions,
};
use iced::{
    Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
};
use std::cell::Cell;

pub(crate) fn target<'a>(
    content: impl Into<Element<'a, Message>>,
    command: bool,
    target: Target,
) -> Element<'a, Message> {
    Element::new(Input {
        content: content.into(),
        mode: Mode::Target(command, target),
    })
}

pub(crate) fn watch<'a>(content: Element<'a, Message>, active: bool) -> Element<'a, Message> {
    Element::new(Input {
        content,
        mode: Mode::Watch(active),
    })
}

enum Mode {
    Target(bool, Target),
    Watch(bool),
}
struct Input<'a> {
    content: Element<'a, Message>,
    mode: Mode,
}
#[derive(Default)]
struct State {
    pressed: bool,
}

impl Widget<Message, Theme, Renderer> for Input<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }
    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }
    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let pressed = matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(
                mouse::Button::Left | mouse::Button::Right
            ))
        );
        if pressed && shell.is_event_captured() {
            return;
        }
        let over = cursor
            .position()
            .is_some_and(|point| layout.bounds().contains(point) && viewport.contains(point));
        let state = tree.state.downcast_mut::<State>();
        if let Mode::Target(command, target) = &self.mode {
            if *command
                && over
                && matches!(
                    event,
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                )
            {
                state.pressed = true;
                shell.publish(Message::BranchSelection(SelectionEvent::Toggle(
                    target.clone(),
                )));
                shell.capture_event();
                return;
            }
            if state.pressed
                && matches!(
                    event,
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                )
            {
                state.pressed = false;
                shell.capture_event();
                return;
            }
            if matches!(event, Event::Window(iced::window::Event::Unfocused)) {
                state.pressed = false;
            }
        }
        if matches!(self.mode, Mode::Watch(true)) && pressed && over {
            let mut messages = Vec::new();
            let mut child = Shell::new(&mut messages);
            self.content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                layout,
                cursor,
                renderer,
                clipboard,
                &mut child,
                viewport,
            );
            let preserve = Cell::new(false);
            shell.merge(child, |message| {
                if matches!(
                    &message,
                    Message::BranchSelection(
                        SelectionEvent::Toggle(_) | SelectionEvent::Menu(_, _)
                    ) | Message::Git(git_actions::Message::Menu(_, _))
                ) {
                    preserve.set(true);
                }
                message
            });
            if !preserve.get() {
                shell.publish(Message::BranchSelection(SelectionEvent::Clear));
            }
        } else {
            self.content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }
    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}
