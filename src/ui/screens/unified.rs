//! Compose a unified comparison with both source line numbers and compact fold separators.

use crate::ui::tooltip;
use crate::{
    app::{App, Message},
    i18n::{FluentArgs, Key},
    ui::{
        code::{self, Document, comparison::UnifiedLine},
        theme,
    },
};
use iced::{
    Alignment, Color, Element, Length, Point, Rectangle, Renderer, Theme, mouse,
    widget::{
        Row, button,
        canvas::{self, Frame, Geometry, Path, Stroke},
        container, row, space, text,
    },
};
use std::ops::Range;

fn number_width(document: &Document, size: f32) -> f32 {
    let digits = document.lines.len().max(1).ilog10() + 1;
    (digits as f32 * size * 0.65 + 8.).max(theme::UNIFIED_NUMBER_MIN_WIDTH)
}

pub(super) fn gutter_width(document: &Document, size: f32, numbers: bool) -> f32 {
    if numbers {
        3. + number_width(document, size) * 2.
            + theme::UNIFIED_NUMBER_GAP
            + theme::UNIFIED_GUTTER_PADDING * 2.
    } else {
        3.
    }
}

pub(super) fn line<'a>(
    app: &App,
    document: &'a Document,
    line: UnifiedLine<'_>,
    index: usize,
    height: f32,
) -> Element<'a, Message> {
    let size = app.workspace.preferences.code_size as f32;
    let highlighted =
        app.preview()
            .selection
            .highlights_line(document, code::selection::Lane::Unified, index);
    let color = line
        .change
        .map(|change| theme::unified_color(change).resolve(&app.theme()))
        .unwrap_or(Color::TRANSPARENT);
    let marker = container(space())
        .width(3)
        .height(height)
        .style(move |_| container::Style {
            background: Some(color.into()),
            ..Default::default()
        });
    let mut gutter = Row::new().push(marker).height(height);
    if app.workspace.preferences.line_numbers {
        let width = number_width(document, size);
        let numbers = [0, 1].map(|side| {
            let number = text(
                line.numbers[side]
                    .map(|number| number.to_string())
                    .unwrap_or_default(),
            )
            .font(theme::code_font())
            .size(size)
            .line_height(1.65)
            .style(
                line.change
                    .map(theme::unified_color)
                    .unwrap_or(theme::FAINT)
                    .text(),
            )
            .width(width)
            .align_x(Alignment::End);
            container(number).id(format!("unified-number-{side}-{index}"))
        });
        gutter = gutter.push(
            container(
                Row::with_children(numbers.into_iter().map(Element::from))
                    .spacing(theme::UNIFIED_NUMBER_GAP),
            )
            .padding([0., theme::UNIFIED_GUTTER_PADDING]),
        );
    }
    let gutter = container(gutter)
        .width(gutter_width(
            document,
            size,
            app.workspace.preferences.line_numbers,
        ))
        .height(height)
        .style(move |theme| {
            theme::source_row(
                theme,
                theme::unified_gutter(line.change)(theme),
                highlighted,
            )
        });
    let source = code::selection::line(
        app,
        document,
        index,
        code::selection::Lane::Unified,
        index,
        code::emphasized_spans(
            &document.lines[index],
            line.emphasis,
            line.change
                .map(|change| theme::change_fill(theme::unified_color(change), &app.theme(), 0.30))
                .unwrap_or(Color::TRANSPARENT),
            &app.theme(),
        ),
        size,
    );
    code::frozen_gutter::view(
        container(row![gutter, source].spacing(8).height(height))
            .id(format!("unified-line-{index}"))
            .width(Length::Fill)
            .style(move |theme| {
                theme::source_row(theme, theme::unified_row(line.change)(theme), highlighted)
            }),
        gutter_width(document, size, app.workspace.preferences.line_numbers) + 8.,
    )
}

pub(super) fn fold<'a>(
    app: &'a App,
    document: &Document,
    range: Range<usize>,
    height: f32,
) -> Element<'a, Message> {
    let mut args = FluentArgs::new();
    args.set("start", document.line_info[range.start].number);
    args.set("end", document.line_info[range.end - 1].number);
    let hint = app.i18n.format(Key::UnchangedLines, &args);
    let separator = canvas::Canvas::new(FoldSeparator)
        .width(Length::Fill)
        .height(height);
    container(tooltip::view(
        button(separator)
            .width(Length::Fill)
            .height(height)
            .padding(0)
            .style(theme::fold_row)
            .on_press(Message::ExpandUnchanged(app.preview().id, range.clone())),
        text(hint).size(12),
        tooltip::Position::FollowCursor,
    ))
    .id(format!("unchanged-{}-{}", app.preview().id, range.start))
    .width(Length::Fill)
    .into()
}

struct FoldSeparator;

impl<Message> canvas::Program<Message> for FoldSeparator {
    type State = ();
    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let y = bounds.height / 2.;
        let path = Path::new(|path| {
            path.move_to(Point::new(0., y));
            for step in 0..(bounds.width / 12.).ceil() as usize {
                let x = step as f32 * 12.;
                path.bezier_curve_to(
                    Point::new(x + 3., y - 4.),
                    Point::new(x + 3., y - 4.),
                    Point::new(x + 6., y),
                );
                path.bezier_curve_to(
                    Point::new(x + 9., y + 4.),
                    Point::new(x + 9., y + 4.),
                    Point::new(x + 12., y),
                );
            }
        });
        frame.stroke(
            &path,
            Stroke::default()
                .with_width(1.)
                .with_color(theme::FAINT.resolve(theme)),
        );
        vec![frame.into_geometry()]
    }
}
