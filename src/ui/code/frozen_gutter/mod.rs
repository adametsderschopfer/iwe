//! Pin a source row's gutter while its containing scrollable moves horizontally.

use iced::{
    Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse,
        renderer::{self, Renderer as _},
        widget::{Operation, Tree},
    },
};

pub(crate) fn view<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    width: f32,
) -> Element<'a, Message> {
    Element::new(Frozen {
        content: content.into(),
        width,
    })
}

struct Frozen<'a, Message> {
    content: Element<'a, Message>,
    width: f32,
}

impl<Message> Widget<Message, Theme, Renderer> for Frozen<'_, Message> {
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
        let bounds = layout.bounds();
        let Some(visible) = bounds.intersection(viewport) else {
            return;
        };
        let gutter = Rectangle {
            width: self.width.min(visible.width),
            ..visible
        };
        if bounds.x >= viewport.x {
            if let Some(background) = crate::ui::theme::code_surface(theme).background {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: gutter,
                        ..Default::default()
                    },
                    background,
                );
            }
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout,
                cursor,
                viewport,
            );
            return;
        }
        let source = source_viewport(viewport, bounds, self.width);
        if let Some(clip) = source.intersection(&visible) {
            renderer.with_layer(clip, |renderer| {
                self.content.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    layout,
                    cursor,
                    &source,
                )
            });
        }
        renderer.with_layer(gutter, |renderer| {
            // Cover source text before applying the row's own background and gutter colors.
            if let Some(background) = crate::ui::theme::code_surface(theme).background {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: gutter,
                        ..Default::default()
                    },
                    background,
                );
            }
            let offset = (viewport.x - bounds.x).max(0.);
            renderer.with_translation(Vector::new(offset, 0.), |renderer| {
                self.content.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    layout,
                    mouse::Cursor::Unavailable,
                    &Rectangle {
                        x: bounds.x,
                        width: gutter.width,
                        ..gutter
                    },
                );
            });
        });
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
        let viewport = source_viewport(viewport, layout.bounds(), self.width);
        let cursor = if cursor.is_over(viewport) {
            cursor
        } else {
            mouse::Cursor::Unavailable
        };
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            &viewport,
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
        let viewport = source_viewport(viewport, layout.bounds(), self.width);
        if !cursor.is_over(viewport) {
            return mouse::Interaction::default();
        }
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            &viewport,
            renderer,
        )
    }
}

fn source_viewport(viewport: &Rectangle, bounds: Rectangle, gutter: f32) -> Rectangle {
    let left = viewport.x.max(bounds.x) + gutter.min(viewport.width);
    Rectangle {
        x: left,
        width: (viewport.x + viewport.width - left).max(0.),
        ..*viewport
    }
}

#[cfg(test)]
mod tests;
