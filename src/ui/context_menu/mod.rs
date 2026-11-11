//! Anchor context menus to right-click positions without tracking every pointer movement.

use iced::{
    Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree},
    },
    keyboard,
    widget::{container, mouse_area, opaque, responsive, stack},
};

pub(crate) fn trigger<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    on_open: impl Fn(Point) -> Message + 'a,
) -> Element<'a, Message> {
    Element::new(Trigger {
        content: content.into(),
        on_open: Some(Box::new(on_open)),
        on_key: None,
        anchored: false,
    })
}

pub(crate) fn anchored<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    on_open: impl Fn(Point) -> Message + 'a,
) -> Element<'a, Message> {
    Element::new(Trigger {
        content: content.into(),
        on_open: Some(Box::new(on_open)),
        on_key: None,
        anchored: true,
    })
}

type KeyHandler<'a, Message> = Box<dyn Fn(&keyboard::Event) -> Option<Message> + 'a>;

/// Handle popup keys before focused content can consume them.
pub(crate) fn on_key<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    handler: impl Fn(&keyboard::Event) -> Option<Message> + 'a,
) -> Element<'a, Message> {
    Element::new(Trigger {
        content: content.into(),
        on_open: None,
        on_key: Some(Box::new(handler)),
        anchored: false,
    })
}

pub(crate) fn dismiss_on_escape<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    dismiss: Message,
) -> Element<'a, Message> {
    on_key(content, move |event| {
        matches!(
            event,
            keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }
        )
        .then(|| dismiss.clone())
    })
}

struct Trigger<'a, Message> {
    content: Element<'a, Message>,
    on_open: Option<Box<dyn Fn(Point) -> Message + 'a>>,
    on_key: Option<KeyHandler<'a, Message>>,
    anchored: bool,
}

impl<Message> Widget<Message, Theme, Renderer> for Trigger<'_, Message> {
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
        if let Event::Keyboard(event) = event
            && let Some(message) = self.on_key.as_ref().and_then(|handler| handler(event))
        {
            shell.publish(message);
            shell.capture_event();
            return;
        }
        let button = if self.anchored {
            mouse::Button::Left
        } else {
            mouse::Button::Right
        };
        if matches!(event, Event::Mouse(mouse::Event::ButtonPressed(pressed)) if *pressed == button)
            && let Some(on_open) = &self.on_open
            && let Some(position) = cursor
                .position()
                .filter(|point| layout.bounds().contains(*point) && viewport.contains(*point))
        {
            let position = if self.anchored {
                let bounds = layout.bounds();
                Point::new(bounds.x, bounds.y + bounds.height + 6.)
            } else {
                position
            };
            shell.publish(on_open(position));
            shell.capture_event();
            return;
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
        if self.anchored && cursor.is_over(layout.bounds()) && cursor.is_over(*viewport) {
            return mouse::Interaction::Pointer;
        }
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

pub(crate) fn overlay<'a, Message: Clone + 'a>(
    base: Element<'a, Message>,
    position: Point,
    size: Size,
    content: impl Fn() -> Element<'a, Message> + 'a,
    dismiss: Message,
) -> Element<'a, Message> {
    let popup = responsive(move |available| {
        container(opaque(
            container(content())
                .padding(8)
                .width(size.width)
                .style(super::theme::tooltip),
        ))
        .padding(iced::Padding {
            left: position.x.clamp(0., (available.width - size.width).max(0.)),
            top: position
                .y
                .clamp(0., (available.height - size.height).max(0.)),
            ..iced::Padding::ZERO
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    });
    let content = stack![
        base,
        opaque(
            mouse_area(popup)
                .on_press(dismiss.clone())
                .on_right_press(dismiss.clone())
        )
    ];
    dismiss_on_escape(content, dismiss)
}

#[cfg(test)]
mod tests;
