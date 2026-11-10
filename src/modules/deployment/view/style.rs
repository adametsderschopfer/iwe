//! Reuse application colors with compact deployment control borders.

use crate::ui::theme;
use iced::{Border, Theme, widget::button};

pub(super) fn secondary(theme: &Theme, status: button::Status) -> button::Style {
    let disabled = status == button::Status::Disabled;
    button::Style {
        background: Some(theme::select(theme, iced::widget::pick_list::Status::Active).background),
        text_color: if disabled {
            theme::FAINT.resolve(theme)
        } else {
            theme::TEXT.resolve(theme)
        },
        border: Border {
            color: theme::LINE.resolve(theme),
            width: 1.,
            radius: 4.into(),
        },
        ..Default::default()
    }
}

pub(super) fn primary(theme: &Theme, status: button::Status) -> button::Style {
    if status == button::Status::Disabled {
        return secondary(theme, status);
    }
    button::Style {
        background: Some(theme::BLUE.resolve(theme).into()),
        text_color: theme::code_background(theme),
        ..secondary(theme, status)
    }
}
