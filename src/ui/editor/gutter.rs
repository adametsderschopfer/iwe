//! Draw line numbers from the editor's actual layout and scroll position.

use crate::ui::theme;
use iced::{
    Point, Rectangle, Renderer, Theme, mouse,
    widget::{canvas, text_editor},
};

pub(super) struct Gutter<'a> {
    pub content: &'a text_editor::Content,
    pub size: f32,
}

impl<Message> canvas::Program<Message> for Gutter<'_> {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        frame.with_clip(Rectangle::with_size(bounds.size()), |frame| {
            self.content.with_editor(|editor| {
                for line in editor.buffer().layout_runs() {
                    frame.fill_text(canvas::Text {
                        content: (line.line_i + 1).to_string(),
                        position: Point::new(
                            bounds.width - 8.,
                            super::TEXT_PADDING + line.line_top,
                        ),
                        color: theme::MUTED.resolve(theme),
                        size: self.size.into(),
                        line_height: iced::widget::text::LineHeight::Relative(super::LINE_HEIGHT),
                        font: theme::code_font(),
                        align_x: iced::widget::text::Alignment::Right,
                        ..Default::default()
                    });
                }
            });
        });
        vec![frame.into_geometry()]
    }
}
