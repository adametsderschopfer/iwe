//! Select displayed source text across virtual rows without including gutters.

use super::{Document, comparison::Side};
use crate::{app::{App, Message}, ui::theme};
use iced::{
    Element, Event, Font, Length, Point, Rectangle, Renderer, Size, Theme, keyboard,
    advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, renderer, text::{self, Paragraph},
        widget::{Tree, tree}},
    widget::{rich_text, text as widget_text},
};
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lane { Unified, Before, After }

impl From<Side> for Lane {
    fn from(side: Side) -> Self {
        match side { Side::Before => Self::Before, Side::After => Self::After }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Position { pub row: usize, pub byte: usize }

#[derive(Debug, Clone)]
pub(crate) enum Action {
    Start(Lane, Position), Extend(Lane, Position), End, All(Lane), Clear,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Selection {
    lane: Option<Lane>, anchor: Option<Position>, focus: Option<Position>, dragging: bool,
    fingerprint: Option<[u8; 32]>, pub revision: u64,
}

impl Selection {
    pub(crate) fn lane(&self) -> Option<Lane> { self.lane }
    pub(crate) fn update(&mut self, action: Action, document: &Document) {
        if self.fingerprint != Some(document.fingerprint) {
            *self = Self { revision: self.revision.wrapping_add(1), ..Self::default() };
        }
        self.fingerprint = Some(document.fingerprint);
        match action {
            Action::Start(lane, position) => {
                self.lane = Some(lane); self.anchor = Some(position); self.focus = Some(position);
                self.dragging = true;
            }
            Action::Extend(lane, position) if self.dragging && self.lane == Some(lane) => {
                if self.focus == Some(position) { return; }
                self.focus = Some(position);
            }
            Action::End => self.dragging = false,
            Action::All(lane) => {
                let rows = indices(document, lane);
                if let Some(index) = rows.last() {
                    self.lane = Some(lane);
                    self.anchor = Some(Position { row: 0, byte: 0 });
                    self.focus = Some(Position { row: rows.len() - 1, byte: line_text(document, *index).len() });
                    self.dragging = false;
                }
            }
            Action::Clear => { self.anchor = None; self.focus = None; self.dragging = false; }
            _ => return,
        }
        self.revision = self.revision.wrapping_add(1);
    }

    fn range(&self, lane: Lane, row: usize, length: usize) -> Option<Range<usize>> {
        if self.lane != Some(lane) { return None; }
        let (anchor, focus) = (self.anchor?, self.focus?);
        let (start, end) = (anchor.min(focus), anchor.max(focus));
        if row < start.row || row > end.row { return None; }
        Some((if row == start.row { start.byte.min(length) } else { 0 })
            ..(if row == end.row { end.byte.min(length) } else { length }))
    }

    fn copy(&self, document: &Document) -> Option<String> {
        let (anchor, focus) = (self.anchor?, self.focus?);
        if anchor == focus { return None; }
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
        if visual <= displayed { return offset; }
        let width = if character == '\t' { 4 } else { character.len_utf8() };
        if visual < displayed + width {
            return if end { offset + character.len_utf8() } else { offset };
        }
        displayed += width;
    }
    original.len()
}

fn indices(document: &Document, lane: Lane) -> Vec<usize> {
    match lane {
        Lane::Unified => (0..document.lines.len()).collect(),
        Lane::Before | Lane::After => document.comparison.as_ref().map(|comparison| {
            let side = usize::from(lane == Lane::After);
            comparison.rows.iter().filter_map(|row| row.cells[side].as_ref().map(|cell| cell.index)).collect()
        }).unwrap_or_default(),
    }
}

fn line_text(document: &Document, index: usize) -> String {
    document.lines[index].iter().map(|(text, _)| text.as_str()).collect()
}

pub(crate) fn line<'a>(
    app: &App, document: &'a Document, index: usize, lane: Lane, row: usize,
    spans: Vec<iced::widget::text::Span<'a>>, size: f32,
) -> Element<'a, Message> {
    let mut selection = app.preview().selection.clone();
    if selection.fingerprint != Some(document.fingerprint) { selection = Selection::default(); }
    let value = line_text(document, index);
    let range = selection.range(lane, row, value.len());
    let mut highlighted = Vec::new();
    let mut offset = 0;
    for span in spans {
        let text = span.text.to_string();
        let length = text.len();
        let mut cuts = vec![0, length];
        if let Some(range) = &range {
            for boundary in [range.start, range.end] {
                if boundary > offset && boundary < offset + length { cuts.push(boundary - offset); }
            }
        }
        cuts.sort_unstable(); cuts.dedup();
        for pair in cuts.windows(2) {
            let mut fragment = span.clone();
            fragment.text = text[pair[0]..pair[1]].to_owned().into();
            if range.as_ref().is_some_and(|range| range.contains(&(offset + pair[0]))) {
                fragment = fragment.background(theme::CODE_SELECTION.resolve(&app.theme()));
            }
            highlighted.push(fragment);
        }
        offset += length;
    }
    let content = rich_text::<(), Message, _, _>(highlighted).font(Font::MONOSPACE)
        .size(size).line_height(1.65).wrapping(widget_text::Wrapping::None).into();
    Element::new(Selectable { content, value, document, selection, lane, row, id: app.preview().id, size })
}

type TextParagraph = <Renderer as text::Renderer>::Paragraph;

struct Selectable<'a> {
    content: Element<'a, Message>, value: String, document: &'a Document,
    selection: Selection, lane: Lane, row: usize, id: u64, size: f32,
}

