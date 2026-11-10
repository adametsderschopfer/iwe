//! Resolve protocol colors while preserving readable defaults in both themes.

use crate::ui::theme::{self, Tone};
use iced::{Color, Theme};

pub(crate) fn background(theme: &Theme) -> Color {
    theme::code_background(theme)
}

pub(crate) fn color(color: vt100::Color, default: Color, theme: &Theme) -> Color {
    const ANSI: [Tone; 16] = [
        Tone::new(0x303030, 0x242424),
        Tone::new(0xec9191, 0xa72b2d),
        Tone::new(0x81c99c, 0x28663f),
        Tone::new(0xdec287, 0x7b5b0b),
        Tone::new(0x8bb4eb, 0x245da0),
        Tone::new(0xc09de3, 0x7650a0),
        Tone::new(0x78c7c2, 0x16706b),
        Tone::new(0xd8d8d8, 0x55585e),
        Tone::new(0x949494, 0x666666),
        Tone::new(0xf16c6c, 0xc03737),
        Tone::new(0x73c991, 0x2b7c46),
        Tone::new(0xf4cc52, 0x856619),
        Tone::new(0x75beff, 0x2475ac),
        Tone::new(0xdf91aa, 0xa13b65),
        Tone::new(0x53c1de, 0x12758e),
        Tone::new(0xffffff, 0x303030),
    ];
    match color {
        vt100::Color::Default => default,
        vt100::Color::Rgb(r, g, b) => Color::from_rgb8(r, g, b),
        vt100::Color::Idx(index @ 0..=15) => ANSI[usize::from(index)].resolve(theme),
        vt100::Color::Idx(index @ 16..=231) => {
            let index = index - 16;
            let component = |value| if value == 0 { 0 } else { 55 + value * 40 };
            Color::from_rgb8(
                component(index / 36),
                component((index / 6) % 6),
                component(index % 6),
            )
        }
        vt100::Color::Idx(index) => {
            let value = 8 + (index - 232) * 10;
            Color::from_rgb8(value, value, value)
        }
    }
}

pub(super) const CELL_WIDTH: f32 = 0.6;
pub(super) const LINE_HEIGHT: f32 = 1.4;
pub(super) const PADDING: f32 = 12.;
pub(super) const HEADER_HEIGHT: f32 = 32.;
pub(super) const TAB_BAR_HEIGHT: f32 = 36.;
pub(super) const TAB_GAP: f32 = 6.;
pub(super) const TAB_CONTENT_GAP: f32 = 6.;
