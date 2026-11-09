//! Share syntax roles between Darcula and the light editor palette.

use super::Tone;
use two_face::re_exports::syntect::highlighting::{
    Color as SyntaxColor, StyleModifier, Theme, ThemeItem, ThemeSettings,
};

pub(crate) const TEXT: Tone = Tone::new(0xafb9c3, 0x30343b);
pub(crate) const KEYWORD: Tone = Tone::new(0xc88cdc, 0x7b288c);
pub(crate) const FUNCTION: Tone = Tone::new(0xffc86e, 0x785400);
pub(crate) const STRING: Tone = Tone::new(0x96be78, 0x2d661d);
pub(crate) const COMMENT: Tone = Tone::new(0x808080, 0x586058);
pub(crate) const NUMBER: Tone = Tone::new(0x36beff, 0x00618b);
pub(crate) const PARAMETER: Tone = Tone::new(0x61afe1, 0x245e8b);
pub(crate) const FIELD: Tone = Tone::new(0xff9696, 0x9a365a);
pub(crate) const ESCAPE: Tone = Tone::new(0x17bbdd, 0x00656c);
pub(crate) const DECORATOR: Tone = Tone::new(0xbbb529, 0x645900);

fn color(color: Tone) -> SyntaxColor {
    let [r, g, b, a] = color.dark.into_rgba8();
    SyntaxColor { r, g, b, a }
}

pub(crate) fn tone(color: SyntaxColor) -> Tone {
    [
        TEXT, KEYWORD, FUNCTION, STRING, COMMENT, NUMBER, PARAMETER, FIELD, ESCAPE, DECORATOR,
    ]
    .into_iter()
    .find(|tone| tone.dark.into_rgba8() == [color.r, color.g, color.b, color.a])
    .unwrap_or(TEXT)
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
