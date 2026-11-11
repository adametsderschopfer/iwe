//! Select displayed source text across virtual rows without including gutters.

use super::{Document, comparison::Side};
use crate::{
    app::{App, Message},
    ui::theme,
};
use iced::{
    Element, Event, Length, Point, Rectangle, Renderer, Size, Theme,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, renderer,
        text::{self, Paragraph},
        widget::{Operation, Tree, tree},
    },
    keyboard,
    widget::{rich_text, text as widget_text},
};
use std::{borrow::Cow, ops::Range};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lane {
    Unified,
    Before,
    After,
}

impl From<Side> for Lane {
    fn from(side: Side) -> Self {
        match side {
            Side::Before => Self::Before,
            Side::After => Self::After,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Position {
    pub row: usize,
    pub byte: usize,
}

#[derive(Debug, Clone)]
pub(crate) enum Action {
    Start(Lane, Position),
    Extend(Lane, Position),
    End,
    All(Lane),
    Line(Lane, usize, usize),
    Clear,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Selection {
    lane: Option<Lane>,
    anchor: Option<Position>,
    focus: Option<Position>,
    dragging: bool,
    highlighted_row: Option<usize>,
    fingerprint: Option<[u8; 32]>,
    pub revision: u64,
}

impl Selection {
    pub(crate) fn lane(&self) -> Option<Lane> {
        self.lane
    }
    pub(crate) fn highlights_line(&self, document: &Document, lane: Lane, row: usize) -> bool {
        self.fingerprint == Some(document.fingerprint)
            && self.lane == Some(lane)
            && self.highlighted_row == Some(row)
    }

    pub(crate) fn update(&mut self, action: Action, document: &Document) {
        if self.fingerprint != Some(document.fingerprint) {
            *self = Self {
                revision: self.revision.wrapping_add(1),
                ..Self::default()
            };
        }
        self.fingerprint = Some(document.fingerprint);
        if !matches!(action, Action::End) {
            self.highlighted_row = None;
        }
        match action {
            Action::Line(lane, row, length) => {
                self.lane = Some(lane);
                self.anchor = Some(Position { row, byte: 0 });
                self.focus = Some(Position { row, byte: length });
                self.dragging = false;
                self.highlighted_row = Some(row);
            }
            Action::Start(lane, position) => {
                self.lane = Some(lane);
                self.anchor = Some(position);
                self.focus = Some(position);
                self.dragging = true;
            }
            Action::Extend(lane, position) if self.dragging && self.lane == Some(lane) => {
                if self.focus == Some(position) {
                    return;
                }
                self.focus = Some(position);
            }
            Action::End => self.dragging = false,
            Action::All(lane) => {
                let rows = indices(document, lane);
                if let Some(index) = rows.last() {
                    self.lane = Some(lane);
                    self.anchor = Some(Position { row: 0, byte: 0 });
                    self.focus = Some(Position {
                        row: rows.len() - 1,
                        byte: line_text(document, *index).len(),
                    });
                    self.dragging = false;
                }
            }
            Action::Clear => {
                self.anchor = None;
                self.focus = None;
                self.dragging = false;
            }
            _ => return,
        }
        self.revision = self.revision.wrapping_add(1);
    }

    fn range(&self, lane: Lane, row: usize, length: usize) -> Option<Range<usize>> {
        if self.lane != Some(lane) {
            return None;
        }
        let (anchor, focus) = (self.anchor?, self.focus?);
        let (start, end) = (anchor.min(focus), anchor.max(focus));
        if row < start.row || row > end.row {
            return None;
        }
        Some(
            (if row == start.row {
                start.byte.min(length)
            } else {
                0
            })..(if row == end.row {
                end.byte.min(length)
            } else {
                length
            }),
        )
    }

    fn copy(&self, document: &Document) -> Option<String> {
        let (anchor, focus) = (self.anchor?, self.focus?);
        if anchor == focus {
            return None;
        }
        let lane = self.lane?;
        let mut result = Vec::new();
        for (row, index) in indices(document, lane).into_iter().enumerate() {
            let text = line_text(document, index);
            if let Some(range) = self.range(lane, row, text.len()) {
                let original = document.source_lines.get(index)?;
                let start = source_offset(original, range.start, false);
                let end = source_offset(original, range.end, true);
                result.push(original.get(start..end)?.to_owned());
            }
        }
        Some(result.join("\n"))
    }
}

fn source_offset(original: &str, visual: usize, end: bool) -> usize {
    let mut displayed = 0;
    for (offset, character) in original.char_indices() {
        if visual <= displayed {
            return offset;
        }
        let width = if character == '\t' {
            4
        } else {
            character.len_utf8()
        };
        if visual < displayed + width {
            return if end {
                offset + character.len_utf8()
            } else {
                offset
            };
        }
        displayed += width;
    }
    original.len()
}

fn indices(document: &Document, lane: Lane) -> Vec<usize> {
    match lane {
        Lane::Unified => (0..document.lines.len()).collect(),
        Lane::Before | Lane::After => document
            .comparison
            .as_ref()
            .map(|comparison| {
                let side = usize::from(lane == Lane::After);
                comparison
                    .rows
                    .iter()
                    .filter_map(|row| row.cells[side].as_ref().map(|cell| cell.index))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn line_text(document: &Document, index: usize) -> Cow<'_, str> {
    Cow::Borrowed(document.display_line(index))
}

pub(crate) fn line<'a>(
    app: &App,
    document: &'a Document,
    index: usize,
    lane: Lane,
    row: usize,
    spans: Vec<iced::widget::text::Span<'a>>,
    size: f32,
) -> Element<'a, Message> {
    let mut selection = app.preview().selection.clone();
    if selection.fingerprint != Some(document.fingerprint) {
        selection = Selection::default();
    }
    let value = line_text(document, index);
    let range = selection.range(lane, row, value.len());
    let highlighted = highlight_spans(spans, range, theme::CODE_SELECTION.resolve(&app.theme()));
    let content = rich_text::<(), Message, _, _>(highlighted)
        .font(theme::code_font())
        .size(size)
        .line_height(1.65)
        .wrapping(widget_text::Wrapping::None)
        .into();
    Element::new(Selectable {
        content,
        value,
        document,
        selection,
        lane,
        row,
        id: app.preview().id,
        size,
        shortcuts: app.workspace.preferences.shortcuts.clone(),
    })
}

fn highlight_spans(
    spans: Vec<iced::widget::text::Span<'_>>,
    range: Option<Range<usize>>,
    background: iced::Color,
) -> Vec<iced::widget::text::Span<'_>> {
    let Some(range) = range.filter(|range| !range.is_empty()) else {
        return spans;
    };
    let mut highlighted = Vec::with_capacity(spans.len() + 2);
    let mut offset = 0;
    for span in spans {
        let length = span.text.len();
        if range.end <= offset || range.start >= offset + length {
            highlighted.push(span);
            offset += length;
            continue;
        }
        if range.start <= offset && range.end >= offset + length {
            highlighted.push(span.background(background));
            offset += length;
            continue;
        }
        let mut cuts = vec![0, length];
        for boundary in [range.start, range.end] {
            if boundary > offset && boundary < offset + length {
                cuts.push(boundary - offset);
            }
        }
        cuts.sort_unstable();
        cuts.dedup();
        for pair in cuts.windows(2) {
            let mut fragment = span.clone();
            fragment.text = span.text[pair[0]..pair[1]].to_owned().into();
            if range.contains(&(offset + pair[0])) {
                fragment = fragment.background(background);
            }
            highlighted.push(fragment);
        }
        offset += length;
    }
    highlighted
}

type TextParagraph = <Renderer as text::Renderer>::Paragraph;

struct Selectable<'a> {
    content: Element<'a, Message>,
    value: Cow<'a, str>,
    document: &'a Document,
    selection: Selection,
    lane: Lane,
    row: usize,
    id: u64,
    size: f32,
    shortcuts: crate::domain::shortcuts::Shortcuts,
}

impl Widget<Message, Theme, Renderer> for Selectable<'_> {
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout.children().next().unwrap(),
            renderer,
            operation,
        );
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<text::paragraph::Plain<TextParagraph>>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(text::paragraph::Plain::<TextParagraph>::default())
    }
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }
    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let child = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        let size = Size::new(limits.max().width, child.size().height);
        layout::Node::with_children(size, vec![child])
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
            layout.children().next().unwrap(),
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
        _renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if shell.is_event_captured() {
            return;
        }
        let needs_position = match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => true,
            Event::Mouse(mouse::Event::CursorMoved { .. }) => self.selection.dragging,
            _ => false,
        };
        let over = cursor.position().filter(|point| {
            needs_position && layout.bounds().contains(*point) && viewport.contains(*point)
        });
        let position = over.map(|point| {
            let local = Point::new((point.x - layout.bounds().x).max(0.), self.size * 0.8);
            let paragraph = tree
                .state
                .downcast_mut::<text::paragraph::Plain<TextParagraph>>();
            // Shape the pointer lookup only when selection needs a character position.
            paragraph.update(text::Text {
                content: &self.value,
                bounds: layout.bounds().size(),
                size: self.size.into(),
                line_height: text::LineHeight::Relative(1.65),
                font: theme::code_font(),
                align_x: text::Alignment::Default,
                align_y: iced::alignment::Vertical::Top,
                shaping: text::Shaping::Advanced,
                wrapping: text::Wrapping::None,
            });
            let mut byte = paragraph
                .raw()
                .hit_test(local)
                .map_or(self.value.len(), |hit| hit.cursor())
                .min(self.value.len());
            while !self.value.is_char_boundary(byte) {
                byte = byte.saturating_sub(1);
            }
            Position {
                row: self.row,
                byte,
            }
        });
        let action = match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                position.map(|position| Action::Start(self.lane, position))
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) if self.selection.dragging => {
                position.map(|position| Action::Extend(self.lane, position))
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if self.selection.dragging =>
            {
                Some(Action::End)
            }
            Event::Window(iced::window::Event::Unfocused) if self.selection.dragging => {
                Some(Action::End)
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                physical_key,
                modifiers,
                ..
            }) if self.selection.anchor.is_some() && self.selection.lane == Some(self.lane) => {
                use crate::domain::shortcuts::Command;
                let matches = |command| {
                    crate::ui::shortcuts::matches(
                        &self.shortcuts.binding(command),
                        key,
                        *physical_key,
                        *modifiers,
                    )
                };
                if matches(Command::Copy) {
                    if let Some(text) = self.selection.copy(self.document) {
                        clipboard.write(iced::advanced::clipboard::Kind::Standard, text);
                        shell.capture_event();
                    }
                    None
                } else if matches(Command::SelectAll) {
                    Some(Action::All(self.lane))
                } else if *key == keyboard::Key::Named(keyboard::key::Named::Escape) {
                    Some(Action::Clear)
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(action) = action {
            shell.publish(Message::CodeSelection(self.id, action));
            shell.capture_event();
        }
    }
    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) && cursor.is_over(*viewport) {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::None
        }
    }
}

#[cfg(test)]
mod tests;
