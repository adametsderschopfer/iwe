use crate::domain::repository::GraphRow;
use iced::{
    Color, Element, Point, Rectangle, Renderer, Theme, mouse,
    widget::canvas::{self, Frame, Geometry, Path, Stroke},
};

fn lane_x(lane: usize) -> f32 {
    12. + lane as f32 * 12.
}

pub(crate) fn width(row: &GraphRow) -> f32 {
    let rightmost = row
        .incoming
        .iter()
        .chain(&row.outgoing)
        .fold(row.lane, |lane, &(from, to)| lane.max(from).max(to));
    lane_x(rightmost) + 12.
}

pub(crate) fn view<'a, Message: 'a>(row: &'a GraphRow, width: f32) -> Element<'a, Message> {
    canvas::Canvas::new(Graph(row))
        .width(width)
        .height(30)
        .into()
}

struct Graph<'a>(&'a GraphRow);
impl<Message> canvas::Program<Message> for Graph<'_> {
    type State = ();
    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let colors = [
            Color::from_rgb8(139, 180, 235),
            Color::from_rgb8(129, 201, 156),
            Color::from_rgb8(222, 194, 135),
            Color::from_rgb8(192, 157, 227),
            Color::from_rgb8(223, 145, 170),
        ];
        for &(from, to) in &self.0.incoming {
            frame.stroke(
                &Path::line(Point::new(lane_x(from), 0.), Point::new(lane_x(to), 15.)),
                Stroke::default()
                    .with_width(1.4)
                    .with_color(colors[from % colors.len()]),
            );
        }
        for &(from, to) in &self.0.outgoing {
            frame.stroke(
                &Path::line(Point::new(lane_x(from), 15.), Point::new(lane_x(to), 30.)),
                Stroke::default()
                    .with_width(1.4)
                    .with_color(colors[to % colors.len()]),
            );
        }
        frame.fill(
            &Path::circle(Point::new(lane_x(self.0.lane), 15.), 3.7),
            colors[self.0.lane % colors.len()],
        );
        vec![frame.into_geometry()]
    }
}
