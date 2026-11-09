//! Draw compact file-type symbols without platform icon fonts or external assets.

use super::theme;

use iced::{
    Color, Element, Font, Point, Rectangle, Renderer, Theme, font, mouse,
    widget::canvas::{self, Frame, Geometry, Path, Stroke, Text},
};
use std::path::Path as FilePath;

#[derive(Clone, Copy)]
enum Kind {
    Python,
    JavaScript,
    TypeScript,
    React,
    Vue,
    Svelte,
    Rust,
    Go,
    Html,
    Css,
    Sass,
    Json,
    Yaml,
    Toml,
    Markdown,
    Docker,
    Make,
    Shell,
    Database,
    Image,
    Git,
    GitLab,
    Lock,
    Settings,
    Text,
    Folder(bool, bool),
    Chevron(bool),
}

pub(crate) fn file<'a, Message: 'a>(path: &FilePath) -> Element<'a, Message> {
    draw(kind(path))
}

pub(crate) fn folder<'a, Message: 'a>(expanded: bool, package: bool) -> Element<'a, Message> {
    draw(Kind::Folder(expanded, package))
}

pub(crate) fn chevron<'a, Message: 'a>(expanded: bool) -> Element<'a, Message> {
    canvas::Canvas::new(Drawing(Kind::Chevron(expanded)))
        .width(10)
        .height(16)
        .into()
}

fn draw<'a, Message: 'a>(kind: Kind) -> Element<'a, Message> {
    canvas::Canvas::new(Drawing(kind))
        .width(16)
        .height(16)
        .into()
}

fn kind(path: &FilePath) -> Kind {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    match name.as_str() {
        ".gitlab-ci.yml" => return Kind::GitLab,
        ".gitignore" | ".gitattributes" | ".gitmodules" => return Kind::Git,
        "dockerfile" | "containerfile" => return Kind::Docker,
        "makefile" | "gnumakefile" => return Kind::Make,
        ".env" | ".editorconfig" => return Kind::Settings,
        "yarn.lock" | "package-lock.json" | "pnpm-lock.yaml" => return Kind::Lock,
        _ => {}
    }
    if name.starts_with("dockerfile.")
        || name.starts_with("docker-compose.")
        || name.starts_with("compose.")
    {
        return Kind::Docker;
    }
    if name.starts_with(".env.") {
        return Kind::Settings;
    }
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "py" | "pyi" | "pyw" => Kind::Python,
        "js" | "mjs" | "cjs" => Kind::JavaScript,
        "ts" | "mts" | "cts" => Kind::TypeScript,
        "jsx" | "tsx" => Kind::React,
        "vue" => Kind::Vue,
        "svelte" => Kind::Svelte,
        "rs" => Kind::Rust,
        "go" => Kind::Go,
        "html" | "htm" | "xml" => Kind::Html,
        "css" | "less" => Kind::Css,
        "scss" | "sass" => Kind::Sass,
        "json" | "jsonc" => Kind::Json,
        "yaml" | "yml" => Kind::Yaml,
        "toml" => Kind::Toml,
        "md" | "mdx" | "markdown" => Kind::Markdown,
        "sh" | "bash" | "zsh" | "fish" => Kind::Shell,
        "sql" | "sqlite" | "db" => Kind::Database,
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" | "ico" => Kind::Image,
        "lock" => Kind::Lock,
        _ => Kind::Text,
    }
}

struct Drawing(Kind);

fn line(frame: &mut Frame, points: &[(f32, f32)], color: Color, width: f32) {
    let path = Path::new(|builder| {
        if let Some(&(x, y)) = points.first() {
            builder.move_to(Point::new(x, y));
        }
        for &(x, y) in points.iter().skip(1) {
            builder.line_to(Point::new(x, y));
        }
    });
    frame.stroke(&path, Stroke::default().with_color(color).with_width(width));
}

fn shape(frame: &mut Frame, points: &[(f32, f32)], color: Color) {
    let path = Path::new(|builder| {
        if let Some(&(x, y)) = points.first() {
            builder.move_to(Point::new(x, y));
        }
        for &(x, y) in points.iter().skip(1) {
            builder.line_to(Point::new(x, y));
        }
        builder.close();
    });
    frame.fill(&path, color);
}

