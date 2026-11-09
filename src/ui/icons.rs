//! Draw small, consistent outline icons without a platform icon font.

use super::theme::Tone;

use iced::{
    Element, Point, Rectangle, Renderer, Theme, mouse,
    widget::canvas::{self, Frame, Geometry, Path, Stroke},
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Icon {
    CollapseLines,
    ExpandLines,
    SplitDiff,
    UnifiedDiff,
    Back,
    Branch,
    Tag,
    Play,
    Settings,
    Plus,
    Right,
    Down,
    Drag,
    Changes,
    Fetch,
    Upload,
    Pin,
    Refresh,
    Close,
    Folder,
    External,
}

pub(crate) fn icon<'a, Message: 'a>(icon: Icon, color: Tone) -> Element<'a, Message> {
    canvas::Canvas::new(Drawing(icon, color))
        .width(20)
        .height(20)
        .into()
}

struct Drawing(Icon, Tone);

impl<Message> canvas::Program<Message> for Drawing {
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
        let stroke = Stroke::default()
            .with_width(1.5)
            .with_color(self.1.resolve(theme));
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
            Icon::SplitDiff => {
                line(&[(2., 3.), (18., 3.), (18., 17.), (2., 17.), (2., 3.)]);
                line(&[(10., 3.), (10., 17.)]);
                line(&[(4., 7.), (7., 7.)]);
                line(&[(13., 11.), (16., 11.)]);
            }
            Icon::UnifiedDiff => {
                line(&[(2., 3.), (18., 3.), (18., 17.), (2., 17.), (2., 3.)]);
                line(&[(5., 7.), (15., 7.)]);
                line(&[(5., 10.), (15., 10.)]);
                line(&[(5., 13.), (15., 13.)]);
            }
            Icon::Changes => {
                line(&[
                    (4., 2.),
                    (13., 2.),
                    (17., 6.),
                    (17., 18.),
                    (4., 18.),
                    (4., 2.),
                ]);
                line(&[(7., 8.), (13., 8.)]);
                line(&[(7., 12.), (13., 12.)]);
            }
            Icon::Fetch => {
                line(&[(10., 2.), (10., 13.)]);
                line(&[(6., 9.), (10., 13.), (14., 9.)]);
                line(&[(3., 14.), (3., 18.), (17., 18.), (17., 14.)]);
            }
            Icon::Upload => {
                line(&[(10., 14.), (10., 3.)]);
                line(&[(6., 7.), (10., 3.), (14., 7.)]);
                line(&[(3., 14.), (3., 18.), (17., 18.), (17., 14.)]);
            }
            Icon::Pin => {
                line(&[
                    (7., 2.),
                    (14., 2.),
                    (13., 8.),
                    (16., 12.),
                    (4., 12.),
                    (7., 8.),
                    (7., 2.),
                ]);
                line(&[(10., 12.), (10., 19.)]);
            }
            Icon::CollapseLines => {
                line(&[(3., 10.), (17., 10.)]);
                line(&[(10., 1.), (10., 6.)]);
                line(&[(7., 3.), (10., 6.), (13., 3.)]);
                line(&[(10., 19.), (10., 14.)]);
                line(&[(7., 17.), (10., 14.), (13., 17.)]);
            }
            Icon::ExpandLines => {
                line(&[(3., 10.), (17., 10.)]);
                line(&[(10., 6.), (10., 1.)]);
                line(&[(7., 4.), (10., 1.), (13., 4.)]);
                line(&[(10., 14.), (10., 19.)]);
                line(&[(7., 16.), (10., 19.), (13., 16.)]);
            }
            Icon::Back => {
                line(&[(9., 4.), (3., 10.), (9., 16.)]);
                line(&[(3., 10.), (17., 10.)]);
            }
            Icon::Folder => line(&[
                (2., 5.),
                (8., 5.),
                (10., 7.),
                (18., 7.),
                (18., 17.),
                (2., 17.),
                (2., 5.),
            ]),
            Icon::Plus => {
                line(&[(10., 3.), (10., 17.)]);
                line(&[(3., 10.), (17., 10.)]);
            }
            Icon::Right => line(&[(7., 5.), (12., 10.), (7., 15.)]),
            Icon::Down => line(&[(5., 7.), (10., 12.), (15., 7.)]),
            Icon::Drag => {
                for y in [5., 10., 15.] {
                    line(&[(6., y), (7., y)]);
                    line(&[(13., y), (14., y)]);
                }
            }
            Icon::Close => {
                line(&[(5., 5.), (15., 15.)]);
                line(&[(5., 15.), (15., 5.)]);
            }
            Icon::External => {
                line(&[(11., 3.), (17., 3.), (17., 9.)]);
                line(&[(17., 3.), (9., 11.)]);
                line(&[(8., 3.), (3., 3.), (3., 17.), (17., 17.), (17., 12.)]);
            }
            Icon::Play => {
                line(&[(7., 5.), (15., 10.), (7., 15.), (7., 5.)]);
                frame.stroke(&Path::circle(Point::new(10., 10.), 9.), stroke);
            }
            Icon::Branch => {
                line(&[(5., 5.), (5., 15.)]);
                line(&[(15., 5.), (15., 9.), (5., 13.)]);
                for (x, y) in [(5., 3.), (5., 17.), (15., 3.)] {
                    frame.stroke(&Path::circle(Point::new(x, y), 2.), stroke);
                }
            }
            Icon::Tag => {
                line(&[
                    (2., 3.),
                    (10., 3.),
                    (18., 11.),
                    (11., 18.),
                    (2., 9.),
                    (2., 3.),
                ]);
                frame.stroke(&Path::circle(Point::new(6., 7.), 1.5), stroke);
            }
            Icon::Refresh => {
                line(&[(16., 3.), (16., 8.), (11., 8.)]);
                line(&[
                    (16., 8.),
                    (13., 4.),
                    (8., 3.),
                    (4., 6.),
                    (3., 11.),
                    (6., 16.),
                    (12., 17.),
                    (16., 14.),
                ]);
            }
            Icon::Settings => {
                let cog = Path::new(|builder| {
                    for index in 0..32 {
                        let angle = index as f32 * std::f32::consts::TAU / 32.;
                        let radius = if index % 4 == 0 || index % 4 == 3 {
                            8.8
                        } else {
                            6.8
                        };
                        let point =
                            Point::new(10. + radius * angle.cos(), 10. + radius * angle.sin());
                        if index == 0 {
                            builder.move_to(point);
                        } else {
                            builder.line_to(point);
                        }
                    }
                    builder.close();
                });
                frame.stroke(&cog, stroke);
                frame.stroke(&Path::circle(Point::new(10., 10.), 3.), stroke);
            }
        }
        vec![frame.into_geometry()]
    }
}
