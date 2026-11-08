//! Use opaque charcoal surfaces above the native window material.

pub(crate) mod syntax;

pub(crate) const GIT_ADDED: iced::Color = iced::Color::from_rgb8(115, 201, 145);
pub(crate) const GIT_MODIFIED: iced::Color = iced::Color::from_rgb8(226, 192, 141);
pub(crate) const DIFF_MODIFIED: iced::Color = iced::Color::from_rgb8(117, 190, 255);
pub(crate) const GIT_REMOVED: iced::Color = iced::Color::from_rgb8(241, 108, 108);

pub(crate) fn file_status(status: crate::domain::changes::FileStatus) -> iced::Color {
    use crate::domain::changes::FileStatus;
    match status {
        FileStatus::Untracked | FileStatus::Added | FileStatus::Renamed => GIT_ADDED,
        FileStatus::Modified => GIT_MODIFIED,
        FileStatus::Deleted | FileStatus::Conflict => GIT_REMOVED,
    }
}

pub(crate) fn line_change(change: crate::ui::code::LineChange) -> iced::Color {
    use crate::ui::code::LineChange;
    match change {
        LineChange::Added => GIT_ADDED,
        LineChange::Modified => DIFF_MODIFIED,
        LineChange::Removed => GIT_REMOVED,
    }
}

pub(crate) fn diff_row(
    change: Option<crate::ui::code::LineChange>,
) -> impl Fn(&iced::Theme) -> iced::widget::container::Style {
    move |_| iced::widget::container::Style {
        background: change.map(|change| {
            iced::Color {
                a: 0.11,
                ..line_change(change)
            }
            .into()
        }),
        ..Default::default()
    }
}

use iced::{
    Border, Color, Font, Theme, font, theme,
    widget::{button, container, pane_grid, scrollable, slider, text_input, toggler},
};

const BACKGROUND: Color = Color::from_rgb8(33, 33, 33);
const SIDEBAR: Color = Color::from_rgb8(24, 24, 24);
const ELEVATED: Color = Color::from_rgb8(47, 47, 47);

pub(crate) const SIDEBAR_WIDTH: f32 = 264.0;
pub(crate) const SIDEBAR_TOP_INSET: f32 = 32.0;
pub(crate) const FILE_TAB_WIDTH: f32 = 200.0;
pub(crate) const FILE_TREE_ROW_HEIGHT: f32 = 24.0;
pub(crate) const FILE_TREE_TEXT_SIZE: f32 = 14.0;
pub(crate) const FILE_TREE_INDENT: f32 = 20.0;
pub(crate) const TEXT: Color = Color::from_rgb8(236, 236, 236);
pub(crate) const MUTED: Color = Color::from_rgb8(180, 180, 180);
pub(crate) const FAINT: Color = Color::from_rgb8(148, 148, 148);
pub(crate) const LINE: Color = Color::from_rgb8(55, 55, 55);
pub(crate) const GREEN: Color = Color::from_rgb8(129, 201, 156);
pub(crate) const BLUE: Color = Color::from_rgb8(139, 180, 235);
pub(crate) const RED: Color = Color::from_rgb8(236, 145, 145);
pub(crate) const YELLOW: Color = Color::from_rgb8(222, 194, 135);

pub(crate) fn font() -> Font {
    if cfg!(target_os = "macos") {
        Font::with_name("Helvetica Neue")
    } else if cfg!(target_os = "windows") {
        Font::with_name("Segoe UI")
    } else {
        Font::DEFAULT
    }
}

pub(crate) fn semibold() -> Font {
    Font {
        weight: font::Weight::Semibold,
        ..font()
    }
}

pub(crate) fn toggle(theme: &Theme, status: toggler::Status) -> toggler::Style {
    let active = match status {
        toggler::Status::Active { is_toggled }
        | toggler::Status::Hovered { is_toggled }
        | toggler::Status::Disabled { is_toggled } => is_toggled,
    };
    let mut style = toggler::default(theme, status);
    style.background = if active { TEXT } else { LINE }.into();
    style.foreground = if active {
        Color::from_rgb8(32, 32, 32)
    } else {
        MUTED
    }
    .into();
    style
}

pub(crate) fn font_slider(theme: &Theme, status: slider::Status) -> slider::Style {
    let mut style = slider::default(theme, status);
    style.rail.backgrounds = (TEXT.into(), LINE.into());
    style.handle.background = TEXT.into();
    style
}

pub(crate) fn surface(_: &Theme) -> container::Style {
    container::Style {
        background: Some(BACKGROUND.into()),
        text_color: Some(TEXT),
        ..Default::default()
    }
}

pub(crate) fn sidebar(_: &Theme) -> container::Style {
    container::Style {
        background: Some(SIDEBAR.into()),
        text_color: Some(TEXT),
        ..Default::default()
    }
}

pub(crate) fn panel(theme: &Theme) -> container::Style {
    surface(theme)
}

pub(crate) fn file_tab(active: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(if active { BACKGROUND } else { SIDEBAR }.into()),
        text_color: Some(if active { TEXT } else { MUTED }),
        ..Default::default()
    }
}

pub(crate) fn file_tab_button(_: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: matches!(status, button::Status::Hovered | button::Status::Pressed)
            .then_some(ELEVATED.into()),
        text_color: TEXT,
        ..Default::default()
    }
}

pub(crate) fn file_tab_indicator(active: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(if active { BLUE } else { LINE }.into()),
        ..Default::default()
    }
}

pub(crate) fn separator(_: &Theme) -> container::Style {
    container::Style {
        background: Some(LINE.into()),
        ..Default::default()
    }
}

