//! Draw small, consistent outline icons without a platform icon font.

use iced::{
    Color, Element, Point, Rectangle, Renderer, Theme, mouse,
    widget::canvas::{self, Frame, Geometry, Path, Stroke},
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Icon {
    FolderOpen,
    CollapseLines,
    ExpandLines,
    Back,
    Branch,
    Tag,
    Play,
    Settings,
    Plus,
    Right,
    Down,
    Refresh,
    Close,
    Folder,
    External,
}

pub(crate) fn icon<'a, Message: 'a>(icon: Icon, color: Color) -> Element<'a, Message> {
    canvas::Canvas::new(Drawing(icon, color))
        .width(20)
        .height(20)
        .into()
}

struct Drawing(Icon, Color);

impl<Message> canvas::Program<Message> for Drawing {
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
        let stroke = Stroke::default().with_width(1.5).with_color(self.1);
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
            Icon::FolderOpen => {
                line(&[
                    (2., 16.),
                    (2., 4.),
                    (8., 4.),
                    (10., 6.),
                    (17., 6.),
                    (17., 9.),
                ]);
                line(&[(2., 16.), (5., 9.), (19., 9.), (16., 16.), (2., 16.)]);
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
