//! Keep native glass visible and provide an opaque gray fallback.

use iced::{Color, theme};

pub(crate) const GREETING_SIZE: u32 = 32;

pub(crate) fn application_style(glass_ready: bool) -> theme::Style {
    theme::Style {
        background_color: if glass_ready {
            Color::TRANSPARENT
        } else {
            Color::from_rgb8(24, 24, 24)
        },
        text_color: Color::from_rgb8(242, 242, 242),
    }
}