impl Widget<Message, Theme, Renderer> for Selectable<'_> {
    fn tag(&self) -> tree::Tag { tree::Tag::of::<text::paragraph::Plain<TextParagraph>>() }
    fn state(&self) -> tree::State { tree::State::new(text::paragraph::Plain::<TextParagraph>::default()) }
    fn children(&self) -> Vec<Tree> { vec![Tree::new(&self.content)] }
    fn diff(&self, tree: &mut Tree) { tree.diff_children(std::slice::from_ref(&self.content)); }
    fn size(&self) -> Size<Length> { Size::new(Length::Fill, Length::Shrink) }
    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        let child = self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits);
        let size = Size::new(limits.max().width, child.size().height);
        tree.state.downcast_mut::<text::paragraph::Plain<TextParagraph>>().update(text::Text {
            content: &self.value, bounds: size, size: self.size.into(),
            line_height: text::LineHeight::Relative(1.65), font: Font::MONOSPACE,
            align_x: text::Alignment::Default, align_y: iced::alignment::Vertical::Top,
            shaping: text::Shaping::Advanced, wrapping: text::Wrapping::None,
        });
        layout::Node::with_children(size, vec![child])
    }
    fn draw(&self, tree: &Tree, renderer: &mut Renderer, theme: &Theme,
        style: &renderer::Style, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle) {
        self.content.as_widget().draw(&tree.children[0], renderer, theme, style,
            layout.children().next().unwrap(), cursor, viewport);
    }
    fn update(&mut self, tree: &mut Tree, event: &Event, layout: Layout<'_>, cursor: mouse::Cursor,
        _renderer: &Renderer, clipboard: &mut dyn Clipboard, shell: &mut Shell<'_, Message>, viewport: &Rectangle) {
        if shell.is_event_captured() { return; }
        let over = cursor.position().filter(|point| layout.bounds().contains(*point) && viewport.contains(*point));
        let position = over.map(|point| {
            let local = Point::new((point.x - layout.bounds().x).max(0.), self.size * 0.8);
            let paragraph = tree.state.downcast_ref::<text::paragraph::Plain<TextParagraph>>();
            let mut byte = paragraph.raw().hit_test(local).map_or(self.value.len(), |hit| hit.cursor()).min(self.value.len());
            while !self.value.is_char_boundary(byte) { byte = byte.saturating_sub(1); }
            Position { row: self.row, byte }
        });
        let action = match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => position.map(|position| Action::Start(self.lane, position)),
            Event::Mouse(mouse::Event::CursorMoved { .. }) if self.selection.dragging => position.map(|position| Action::Extend(self.lane, position)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if self.selection.dragging => Some(Action::End),
            Event::Window(iced::window::Event::Unfocused) if self.selection.dragging => Some(Action::End),
            Event::Keyboard(keyboard::Event::KeyPressed { key, physical_key, modifiers, .. })
                if self.selection.anchor.is_some() && self.selection.lane == Some(self.lane) => {
                let command = *modifiers == keyboard::Modifiers::COMMAND;
                let character = |value: &str| matches!(key, keyboard::Key::Character(text) if text.eq_ignore_ascii_case(value));
                if command && (character("c") || *physical_key == keyboard::key::Physical::Code(keyboard::key::Code::KeyC)) {
                    if let Some(text) = self.selection.copy(self.document) {
                        clipboard.write(iced::advanced::clipboard::Kind::Standard, text);
                        shell.capture_event();
                    }
                    None
                } else if command && (character("a") || *physical_key == keyboard::key::Physical::Code(keyboard::key::Code::KeyA)) {
                    Some(Action::All(self.lane))
                } else if *key == keyboard::Key::Named(keyboard::key::Named::Escape) { Some(Action::Clear) }
                else { None }
            }
            _ => None,
        };
        if let Some(action) = action {
            shell.publish(Message::CodeSelection(self.id, action));
            shell.capture_event();
        }
    }
    fn mouse_interaction(&self, _tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor,
        viewport: &Rectangle, _renderer: &Renderer) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) && cursor.is_over(*viewport) { mouse::Interaction::Text }
        else { mouse::Interaction::None }
    }
}
