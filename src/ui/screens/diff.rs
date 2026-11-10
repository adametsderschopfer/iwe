//! Render synchronized comparison panes with shared folding and change connectors.

use super::workspace::divider;
use crate::{
    app::{App, Message},
    i18n::{FluentArgs, Key},
    ui::{
        code::{
            self, Document, LineChange,
            comparison::{Cell, Comparison, Hunk, Side},
            folding::SideRow,
        },
        file_icons, theme,
    },
};
use iced::{
    Alignment, Color, Element, Length, Point, Rectangle, Renderer, Theme, mouse,
    widget::{
        Row, button,
        canvas::{self, Frame, Geometry, Path, Stroke},
        column, container, responsive, row, scrollable, space, text,
    },
};

const BRIDGE_WIDTH: f32 = 32.;

pub(super) fn view<'a>(
    app: &'a App,
    document: &'a Document,
    comparison: &'a Comparison,
) -> Element<'a, Message> {
    let heading = row![
        container(
            text(app.i18n.text(if app.preview().remote {
                Key::DeploymentRemote
            } else {
                Key::DiffBefore
            }))
            .size(12)
            .style(theme::MUTED.text())
        )
        .padding([6, 12])
        .width(Length::Fill),
        space().width(BRIDGE_WIDTH),
        container(
            text(app.i18n.text(if app.preview().remote {
                Key::DeploymentLocal
            } else {
                Key::DiffAfter
            }))
            .size(12)
            .style(theme::MUTED.text())
        )
        .padding([6, 12])
        .width(Length::Fill),
    ];
    let bridges = canvas::Canvas::new(Connectors {
        hunks: app.preview().folding.side_hunks(comparison),
        row_height: app.workspace.preferences.code_size as f32 * 1.65,
        offset: app.preview().diff_scroll_y,
    })
    .width(BRIDGE_WIDTH)
    .height(Length::Fill);
    column![
        container(heading).style(theme::surface),
        divider(),
        row![
            pane(app, document, comparison, Side::Before),
            column![bridges, space().height(12)].height(Length::Fill),
            pane(app, document, comparison, Side::After),
        ]
        .height(Length::Fill),
    ]
    .height(Length::Fill)
    .into()
}

