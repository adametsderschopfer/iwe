//! Resize the file panel without rebuilding its scroll state or the source preview.

use super::theme;
use iced::{
    Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    window,
};
use std::ops::RangeInclusive;

pub(crate) fn horizontal<'a, Message: 'a>(
    content: Element<'a, Message>,
    width: Option<f32>,
    offset: f32,
    limits: RangeInclusive<f32>,
    on_resize: impl Fn(f32, bool) -> Message + 'a,
) -> Element<'a, Message> {
    Element::new(Resize {
        content,
        width,
        offset,
        limits,
        on_resize: Box::new(on_resize),
        direction: 1.,
    })
}

pub(crate) fn right<'a, Message: 'a>(
    content: Element<'a, Message>,
    width: Option<f32>,
    offset: f32,
    limits: RangeInclusive<f32>,
    on_resize: impl Fn(f32, bool) -> Message + 'a,
) -> Element<'a, Message> {
    Element::new(Resize {
        content,
        width,
        offset,
        limits,
        on_resize: Box::new(on_resize),
        direction: -1.,
    })
}

struct Resize<'a, Message> {
    content: Element<'a, Message>,
    width: Option<f32>,
    offset: f32,
    limits: RangeInclusive<f32>,
    on_resize: Box<dyn Fn(f32, bool) -> Message + 'a>,
    direction: f32,
}

#[derive(Default)]
struct State {
    drag: Option<Drag>,
}

struct Drag {
    origin: f32,
    width: f32,
    last: f32,
}

impl<Message> Resize<'_, Message> {
    fn hovered(&self, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle) -> bool {
        self.width.is_some()
            && cursor.position().is_some_and(|point| {
                viewport.contains(point)
                    && layout.bounds().contains(point)
                    && (0. ..=theme::FILE_PANEL_RESIZE_GRAB)
                        .contains(&(point.x - layout.bounds().x - self.offset))
            })
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Resize<'_, Message> {
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
        if self.width.is_none() {
            tree.state.downcast_mut::<State>().drag = None;
        }
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
        let state = tree.state.downcast_mut::<State>();
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if self.hovered(layout, cursor, viewport) =>
            {
                if let (Some(width), Some(position)) = (self.width, cursor.position()) {
                    state.drag = Some(Drag {
                        origin: position.x,
                        width,
                        last: width,
                    });
                    shell.capture_event();
                    shell.request_redraw();
                    return;
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some(drag) = &mut state.drag {
                    drag.last = (drag.width + (position.x - drag.origin) * self.direction)
                        .clamp(*self.limits.start(), *self.limits.end());
                    shell.publish((self.on_resize)(drag.last, false));
                    shell.capture_event();
                    return;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Window(window::Event::Unfocused)
            | Event::Mouse(mouse::Event::CursorLeft) => {
                if let Some(drag) = state.drag.take() {
                    if drag.last != drag.width {
                        shell.publish((self.on_resize)(drag.last, true));
                    }
                    shell.capture_event();
                    shell.request_redraw();
                    return;
                }
            }
            _ => {}
        }
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
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if tree.state.downcast_ref::<State>().drag.is_some()
            || self.hovered(layout, cursor, viewport)
        {
            mouse::Interaction::ResizingHorizontally
        } else {
            self.content.as_widget().mouse_interaction(
                &tree.children[0],
                layout,
                cursor,
                viewport,
                renderer,
            )
        }
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

#[cfg(test)]
mod tests;
