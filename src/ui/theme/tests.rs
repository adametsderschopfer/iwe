use super::*;

fn luminance(color: Color) -> f32 {
    let linear = |channel: f32| {
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    linear(color.r) * 0.2126 + linear(color.g) * 0.7152 + linear(color.b) * 0.0722
}

fn contrast(foreground: Color, background: Color) -> f32 {
    let a = luminance(foreground);
    let b = luminance(background);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[test]
fn light_text_remains_readable_on_neutral_surfaces() {
    for background in [BACKGROUND, CODE_BACKGROUND, SIDEBAR, ELEVATED] {
        for foreground in [
            TEXT,
            MUTED,
            FAINT,
            BLUE,
            GREEN,
            RED,
            YELLOW,
            FILE_ADDED,
            FILE_MODIFIED,
        ] {
            assert!(
                contrast(foreground.light, background.light) >= 4.5,
                "{foreground:?} on {background:?}"
            );
        }
    }
}

#[test]
fn syntax_colors_remain_readable_in_light_diff_rows() {
    for change in [
        crate::ui::code::LineChange::Added,
        crate::ui::code::LineChange::Modified,
        crate::ui::code::LineChange::Removed,
    ] {
        let color = unified_color(change).resolve(&Theme::Light);
        let iced::Background::Color(row) =
            unified_row(Some(change))(&Theme::Light).background.unwrap()
        else {
            panic!("Solid diff row");
        };
        let highlight = change_fill(unified_color(change), &Theme::Light, 0.30);
        let opacity = row.a + (1. - row.a) * highlight.a;
        let background = Color {
            r: color.r * opacity + CODE_BACKGROUND.light.r * (1. - opacity),
            g: color.g * opacity + CODE_BACKGROUND.light.g * (1. - opacity),
            b: color.b * opacity + CODE_BACKGROUND.light.b * (1. - opacity),
            a: 1.,
        };
        for foreground in [
            syntax::TEXT,
            syntax::KEYWORD,
            syntax::FUNCTION,
            syntax::STRING,
            syntax::COMMENT,
            syntax::NUMBER,
            syntax::PARAMETER,
            syntax::FIELD,
            syntax::ESCAPE,
            syntax::DECORATOR,
        ] {
            assert!(
                contrast(foreground.light, background) >= 4.5,
                "{change:?}: {foreground:?} on {background:?}"
            );
        }
    }
}
