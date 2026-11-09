//! Reorder column rows while preserving clicks and trailing row actions.

use super::theme;
use iced::{
    Border, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay,
        renderer::{self, Renderer as _},
        widget::{Operation, Tree, tree},
    },
    widget::Column,
    window,
};
use std::time::{Duration, Instant};

pub(crate) fn column<'a, Message: 'a>(
    content: Column<'a, Message>,
    action_width: f32,
    on_move: impl Fn(usize, usize) -> Message + 'a,
    on_scroll: impl Fn(f32) -> Message + 'a,
) -> Element<'a, Message> {
    Element::new(Reorder {
        content: content.into(),
        action_width,
        on_move: Box::new(on_move),
        on_scroll: Some(Box::new(on_scroll)),
    })
}

pub(crate) fn fixed_column<'a, Message: 'a>(
    content: Column<'a, Message>,
    on_move: impl Fn(usize, usize) -> Message + 'a,
) -> Element<'a, Message> {
    Element::new(Reorder {
        content: content.into(),
        action_width: 0.,
        on_move: Box::new(on_move),
        on_scroll: None,
    })
}

struct Reorder<'a, Message> {
    content: Element<'a, Message>,
    action_width: f32,
    on_move: Box<dyn Fn(usize, usize) -> Message + 'a>,
    on_scroll: Option<Box<dyn Fn(f32) -> Message + 'a>>,
}

#[derive(Default)]
struct State {
    drag: Option<Drag>,
}

struct Drag {
    source: usize,
    origin: Point,
    active: bool,
    last_scroll: Instant,
}

fn target(
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    viewport: &Rectangle,
    source: usize,
) -> Option<(usize, f32)> {
    let visible = layout.bounds().intersection(viewport)?;
    let position = cursor.position().filter(|point| visible.contains(*point))?;
    let mut slot = 0;
    let mut edge = layout.bounds().y;
    let mut count = 0;
    for row in layout.children() {
        count += 1;
        if position.y >= row.bounds().center_y() {
            slot += 1;
            edge = row.bounds().y + row.bounds().height;
        } else if slot == count - 1 {
            edge = row.bounds().y;
        }
    }
    if source >= count {
        return None;
    }
    Some((slot.saturating_sub(usize::from(slot > source)), edge))
}

impl<Message> Widget<Message, Theme, Renderer> for Reorder<'_, Message> {
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
        let state = tree.state.downcast_mut::<State>();
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(position) = cursor.position()
                    && viewport.contains(position)
                    && let Some(source) = layout.children().position(|row| {
                        row.bounds().contains(position)
                            && position.x < row.bounds().x + row.bounds().width - self.action_width
                    })
                {
                    state.drag = Some(Drag {
                        source,
                        origin: position,
                        active: false,
                        last_scroll: Instant::now(),
                    });
                    shell.capture_event();
                    return;
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some(drag) = &mut state.drag {
                    drag.active |= drag.origin.distance(*position) > 6.;
                    if drag.active {
                        shell.request_redraw();
                    }
                    shell.capture_event();
                    return;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some(drag) = state.drag.take() {
                    if drag.active {
                        if let Some((destination, _)) =
                            target(layout, cursor, viewport, drag.source)
                            && destination != drag.source
                        {
                            shell.publish((self.on_move)(drag.source, destination));
                        }
                        shell.request_redraw();
                        shell.capture_event();
                        return;
                    }
                    if layout.children().nth(drag.source).is_some_and(|row| {
                        cursor.is_over(row.bounds())
                            && cursor
                                .position()
                                .is_some_and(|point| viewport.contains(point))
                    }) {
                        let mut messages = Vec::new();
                        let mut press = Shell::new(&mut messages);
                        self.content.as_widget_mut().update(
                            &mut tree.children[0],
                            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                            layout,
                            cursor,
                            renderer,
                            clipboard,
                            &mut press,
                            viewport,
                        );
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
                        shell.merge(press, std::convert::identity);
                        return;
                    }
                }
            }
            Event::Window(window::Event::Unfocused) | Event::Mouse(mouse::Event::CursorLeft) => {
                state.drag = None;
                shell.request_redraw();
            }
            Event::Window(window::Event::RedrawRequested(now)) => {
                if let Some(drag) = &mut state.drag
                    && drag.active
                    && let Some(visible) = layout.bounds().intersection(viewport)
                    && let Some(position) =
                        cursor.position().filter(|point| visible.contains(*point))
                {
                    let bounds = layout.bounds();
                    let speed = if position.y < visible.y + 28. && bounds.y < visible.y {
                        -420.
                    } else if position.y > visible.y + visible.height - 28.
                        && bounds.y + bounds.height > visible.y + visible.height
                    {
                        420.
                    } else {
                        0.
                    };
                    if speed != 0.
                        && let Some(on_scroll) = &self.on_scroll
                    {
                        let elapsed = now
                            .saturating_duration_since(drag.last_scroll)
                            .as_secs_f32()
                            .min(0.05);
                        shell.publish(on_scroll(speed * elapsed));
                        shell.request_redraw_at(*now + Duration::from_millis(16));
                    }
                    drag.last_scroll = *now;
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
        if tree
            .state
            .downcast_ref::<State>()
            .drag
            .as_ref()
            .is_some_and(|drag| drag.active)
        {
            mouse::Interaction::Grabbing
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
        let drag = tree
            .state
            .downcast_ref::<State>()
            .drag
            .as_ref()
            .filter(|drag| drag.active);
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            if drag.is_some() {
                mouse::Cursor::Unavailable
            } else {
                cursor
            },
            viewport,
        );
        if let Some(drag) = drag
            && let Some(clip) = layout.bounds().intersection(viewport)
            && let Some(source) = layout.children().nth(drag.source)
        {
            renderer.with_layer(clip, |renderer| {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: source.bounds(),
                        border: Border {
                            color: theme::BLUE.resolve(theme),
                            width: 1.,
                            radius: 6.into(),
                        },
                        ..Default::default()
                    },
                    theme::BLUE.resolve(theme).scale_alpha(0.12),
                );
                if let Some((destination, edge)) = target(layout, cursor, viewport, drag.source)
                    && destination != drag.source
                {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle {
                                x: clip.x,
                                y: (edge - 1.)
                                    .clamp(clip.y, (clip.y + clip.height - 2.).max(clip.y)),
                                width: clip.width,
                                height: clip.height.min(2.),
                            },
                            ..Default::default()
                        },
                        theme::BLUE.resolve(theme),
                    );
                }
            });
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        if tree
            .state
            .downcast_ref::<State>()
            .drag
            .as_ref()
            .is_some_and(|drag| drag.active)
        {
            return None;
        }
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
