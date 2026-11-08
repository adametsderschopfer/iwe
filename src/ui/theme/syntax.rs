//! Match the Gerry Darcula palette used by the reference JetBrains editor.

use iced::Color;
use two_face::re_exports::syntect::highlighting::{
    Color as SyntaxColor, StyleModifier, Theme, ThemeItem, ThemeSettings,
};

pub(crate) const TEXT: Color = Color::from_rgb8(175, 185, 195);
pub(crate) const KEYWORD: Color = Color::from_rgb8(200, 140, 220);
pub(crate) const FUNCTION: Color = Color::from_rgb8(255, 200, 110);
pub(crate) const STRING: Color = Color::from_rgb8(150, 190, 120);
pub(crate) const COMMENT: Color = Color::from_rgb8(128, 128, 128);
pub(crate) const NUMBER: Color = Color::from_rgb8(54, 190, 255);
pub(crate) const PARAMETER: Color = Color::from_rgb8(97, 175, 225);
pub(crate) const FIELD: Color = Color::from_rgb8(255, 150, 150);
pub(crate) const ESCAPE: Color = Color::from_rgb8(23, 187, 221);
pub(crate) const DECORATOR: Color = Color::from_rgb8(187, 181, 41);

fn color(color: Color) -> SyntaxColor {
    let [r, g, b, a] = color.into_rgba8();
    SyntaxColor { r, g, b, a }
}

pub(crate) fn palette() -> Theme {
    let rules = [
        ("comment", COMMENT),
        ("keyword, storage, constant.language", KEYWORD),
        ("string, constant.other", STRING),
        ("constant.numeric", NUMBER),
        (
            "entity.name.function, support.function, variable.function, entity.name.type, support.type, entity.other.inherited-class",
            FUNCTION,
        ),
        ("variable.parameter", PARAMETER),
        (
            "variable.other.member, variable.other.property, entity.name.tag",
            FIELD,
        ),
        (
            "entity.other.attribute-name, support.type.property-name, meta.mapping.key string",
            FUNCTION,
        ),
        ("constant.character.escape, string.regexp", ESCAPE),
        (
            "meta.annotation, meta.function.decorator, entity.name.function.decorator, punctuation.definition.annotation",
            DECORATOR,
        ),
        ("keyword.operator", TEXT),
        (
            "keyword.operator.word, keyword.operator.logical.python",
            KEYWORD,
        ),
        (
            "source.css support.function, source.css entity.other.attribute-name",
            PARAMETER,
        ),
    ];
    Theme {
        name: Some("Gerry Darcula".to_owned()),
        settings: ThemeSettings {
            foreground: Some(color(TEXT)),
            ..Default::default()
        },
        scopes: rules
            .into_iter()
            .map(|(scope, foreground)| ThemeItem {
                scope: scope.parse().expect("Valid embedded syntax scopes"),
                style: StyleModifier {
                    foreground: Some(color(foreground)),
                    ..Default::default()
                },
            })
            .collect(),
        ..Default::default()
    }
}
