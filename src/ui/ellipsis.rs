//! Fit a single line using the renderer's actual text measurements.

use iced::{
    Element, Length, Rectangle, Renderer, Size, Theme,
    advanced::{
        Layout, Widget, layout, mouse, renderer,
        text::{self, Paragraph as _, Renderer as _},
        widget::{Operation, Tree, tree},
    },
    alignment,
};

type Paragraph = <Renderer as text::Renderer>::Paragraph;

pub(crate) fn text<'a, Message: 'a>(content: &'a str, size: f32) -> Element<'a, Message> {
    Element::new(Ellipsis { content, size })
}

struct Ellipsis<'a> {
    content: &'a str,
    size: f32,
}

#[derive(Default)]
struct State {
    original: String,
    displayed: String,
    width: f32,
    size: f32,
    paragraph: Paragraph,
}

fn paragraph(content: &str, size: f32) -> Paragraph {
    Paragraph::with_text(text::Text {
        content,
        bounds: Size::new(f32::INFINITY, size * 1.3),
        size: size.into(),
        line_height: text::LineHeight::Relative(1.3),
        font: super::theme::font(),
        align_x: text::Alignment::Left,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: text::Wrapping::None,
    })
}

impl State {
    fn update(&mut self, content: &str, width: f32, size: f32) {
        if self.original == content && self.width == width && self.size == size {
            return;
        }
        self.original = content.to_owned();
        self.width = width;
        self.size = size;
        self.displayed = content.to_owned();
        self.paragraph = paragraph(content, size);
        if self.paragraph.min_width() > width {
            let boundaries: Vec<_> = content
                .char_indices()
                .map(|(index, _)| index)
                .chain([content.len()])
                .collect();
            let mut low = 0;
            let mut high = boundaries.len() - 1;
            while low < high {
                let middle = (low + high).div_ceil(2);
                let candidate = format!("{}…", content[..boundaries[middle]].trim_end());
                if paragraph(&candidate, size).min_width() <= width {
                    low = middle;
                } else {
                    high = middle - 1;
                }
            }
            self.displayed = format!("{}…", content[..boundaries[low]].trim_end());
            self.paragraph = paragraph(&self.displayed, size);
            if self.paragraph.min_width() > width {
                self.displayed.clear();
                self.paragraph = paragraph("", size);
            }
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Ellipsis<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }
    fn layout(&mut self, tree: &mut Tree, _: &Renderer, limits: &layout::Limits) -> layout::Node {
        let state = tree.state.downcast_mut::<State>();
        state.update(self.content, limits.max().width, self.size);
        layout::Node::new(limits.resolve(
            Length::Fill,
            Length::Shrink,
            Size::new(state.width, self.size * 1.3),
        ))
    }
    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if let Some(clip) = layout.bounds().intersection(viewport) {
            renderer.fill_paragraph(
                &tree.state.downcast_ref::<State>().paragraph,
                layout.position(),
                style.text_color,
                clip,
            );
        }
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.text(
            None,
            layout.bounds(),
            &tree.state.downcast_ref::<State>().displayed,
        );
    }
}

#[cfg(test)]
mod tests;
