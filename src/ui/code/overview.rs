//! Draw cached change marks beside the native scrollbar handle without capturing input.

use super::{
    Document,
    comparison::{Hunk, Mark, Side},
    folding::Folding,
};
use crate::ui::theme;
use iced::{
    Alignment, Element, Length, Point, Rectangle, Renderer, Size, Theme, mouse,
    widget::{
        canvas::{self, Cache, Frame, Geometry},
        container, scrollable, stack,
    },
};
use std::{
    cell::Cell,
    hash::{DefaultHasher, Hash, Hasher},
    ops::Range,
};

pub(crate) struct Layout {
    pub rows: usize,
    pub row_height: f32,
    pub viewport: Size,
    pub content_width: f32,
}

pub(crate) fn scrollbar() -> scrollable::Scrollbar {
    scrollable::Scrollbar::default()
        .width(theme::CODE_SCROLLBAR_WIDTH)
        .scroller_width(theme::CODE_SCROLLBAR_THUMB_WIDTH)
}

pub(crate) fn view<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    document: &'a Document,
    folding: &'a Folding,
    side: Option<Side>,
    layout: Layout,
) -> Element<'a, Message> {
    let Some(comparison) = &document.comparison else {
        return content.into();
    };
    let marks = match side {
        Some(side) => Marks::Side(folding.side_hunks(comparison), side),
        None => Marks::Unified(folding.unified_marks(comparison)),
    };
    let mut hasher = DefaultHasher::new();
    (
        document.fingerprint,
        folding.revision,
        side,
        layout.rows,
        layout.row_height.to_bits(),
        layout.viewport.height.to_bits(),
    )
        .hash(&mut hasher);
    // Match Iced's default horizontal scrollbar and the source view's bottom inset.
    let bottom = if layout.content_width > layout.viewport.width {
        10.
    } else {
        0.
    };
    let rail = (layout.viewport.height - bottom).max(0.);
    let overview = canvas::Canvas::new(Overview {
        key: hasher.finish(),
        marks,
        content_height: (layout.rows as f32 * layout.row_height + 12.).max(layout.viewport.height),
        row_height: layout.row_height,
    })
    .width(theme::CODE_CHANGE_MARK_WIDTH)
    .height(rail);
    let id = match side {
        Some(Side::Before) => "source-overview-before",
        Some(Side::After) => "source-overview-after",
        None => "source-overview-unified",
    };
    stack![
        content.into(),
        container(container(overview).id(id))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::End)
            .align_y(Alignment::Start)
    ]
    .into()
}

enum Marks<'a> {
    Unified(&'a [Mark]),
    Side(&'a [Hunk], Side),
}

impl Marks<'_> {
    fn iter(&self) -> impl Iterator<Item = Mark> + '_ {
        let (unified, hunks, side): (&[Mark], &[Hunk], Side) = match self {
            Self::Unified(marks) => (marks, &[], Side::Before),
            Self::Side(hunks, side) => (&[], hunks, *side),
        };
        unified
            .iter()
            .cloned()
            .chain(hunks.iter().map(move |hunk| Mark {
                rows: hunk.start..hunk.start + hunk.lengths[side.index()],
                change: hunk.change,
            }))
    }
}

struct Overview<'a> {
    key: u64,
    marks: Marks<'a>,
    content_height: f32,
    row_height: f32,
}

#[derive(Default)]
struct State {
    key: Cell<Option<(u64, bool)>>,
    cache: Cache,
}

impl<Message> canvas::Program<Message> for Overview<'_> {
    type State = State;

    fn draw(
        &self,
        state: &State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let key = (self.key, theme.extended_palette().is_dark);
        if state.key.get() != Some(key) {
            state.cache.clear();
            state.key.set(Some(key));
        }
        vec![
            state
                .cache
                .draw(renderer, bounds.size(), |frame: &mut Frame| {
                    for mark in self.marks.iter() {
                        if let Some(bounds) = mark_bounds(
                            &mark.rows,
                            self.row_height,
                            self.content_height,
                            frame.size(),
                        ) {
                            frame.fill_rectangle(
                                bounds.position(),
                                bounds.size(),
                                theme::line_change(mark.change).resolve(theme),
                            );
                        }
                    }
                }),
        ]
    }
}

fn mark_bounds(
    rows: &Range<usize>,
    row_height: f32,
    content_height: f32,
    rail: Size,
) -> Option<Rectangle> {
    if rail.width <= 0. || rail.height <= 0. || content_height <= 0. {
        return None;
    }
    let scale = rail.height / content_height;
    let height = ((rows.end.saturating_sub(rows.start)) as f32 * row_height * scale)
        .max(theme::CODE_CHANGE_MARK_MIN_HEIGHT)
        .min(rail.height);
    let y = (rows.start as f32 * row_height * scale).min(rail.height - height);
    Some(Rectangle::new(
        Point::new(0., y.max(0.)),
        Size::new(rail.width, height),
    ))
}

#[cfg(test)]
mod tests;