fn pane<'a>(
    app: &'a App,
    document: &'a Document,
    comparison: &'a Comparison,
    side: Side,
) -> Element<'a, Message> {
    responsive(move |viewport| {
        let preview = app.preview();
        let size = app.workspace.preferences.code_size as f32;
        let height = size * 1.65;
        let gutter = if app.workspace.preferences.line_numbers {
            68.
        } else {
            12.
        };
        let width = (comparison.columns[side.index()] as f32 * size * 0.65
            + gutter
            + 12.
            + theme::CODE_SCROLLBAR_WIDTH)
            .max(viewport.width);
        let count = preview.folding.side_len(comparison);
        let visible = code::visible_rows(preview.diff_scroll_y, viewport.height, height, count);
        let (start, end) = (visible.start, visible.end);
        let lines = (start..end).map(|row| match preview.folding.side_row(row) {
            SideRow::Fold { range, numbers } => {
                let mut args = FluentArgs::new();
                args.set("start", numbers[side.index()].start);
                args.set("end", numbers[side.index()].end - 1);
                container(
                    button(
                        row![
                            file_icons::chevron(false),
                            text(app.i18n.format(Key::UnchangedLines, &args))
                                .size(12)
                                .wrapping(text::Wrapping::None)
                        ]
                        .spacing(8)
                        .height(height)
                        .align_y(Alignment::Center),
                    )
                    .padding([0, 12])
                    .width(Length::Fill)
                    .height(height)
                    .style(theme::fold_row)
                    .on_press(Message::ExpandUnchanged(preview.id, range.clone())),
                )
                .id(format!(
                    "diff-fold-{}-{}-{}",
                    preview.id,
                    side.index(),
                    range.start
                ))
                .width(Length::Fill)
                .into()
            }
            SideRow::Line(index) => {
                let pair = &comparison.rows[index];
                match pair.cells[side.index()].as_ref() {
                    Some(cell) => line(app, document, cell, pair.change, side, height),
                    None => container(space().height(height))
                        .width(Length::Fill)
                        .style(theme::sidebar)
                        .into(),
                }
            }
        });
        let scroll = scrollable(
            container(column![
                space().height(start as f32 * height),
                code::rows(
                    (
                        preview.id,
                        document.fingerprint,
                        preview.folding.revision,
                        preview.selection.revision,
                        app.workspace.preferences.code_size,
                        app.workspace.preferences.line_numbers,
                        app.workspace.preferences.appearance,
                        app.i18n.text(if app.preview().remote {
                            Key::DeploymentRemote
                        } else {
                            Key::DiffBefore
                        }),
                        side
                    ),
                    start..end,
                    lines
                ),
                space().height((count - end) as f32 * height),
            ])
            .width(width)
            .padding(iced::Padding {
                bottom: 12.,
                right: theme::CODE_SCROLLBAR_WIDTH,
                ..iced::Padding::ZERO
            }),
        )
        .id(side.scroll_id())
        .direction(scrollable::Direction::Both {
            vertical: code::overview::scrollbar(),
            horizontal: scrollable::Scrollbar::default(),
        })
        .on_scroll(move |viewport| {
            let offset = viewport.absolute_offset();
            Message::DiffScrolled {
                id: preview.id,
                side,
                x: offset.x,
                y: offset.y,
            }
        })
        .style(theme::code_scroll)
        .width(Length::Fill)
        .height(Length::Fill);
        code::overview::view(
            scroll,
            document,
            &preview.folding,
            Some(side),
            code::overview::Layout {
                rows: count,
                row_height: height,
                viewport,
                content_width: width,
            },
        )
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn line<'a>(
    app: &App,
    document: &'a Document,
    cell: &Cell,
    change: Option<LineChange>,
    side: Side,
    height: f32,
) -> Element<'a, Message> {
    let size = app.workspace.preferences.code_size as f32;
    let highlighted = app.preview().selection.highlights_line(
        document,
        side.into(),
        cell.number.saturating_sub(1),
    );
    let marker = change
        .map(|change| theme::line_change(change).resolve(&app.theme()))
        .unwrap_or(Color::TRANSPARENT);
    let spans = code::emphasized_spans(
        &document.lines[cell.index],
        &cell.emphasis,
        change
            .map(|change| theme::change_fill(theme::line_change(change), &app.theme(), 0.28))
            .unwrap_or(Color::TRANSPARENT),
        &app.theme(),
    );
    let code = code::selection::line(
        app,
        document,
        cell.index,
        side.into(),
        cell.number.saturating_sub(1),
        spans,
        size,
    );
    let mut row = Row::new()
        .spacing(8)
        .height(height)
        .width(Length::Fill)
        .push(
            container(space())
                .width(3)
                .height(height)
                .style(move |_| container::Style {
                    background: Some(marker.into()),
                    ..Default::default()
                }),
        );
    if app.workspace.preferences.line_numbers {
        row = row.push(
            text(cell.number)
                .size(size)
                .font(theme::code_font())
                .line_height(1.65)
                .style(theme::FAINT.text())
                .width(46)
                .align_x(Alignment::End),
        );
    }
    code::frozen_gutter::view(
        container(row.push(code))
            .width(Length::Fill)
            .id(format!("diff-line-{}-{}", side.index(), cell.number))
            .style(move |theme| {
                theme::source_row(
                    theme,
                    container::Style {
                        background: change.map(|change| {
                            theme::change_fill(theme::line_change(change), theme, 0.17).into()
                        }),
                        ..Default::default()
                    },
                    highlighted,
                )
            }),
        if app.workspace.preferences.line_numbers {
            theme::DIFF_GUTTER_WIDTH
        } else {
            theme::DIFF_MARKERS_WIDTH
        },
    )
}

struct Connectors<'a> {
    hunks: &'a [Hunk],
    row_height: f32,
    offset: f32,
}

impl<Message> canvas::Program<Message> for Connectors<'_> {
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
        let width = bounds.width;
        let first = self
            .hunks
            .partition_point(|hunk| hunk.end() as f32 * self.row_height < self.offset);
        for hunk in self.hunks[first..]
            .iter()
            .take_while(|hunk| hunk.start as f32 * self.row_height < self.offset + bounds.height)
        {
            let top = hunk.start as f32 * self.row_height - self.offset;
            let left = top + hunk.lengths[0] as f32 * self.row_height;
            let right = top + hunk.lengths[1] as f32 * self.row_height;
            let path = Path::new(|path| {
                path.move_to(Point::new(0., top));
                path.line_to(Point::new(width, top));
                path.line_to(Point::new(width, right));
                path.bezier_curve_to(
                    Point::new(width * 0.55, right),
                    Point::new(width * 0.45, left),
                    Point::new(0., left),
                );
                path.close();
            });
            let color = theme::line_change(hunk.change).resolve(theme);
            frame.fill(
                &path,
                theme::change_fill(theme::line_change(hunk.change), theme, 0.17),
            );
            frame.stroke(
                &path,
                Stroke::default()
                    .with_width(1.)
                    .with_color(Color { a: 0.45, ..color }),
            );
        }
        for x in [0., width] {
            frame.stroke(
                &Path::line(Point::new(x, 0.), Point::new(x, bounds.height)),
                Stroke::default()
                    .with_width(1.)
                    .with_color(theme::LINE.resolve(theme)),
            );
        }
        vec![frame.into_geometry()]
    }
}