fn label(frame: &mut Frame, value: &str, color: Color, size: f32) {
    frame.fill_text(Text {
        content: value.to_owned(),
        position: Point::new(8., 8.),
        color,
        size: size.into(),
        font: Font {
            weight: font::Weight::Bold,
            ..Font::MONOSPACE
        },
        align_x: iced::alignment::Horizontal::Center.into(),
        align_y: iced::alignment::Vertical::Center,
        ..Default::default()
    });
}

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
        let gray = theme::ICON_GRAY.resolve(theme);
        let blue = theme::ICON_BLUE.resolve(theme);
        let yellow = theme::ICON_YELLOW.resolve(theme);
        let cyan = theme::ICON_CYAN.resolve(theme);
        let orange = theme::ICON_ORANGE.resolve(theme);
        let pink = theme::ICON_PINK.resolve(theme);
        match self.0 {
            Kind::Chevron(expanded) => line(
                &mut frame,
                if expanded {
                    &[(2., 6.), (5., 9.), (8., 6.)]
                } else {
                    &[(3., 4.), (6., 7.), (3., 10.)]
                },
                gray,
                1.1,
            ),
            Kind::Folder(expanded, package) => {
                line(
                    &mut frame,
                    &[
                        (1.5, 4.),
                        (6., 4.),
                        (7.5, 6.),
                        (14.5, 6.),
                        (14.5, 13.),
                        (1.5, 13.),
                        (1.5, 4.),
                    ],
                    gray,
                    1.1,
                );
                if expanded {
                    line(&mut frame, &[(1.5, 8.), (14.5, 8.)], gray, 0.8);
                }
                if package {
                    frame.stroke(
                        &Path::circle(Point::new(8., 9.5), 1.5),
                        Stroke::default().with_color(gray).with_width(1.),
                    );
                }
            }
            Kind::Python => {
                let upper = Path::new(|p| {
                    p.move_to(Point::new(8., 1.));
                    p.bezier_curve_to(Point::new(3., 1.), Point::new(3., 2.), Point::new(3., 5.));
                    p.line_to(Point::new(8., 5.));
                    p.line_to(Point::new(8., 6.));
                    p.line_to(Point::new(2.5, 6.));
                    p.bezier_curve_to(Point::new(0., 6.), Point::new(0., 11.), Point::new(3., 11.));
                    p.line_to(Point::new(4., 11.));
                    p.line_to(Point::new(4., 9.));
                    p.bezier_curve_to(Point::new(4., 7.), Point::new(5., 7.), Point::new(8., 7.));
                    p.line_to(Point::new(11.5, 7.));
                    p.line_to(Point::new(11.5, 3.));
                    p.bezier_curve_to(
                        Point::new(11.5, 1.),
                        Point::new(10., 1.),
                        Point::new(8., 1.),
                    );
                    p.close();
                });
                frame.fill(&upper, blue);
                frame.with_save(|f| {
                    f.translate(iced::Vector::new(16., 16.));
                    f.rotate(std::f32::consts::PI);
                    f.fill(&upper, yellow);
                });
                frame.fill(&Path::circle(Point::new(5.5, 3.), 0.7), Color::WHITE);
                frame.fill(&Path::circle(Point::new(10.5, 13.), 0.7), Color::WHITE);
            }
            Kind::JavaScript | Kind::TypeScript => {
                let js = matches!(self.0, Kind::JavaScript);
                frame.fill_rectangle(
                    Point::new(1., 1.),
                    iced::Size::new(14., 14.),
                    if js { yellow } else { blue },
                );
                label(
                    &mut frame,
                    if js { "JS" } else { "TS" },
                    if js {
                        Color::from_rgb8(41, 41, 41)
                    } else {
                        Color::WHITE
                    },
                    9.,
                );
            }
            Kind::React => {
                for angle in [
                    0.,
                    std::f32::consts::PI / 3.,
                    2. * std::f32::consts::PI / 3.,
                ] {
                    let points: Vec<_> = (0..=36)
                        .map(|step| {
                            let theta = step as f32 * std::f32::consts::TAU / 36.;
                            let (x, y) = (7. * theta.cos(), 2.5 * theta.sin());
                            (
                                8. + x * angle.cos() - y * angle.sin(),
                                8. + x * angle.sin() + y * angle.cos(),
                            )
                        })
                        .collect();
                    line(&mut frame, &points, cyan, 0.9);
                }
                frame.fill(&Path::circle(Point::new(8., 8.), 1.3), cyan);
            }
            Kind::Vue => {
                shape(
                    &mut frame,
                    &[
                        (1., 2.),
                        (4., 2.),
                        (8., 9.),
                        (12., 2.),
                        (15., 2.),
                        (8., 15.),
                    ],
                    Color::from_rgb8(66, 184, 131),
                );
                shape(
                    &mut frame,
                    &[
                        (4., 2.),
                        (6.5, 2.),
                        (8., 4.5),
                        (9.5, 2.),
                        (12., 2.),
                        (8., 9.),
                    ],
                    Color::from_rgb8(87, 120, 140),
                );
            }
            Kind::Docker => {
                for (x, y) in [(2., 6.), (5., 6.), (8., 6.), (5., 3.), (8., 3.), (8., 0.)] {
                    frame.fill_rectangle(Point::new(x, y), iced::Size::new(2.5, 2.5), blue);
                }
                shape(
                    &mut frame,
                    &[
                        (1., 9.5),
                        (12., 9.5),
                        (13., 7.),
                        (15., 7.5),
                        (13., 10.5),
                        (10., 14.),
                        (4., 14.),
                        (2., 12.),
                    ],
                    blue,
                );
            }
            Kind::GitLab => {
                shape(
                    &mut frame,
                    &[
                        (1., 2.),
                        (5., 6.),
                        (11., 6.),
                        (15., 2.),
                        (16., 9.),
                        (8., 15.),
                        (0., 9.),
                    ],
                    orange,
                );
                shape(
                    &mut frame,
                    &[(5., 6.), (11., 6.), (8., 15.)],
                    Color::from_rgb8(226, 78, 48),
                );
            }
            Kind::Git => {
                frame.stroke(
                    &Path::circle(Point::new(8., 8.), 6.),
                    Stroke::default().with_color(gray).with_width(1.),
                );
                line(&mut frame, &[(4., 12.), (12., 4.)], gray, 1.);
            }
            Kind::Lock => {
                line(
                    &mut frame,
                    &[
                        (5., 7.),
                        (5., 4.),
                        (6., 2.),
                        (10., 2.),
                        (11., 4.),
                        (11., 7.),
                    ],
                    gray,
                    1.2,
                );
                line(
                    &mut frame,
                    &[(3., 7.), (13., 7.), (13., 14.), (3., 14.), (3., 7.)],
                    gray,
                    1.2,
                );
                line(&mut frame, &[(8., 9.), (8., 12.)], gray, 1.2);
            }
            Kind::Text => {
                for (y, end) in [(4., 14.), (7., 12.), (10., 14.), (13., 9.)] {
                    line(&mut frame, &[(2., y), (end, y)], gray, 1.1);
                }
            }
            Kind::Image => {
                line(
                    &mut frame,
                    &[(1., 2.), (15., 2.), (15., 14.), (1., 14.), (1., 2.)],
                    pink,
                    1.,
                );
                line(
                    &mut frame,
                    &[(2., 12.), (6., 7.), (9., 10.), (12., 6.), (14., 9.)],
                    pink,
                    1.,
                );
                frame.fill(&Path::circle(Point::new(5., 5.), 1.), pink);
            }
            Kind::Settings => {
                frame.stroke(
                    &Path::circle(Point::new(8., 8.), 4.),
                    Stroke::default().with_color(gray).with_width(1.2),
                );
                frame.stroke(
                    &Path::circle(Point::new(8., 8.), 1.5),
                    Stroke::default().with_color(gray).with_width(1.),
                );
                for index in 0..8 {
                    let angle = index as f32 * std::f32::consts::TAU / 8.;
                    line(
                        &mut frame,
                        &[
                            (8. + 4. * angle.cos(), 8. + 4. * angle.sin()),
                            (8. + 6.5 * angle.cos(), 8. + 6.5 * angle.sin()),
                        ],
                        gray,
                        1.2,
                    );
                }
            }
            other => {
                let (text, color, size) = match other {
                    Kind::Rust => ("R", orange, 13.),
                    Kind::Go => ("GO", cyan, 9.),
                    Kind::Html => ("<>", orange, 11.),
                    Kind::Css => ("#", blue, 14.),
                    Kind::Sass => ("S", pink, 13.),
                    Kind::Svelte => ("S", orange, 13.),
                    Kind::Json => ("{}", yellow, 11.),
                    Kind::Yaml => ("Y", Color::from_rgb8(215, 104, 99), 12.),
                    Kind::Toml => ("T", blue, 12.),
                    Kind::Markdown => ("M↓", blue, 9.),
                    Kind::Make => ("M", blue, 12.),
                    Kind::Shell => (">_", gray, 10.),
                    Kind::Database => ("DB", pink, 9.),
                    _ => unreachable!(),
                };
                label(&mut frame, text, color, size);
            }
        }
        vec![frame.into_geometry()]
    }
}