pub(crate) fn source_progress(_: &Theme) -> iced::widget::progress_bar::Style {
    iced::widget::progress_bar::Style {
        background: Color::TRANSPARENT.into(),
        bar: BLUE.into(),
        border: Border::default(),
    }
}

pub(crate) fn panes(_: &Theme) -> pane_grid::Style {
    pane_grid::Style {
        hovered_region: pane_grid::Highlight {
            background: Color::from_rgba8(236, 236, 236, 0.06).into(),
            border: Border {
                color: FAINT,
                width: 1.0,
                ..Default::default()
            },
        },
        hovered_split: pane_grid::Line {
            color: FAINT,
            width: 1.0,
        },
        picked_split: pane_grid::Line {
            color: MUTED,
            width: 2.0,
        },
    }
}

pub(crate) fn tooltip(_: &Theme) -> container::Style {
    container::Style {
        background: Some(ELEVATED.into()),
        text_color: Some(TEXT),
        border: Border {
            color: LINE,
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    }
}

pub(crate) fn badge(_: &Theme) -> container::Style {
    container::Style {
        background: Some(ELEVATED.into()),
        border: Border {
            radius: 999.0.into(),
            ..Default::default()
        },
        text_color: Some(MUTED),
        ..Default::default()
    }
}

pub(crate) fn dialog(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::from_rgb8(40, 40, 40).into()),
        border: Border {
            color: LINE,
            width: 1.0,
            radius: 24.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
            offset: iced::Vector::new(0.0, 12.0),
            blur_radius: 30.0,
        },
        ..surface(theme)
    }
}

pub(crate) fn action(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: (selected || hovered).then_some(
                Color::from_rgb8(
                    if selected { 48 } else { 42 },
                    if selected { 48 } else { 42 },
                    if selected { 48 } else { 42 },
                )
                .into(),
            ),
            text_color: if matches!(status, button::Status::Disabled) {
                FAINT
            } else {
                TEXT
            },
            border: Border {
                radius: 10.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

pub(crate) fn fold_row(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = action(false)(theme, status);
    style
        .background
        .get_or_insert(Color::from_rgb8(38, 38, 38).into());
    style.border.radius = 2.0.into();
    style.text_color = MUTED;
    style
}

pub(crate) fn icon_action(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let mut style = action(selected)(theme, status);
        style.border.radius = 999.0.into();
        style
    }
}

pub(crate) fn tree_item(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let mut style = action(selected)(theme, status);
        style.border.radius = 3.0.into();
        if selected {
            style.background = Some(Color::from_rgb8(65, 68, 72).into());
        }
        style
    }
}

pub(crate) fn primary(_: &Theme, status: button::Status) -> button::Style {
    let disabled = matches!(status, button::Status::Disabled);
    let shade = if disabled {
        60
    } else if matches!(status, button::Status::Hovered) {
        210
    } else {
        239
    };
    button::Style {
        background: Some(Color::from_rgb8(shade, shade, shade).into()),
        text_color: if disabled {
            MUTED
        } else {
            Color::from_rgb8(24, 24, 24)
        },
        border: Border {
            radius: 999.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(crate) fn input(_: &Theme, status: text_input::Status) -> text_input::Style {
    let focused = matches!(status, text_input::Status::Focused { .. });
    text_input::Style {
        background: ELEVATED.into(),
        border: Border {
            color: if focused {
                MUTED
            } else {
                Color::from_rgb8(72, 72, 72)
            },
            width: 1.0,
            radius: 12.0.into(),
        },
        icon: MUTED,
        placeholder: FAINT,
        value: if matches!(status, text_input::Status::Disabled) {
            FAINT
        } else {
            TEXT
        },
        selection: Color::from_rgb8(83, 83, 83),
    }
}

pub(crate) fn secondary(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = action(false)(theme, status);
    style.border = Border {
        color: Color::from_rgb8(80, 80, 80),
        width: 1.0,
        radius: 999.0.into(),
    };
    style
}

pub(crate) fn scroll(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let mut style = scrollable::default(theme, status);
    let (horizontal_disabled, vertical_disabled) = match status {
        scrollable::Status::Active {
            is_horizontal_scrollbar_disabled,
            is_vertical_scrollbar_disabled,
        }
        | scrollable::Status::Hovered {
            is_horizontal_scrollbar_disabled,
            is_vertical_scrollbar_disabled,
            ..
        }
        | scrollable::Status::Dragged {
            is_horizontal_scrollbar_disabled,
            is_vertical_scrollbar_disabled,
            ..
        } => (
            is_horizontal_scrollbar_disabled,
            is_vertical_scrollbar_disabled,
        ),
    };
    let shade = if matches!(status, scrollable::Status::Active { .. }) {
        64
    } else {
        100
    };
    for (rail, disabled) in [
        (&mut style.horizontal_rail, horizontal_disabled),
        (&mut style.vertical_rail, vertical_disabled),
    ] {
        rail.background = None;
        rail.border = Border::default();
        rail.scroller.background = if disabled {
            Color::TRANSPARENT
        } else {
            Color::from_rgb8(shade, shade, shade)
        }
        .into();
        rail.scroller.border = Border {
            radius: 8.0.into(),
            ..Default::default()
        };
    }
    style.gap = None;
    style.auto_scroll.background = ELEVATED.into();
    style.auto_scroll.border = Border {
        color: LINE,
        width: 1.0,
        radius: 999.0.into(),
    };
    style.auto_scroll.icon = TEXT;
    style
}

pub(crate) fn application_style(glass_ready: bool) -> theme::Style {
    theme::Style {
        background_color: if glass_ready {
            Color::TRANSPARENT
        } else {
            BACKGROUND
        },
        text_color: Color::from_rgb8(242, 242, 242),
    }
}
