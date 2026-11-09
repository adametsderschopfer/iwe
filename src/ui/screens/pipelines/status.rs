//! Render compact statuses with distinct symbols in both appearances.

use crate::{domain::jobs::Status, i18n::Key, ui::theme};
use iced::{
    Element, Point, Rectangle, Renderer, Theme, mouse,
    widget::canvas::{self, Frame, Geometry, Path, Stroke},
};

pub(super) fn label(status: Status) -> (Key, theme::Tone) {
    match status {
        Status::Passed => (Key::Passed, theme::GREEN),
        Status::Failed => (Key::Failed, theme::RED),
        Status::Running => (Key::Running, theme::BLUE),
        Status::Canceling => (Key::Canceling, theme::YELLOW),
        Status::Pending => (Key::Pending, theme::YELLOW),
        Status::Manual => (Key::Manual, theme::YELLOW),
        Status::Cancelled => (Key::Cancelled, theme::MUTED),
        Status::Skipped => (Key::Skipped, theme::MUTED),
        Status::Unknown => (Key::UnknownStatus, theme::MUTED),
    }
}

pub(super) fn badge<'a, Message: 'a>(status: Status, selected: bool) -> Element<'a, Message> {
    canvas::Canvas::new(Badge(status, selected))
        .width(theme::PIPELINE_BADGE_SIZE)
        .height(theme::PIPELINE_BADGE_SIZE)
        .into()
}

struct Badge(Status, bool);

impl<Message> canvas::Program<Message> for Badge {
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
        let center = Point::new(15., 15.);
        let color = match self.0 {
            Status::Passed => theme::PIPELINE_GREEN,
            Status::Failed => theme::PIPELINE_RED,
            Status::Running => theme::PIPELINE_BLUE,
            Status::Pending | Status::Manual | Status::Canceling => theme::PIPELINE_AMBER,
            Status::Cancelled | Status::Skipped | Status::Unknown => theme::PIPELINE_GRAY,
        }
        .resolve(theme);
        frame.fill(&Path::circle(center, 13.), color.scale_alpha(0.24));
        frame.fill(&Path::circle(center, 9.5), color);
        if self.1 {
            frame.stroke(
                &Path::circle(center, 14.),
                Stroke::default()
                    .with_width(1.5)
                    .with_color(theme::BLUE.resolve(theme)),
            );
        }
        let stroke = Stroke::default()
            .with_width(2.)
            .with_color(theme::PIPELINE_GLYPH.resolve(theme));
        let mut line = |points: &[(f32, f32)]| {
            let path = Path::new(|builder| {
                if let Some(&(x, y)) = points.first() {
                    builder.move_to(Point::new(x, y));
                }
                for &(x, y) in points.iter().skip(1) {
                    builder.line_to(Point::new(x, y));
                }
            });
            frame.stroke(&path, stroke);
        };
        match self.0 {
            Status::Passed => line(&[(10., 15.), (14., 19.), (21., 11.)]),
            Status::Failed => {
                line(&[(11., 11.), (19., 19.)]);
                line(&[(19., 11.), (11., 19.)]);
            }
            Status::Running => {
                for x in [10.5, 15., 19.5] {
                    frame.fill(
                        &Path::circle(Point::new(x, 15.), 1.4),
                        theme::PIPELINE_GLYPH.resolve(theme),
                    );
                }
            }
            Status::Pending => {
                line(&[(15., 9.), (15., 15.), (19., 17.)]);
            }
            Status::Manual => line(&[(12., 10.), (20., 15.), (12., 20.), (12., 10.)]),
            Status::Canceling => {
                line(&[(12., 10.), (12., 20.)]);
                line(&[(18., 10.), (18., 20.)]);
            }
            Status::Cancelled => line(&[(11., 19.), (19., 11.)]),
            Status::Skipped => line(&[(10., 15.), (20., 15.)]),
            Status::Unknown => {
                line(&[
                    (11., 11.),
                    (13., 9.),
                    (17., 9.),
                    (19., 11.),
                    (19., 13.),
                    (15., 16.),
                ]);
                frame.fill(
                    &Path::circle(Point::new(15., 20.), 1.),
                    theme::PIPELINE_GLYPH.resolve(theme),
                );
            }
        }
        vec![frame.into_geometry()]
    }
}
