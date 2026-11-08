//! Preserve text layout for source rows that remain in the visible range.

use iced::{
    Element, Event, Length, Rectangle, Renderer, Size, Theme,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, renderer,
        widget::{Operation, Tree, tree},
    },
    widget::Column,
};
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    ops::Range,
};

pub(crate) fn view<'a, Message: 'a>(
    key: impl Hash,
    range: Range<usize>,
    lines: impl IntoIterator<Item = Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    Element::new(Rows {
        key: hasher.finish(),
        range,
        content: Column::with_children(lines).into(),
    })
}

struct Rows<'a, Message> {
    key: u64,
    range: Range<usize>,
    content: Element<'a, Message>,
}

struct State {
    key: u64,
    range: Range<usize>,
    layout: Option<(Size, Size, layout::Node)>,
}

impl<Message> Widget<Message, Theme, Renderer> for Rows<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            key: self.key,
            range: self.range.clone(),
            layout: None,
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State>();
        if state.key == self.key && state.range == self.range {
            return;
        }
        state.key = self.key;
        state.layout = None;
        let previous = &mut state.range;
        let content = &mut tree.children[0];
        let overlap = self.range.start.max(previous.start)..self.range.end.min(previous.end);
        if overlap.is_empty() {
            content.children.clear();
        } else {
            content.children.drain(..overlap.start - previous.start);
            content.children.truncate(overlap.len());
            content.children.splice(
                ..0,
                (self.range.start..overlap.start).map(|_| Tree::empty()),
            );
        }
        content.children.resize_with(self.range.len(), Tree::empty);
        content.diff(self.content.as_widget());
        previous.clone_from(&self.range);
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
        let state = tree.state.downcast_mut::<State>();
        if let Some((min, max, node)) = &state.layout
            && *min == limits.min()
            && *max == limits.max()
        {
            return node.clone();
        }
        let node = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        state.layout = Some((limits.min(), limits.max(), node.clone()));
        node
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
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
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
}

#[cfg(test)]
mod tests;
