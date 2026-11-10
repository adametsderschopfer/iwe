//! Resolve semantic colors at render time without global appearance state.

use iced::{Color, Theme, widget::text};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Tone {
    pub(crate) dark: Color,
    pub(crate) light: Color,
}

impl Tone {
    pub(crate) const fn new(dark: u32, light: u32) -> Self {
        const fn rgb(value: u32) -> Color {
            Color::from_rgb8((value >> 16) as u8, (value >> 8) as u8, value as u8)
        }
        Self {
            dark: rgb(dark),
            light: rgb(light),
        }
    }

    pub(crate) fn resolve(self, theme: &Theme) -> Color {
        if theme.extended_palette().is_dark {
            self.dark
        } else {
            self.light
        }
    }

    pub(crate) fn text(self) -> impl Fn(&Theme) -> text::Style {
        move |theme| text::Style {
            color: Some(self.resolve(theme)),
        }
    }
}

pub(super) const BACKGROUND: Tone = Tone::new(0x212121, 0xfafafa);
pub(super) const CODE_BACKGROUND: Tone = Tone::new(0x1b1b1b, 0xffffff);
pub(super) const SIDEBAR: Tone = Tone::new(0x181818, 0xf0f0f0);
pub(super) const ELEVATED: Tone = Tone::new(0x2f2f2f, 0xe8e8e8);
pub(super) const HOVER: Tone = Tone::new(0x2a2a2a, 0xe5e5e5);
pub(super) const SELECTED: Tone = Tone::new(0x303030, 0xdedede);
pub(super) const TREE_SELECTED: Tone = Tone::new(0x414448, 0xd8e5f4);
pub(crate) const CODE_SELECTION: Tone = Tone::new(0x36516b, 0xc5def5);
pub(super) const DIALOG: Tone = Tone::new(0x282828, 0xf5f5f5);
pub(super) const FOLD: Tone = Tone::new(0x262626, 0xf1f1f1);
pub(super) const INPUT_BORDER: Tone = Tone::new(0x484848, 0xbcbcbc);
pub(super) const INPUT_SELECTION: Tone = Tone::new(0x535353, 0xb9d3f0);
pub(super) const OUTLINE: Tone = Tone::new(0x505050, 0xaaaaaa);
pub(super) const PRIMARY: Tone = Tone::new(0xefefef, 0x2c2c2c);
pub(super) const PRIMARY_HOVER: Tone = Tone::new(0xd2d2d2, 0x444444);
pub(super) const PRIMARY_DISABLED: Tone = Tone::new(0x3c3c3c, 0xe0e0e0);
pub(super) const PRIMARY_TEXT: Tone = Tone::new(0x181818, 0xfafafa);
pub(super) const THUMB: Tone = Tone::new(0x404040, 0xb8b8b8);
pub(super) const THUMB_HOVER: Tone = Tone::new(0x646464, 0x929292);

pub(crate) const TEXT: Tone = Tone::new(0xececec, 0x242424);
pub(crate) const MUTED: Tone = Tone::new(0xb4b4b4, 0x55585e);
pub(crate) const FAINT: Tone = Tone::new(0x949494, 0x666666);
pub(crate) const LINE: Tone = Tone::new(0x373737, 0xd4d4d4);
pub(crate) const GREEN: Tone = Tone::new(0x81c99c, 0x28663f);
pub(crate) const UPDATE_BLUE: Tone = Tone::new(0x3478f6, 0x245da0);
pub(crate) const BLUE: Tone = Tone::new(0x8bb4eb, 0x245da0);
pub(crate) const RED: Tone = Tone::new(0xec9191, 0xa72b2d);
pub(crate) const YELLOW: Tone = Tone::new(0xdec287, 0x7b5b0b);
pub(crate) const PURPLE: Tone = Tone::new(0xc09de3, 0x7650a0);
pub(crate) const PINK: Tone = Tone::new(0xdf91aa, 0xa13b65);
pub(crate) const PIPELINE_GREEN: Tone = Tone::new(0x168447, 0x108548);
pub(crate) const PIPELINE_RED: Tone = Tone::new(0xc94149, 0xba3038);
pub(crate) const PIPELINE_BLUE: Tone = Tone::new(0x326bc4, 0x2466c5);
pub(crate) const PIPELINE_AMBER: Tone = Tone::new(0xa7680b, 0x95600a);
pub(crate) const PIPELINE_GRAY: Tone = Tone::new(0x666b74, 0x666b74);
pub(crate) const PIPELINE_GLYPH: Tone = Tone::new(0xffffff, 0xffffff);
pub(crate) const GIT_ADDED: Tone = Tone::new(0x73c991, 0x2b7c46);
pub(crate) const DIFF_MODIFIED: Tone = Tone::new(0x75beff, 0x2475ac);
pub(crate) const GIT_REMOVED: Tone = Tone::new(0xf16c6c, 0xc03737);
pub(super) const UNIFIED_MODIFIED: Tone = Tone::new(0x899bb5, 0x5e6f87);
pub(super) const FILE_ADDED: Tone = Tone::new(0xb6d87a, 0x487418);
pub(super) const FILE_MODIFIED: Tone = Tone::new(0x78c7c2, 0x16706b);

pub(crate) const ICON_GRAY: Tone = Tone::new(0x9fa4aa, 0x62676d);
pub(crate) const ICON_BLUE: Tone = Tone::new(0x5b9ddf, 0x3275b9);
pub(crate) const ICON_YELLOW: Tone = Tone::new(0xf4cc52, 0xa07a09);
pub(crate) const ICON_CYAN: Tone = Tone::new(0x53c1de, 0x12758e);
pub(crate) const ICON_ORANGE: Tone = Tone::new(0xeb7a46, 0xbb4f20);
pub(crate) const ICON_PINK: Tone = Tone::new(0xcf7ead, 0xa2467e);
