//! Keep the editor scroll position visible and draggable.

use crate::{
    app::{Message, editor::Event as EditorEvent},
    ui::theme,
};
use iced::{
    Point, Rectangle, Renderer, Theme, mouse,
    widget::{canvas, text_editor},
};

pub(super) struct Scrollbar<'a> {
    pub content: &'a text_editor::Content,
    pub tab: u64,
    pub row_height: f32,
}

impl Scrollbar<'_> {
    fn thumb(&self, bounds: Rectangle) -> Option<(Rectangle, f32, f32)> {
        let rows = self.content.line_count() as f32;
        let visible = (bounds.height - 2. * super::TEXT_PADDING).max(0.) / self.row_height;
        let overflow = rows - visible;
        if overflow <= 0. || bounds.height <= 0. {
            return None;
        }
        let height = (bounds.height * visible / rows).max(28_f32.min(bounds.height / 2.));
        let line = self.content.with_editor(|editor| {
            let scroll = editor.buffer().scroll();
            scroll.line as f32 + scroll.vertical / self.row_height
        });
        let width = theme::CODE_SCROLLBAR_THUMB_WIDTH;
        Some((
            Rectangle {
                x: (bounds.width - width) / 2.,
                y: (bounds.height - height) * (line / overflow).clamp(0., 1.),
                width,
                height,
            },
            line,
            overflow,
        ))
    }
}

impl canvas::Program<Message> for Scrollbar<'_> {
    type State = Option<(u64, f32)>;

    fn mouse_interaction(
        &self,
        grabbed: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if grabbed.is_some_and(|(tab, _)| tab == self.tab) {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) && self.thumb(bounds).is_some() {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }

    fn update(
        &self,
        grabbed: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        if matches!(
            event,
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                | canvas::Event::Window(iced::window::Event::Unfocused)
        ) {
            return grabbed.take().map(|_| canvas::Action::request_redraw());
        }
        let (thumb, line, overflow) = self.thumb(bounds)?;
        let travel = bounds.height - thumb.height;
        let offset = match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let y = cursor.position_in(bounds)?.y;
                let offset = if (thumb.y..=thumb.y + thumb.height).contains(&y) {
                    y - thumb.y
                } else {
                    thumb.height / 2.
                };
                *grabbed = Some((self.tab, offset));
                offset
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                grabbed.filter(|(tab, _)| *tab == self.tab)?.1
            }
            _ => return None,
        };
        let y = cursor.position()?.y - bounds.y;
        let target = ((y - offset) / travel).clamp(0., 1.) * overflow.ceil();
        let lines = (target - line).round() as i32;
        Some(if lines == 0 {
            canvas::Action::request_redraw().and_capture()
        } else {
            canvas::Action::publish(Message::Editor(EditorEvent::Action(
                self.tab,
                text_editor::Action::Scroll { lines },
            )))
            .and_capture()
        })
    }

    fn draw(
        &self,
        grabbed: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let Some((thumb, _, _)) = self.thumb(bounds) else {
            return vec![];
        };
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        frame.fill(
            &canvas::Path::rounded_rectangle(
                Point::new(thumb.x, thumb.y),
                thumb.size(),
                5.0.into(),
            ),
            theme::scrollbar_thumb(
                theme,
                cursor.is_over(bounds) || grabbed.is_some_and(|(tab, _)| tab == self.tab),
            ),
        );
        vec![frame.into_geometry()]
    }
}
