//! Share neutral dark and light surfaces above the native window material.

mod palette;
pub(crate) mod syntax;
pub(crate) use palette::*;

use iced::{
    Border, Color, Font, Theme, font, theme,
    widget::{
        button, container, overlay::menu, pane_grid, pick_list, scrollable, slider, text_input,
        toggler,
    },
};

pub(crate) fn file_status(status: crate::domain::changes::FileStatus) -> Tone {
    use crate::domain::changes::FileStatus;
    match status {
        FileStatus::Untracked | FileStatus::Added => FILE_ADDED,
        FileStatus::Modified | FileStatus::Renamed => FILE_MODIFIED,
        FileStatus::Deleted | FileStatus::Conflict => GIT_REMOVED,
    }
}

pub(crate) fn repository_color(index: usize) -> Tone {
    const COLORS: [Tone; 6] = [BLUE, PURPLE, GREEN, YELLOW, PINK, ICON_CYAN];
    COLORS[index % COLORS.len()]
}

pub(crate) fn line_change(change: crate::ui::code::LineChange) -> Tone {
    use crate::ui::code::LineChange;
    match change {
        LineChange::Added => GIT_ADDED,
        LineChange::Modified => DIFF_MODIFIED,
        LineChange::Removed => GIT_REMOVED,
    }
}

pub(crate) fn unified_color(change: crate::ui::code::LineChange) -> Tone {
    match change {
        crate::ui::code::LineChange::Modified => UNIFIED_MODIFIED,
        _ => line_change(change),
    }
}

pub(crate) fn change_fill(color: Tone, theme: &Theme, opacity: f32) -> Color {
    let opacity = if theme.extended_palette().is_dark {
        opacity
    } else {
        opacity * 0.4
    };
    color.resolve(theme).scale_alpha(opacity)
}

pub(crate) fn source_row(
    theme: &Theme,
    mut style: container::Style,
    highlighted: bool,
) -> container::Style {
    if highlighted {
        style.background = Some(CODE_SELECTION.resolve(theme).into());
    }
    style
}

pub(crate) fn unified_row(
    change: Option<crate::ui::code::LineChange>,
) -> impl Fn(&Theme) -> container::Style {
    move |theme| container::Style {
        background: change.map(|change| change_fill(unified_color(change), theme, 0.20).into()),
        ..Default::default()
    }
}

pub(crate) fn unified_gutter(
    change: Option<crate::ui::code::LineChange>,
) -> impl Fn(&Theme) -> container::Style {
    move |theme| container::Style {
        background: Some(
            change
                .map_or_else(
                    || SIDEBAR.resolve(theme),
                    |change| change_fill(unified_color(change), theme, 0.12),
                )
                .into(),
        ),
        ..Default::default()
    }
}

pub(crate) fn diff_row(
    change: Option<crate::ui::code::LineChange>,
) -> impl Fn(&Theme) -> container::Style {
    move |theme| container::Style {
        background: change.map(|change| change_fill(line_change(change), theme, 0.11).into()),
        ..Default::default()
    }
}

pub(crate) const COMMIT_GRAPH_PADDING: f32 = 12.;
pub(crate) const COMMIT_REPOSITORY_GRAPH_PADDING: f32 = 4.;
pub(crate) const COMMIT_REPOSITORY_MAX_WIDTH: f32 = 160.;

pub(crate) const SIDEBAR_WIDTH: f32 = 264.0;
pub(crate) const SIDEBAR_SCROLL_GAP: f32 = 6.0;
pub(crate) const SETTINGS_SIDEBAR_WIDTH: f32 = 220.0;
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(crate) const SETTINGS_HEADER_HEIGHT: f32 = 48.0;
pub(crate) const NOTIFICATION_WIDTH: f32 = 380.0;
pub(crate) const NOTIFICATION_INSET: f32 = 16.0;
pub(crate) const NOTIFICATION_GAP: f32 = 8.0;
pub(crate) const NOTIFICATION_SLIDE_DURATION: std::time::Duration =
    std::time::Duration::from_millis(200);
pub(crate) const NOTIFICATION_SETTLE_DURATION: std::time::Duration =
    std::time::Duration::from_millis(200);
pub(crate) const NOTIFICATION_PADDING: f32 = 14.0;
pub(crate) const FILE_PANEL_TOOL_SIZE: f32 = 40.0;
pub(crate) const FILE_PANEL_TOOL_INSET: f32 = 4.0;
pub(crate) const FILE_PANEL_HEADER_HEIGHT: f32 = FILE_PANEL_TOOL_SIZE + FILE_PANEL_TOOL_INSET * 2.0;
pub(crate) const FILE_TABS_HEIGHT: f32 = FILE_PANEL_HEADER_HEIGHT + 1.0;
pub(crate) const SOURCE_MIN_WIDTH: f32 = 320.0;
pub(crate) const FILE_PANEL_RESIZE_GRAB: f32 = 5.0;
pub(crate) const FILE_PANEL_CONTENT_INSET: f32 = 8.0;
#[cfg(target_os = "windows")]
pub(crate) const WINDOW_CONTROL_WIDTH: f32 = 46.0;
#[cfg(target_os = "windows")]
pub(crate) const WINDOW_CONTROL_HEIGHT: f32 = 40.0;
#[cfg(target_os = "windows")]
pub(crate) const WINDOW_RESIZE_GRIP: f32 = 5.0;
pub(crate) const COMMIT_DATE_WIDTH: f32 = 120.0;
pub(crate) const COMMIT_HASH_WIDTH: f32 = 64.0;
pub(crate) const HEADER_REPOSITORY_WIDTH: f32 = 220.0;
pub(crate) const HEADER_REPOSITORY_MENU_WIDTH: f32 = 360.0;
pub(crate) const HEADER_REPOSITORY_ROW_HEIGHT: f32 = 48.0;
pub(crate) const HEADER_REPOSITORY_MENU_HEIGHT: f32 = 336.0;
pub(crate) const CODE_SCROLLBAR_WIDTH: f32 = 20.0;
pub(crate) const CODE_SCROLLBAR_THUMB_WIDTH: f32 = 10.0;
pub(crate) const CODE_CHANGE_MARK_WIDTH: f32 = 4.0;
pub(crate) const CODE_CHANGE_MARK_MIN_HEIGHT: f32 = 3.0;
pub(crate) const UNIFIED_NUMBER_MIN_WIDTH: f32 = 32.0;
pub(crate) const UNIFIED_NUMBER_GAP: f32 = 6.0;
pub(crate) const UNIFIED_GUTTER_PADDING: f32 = 8.0;
pub(crate) const SOURCE_GUTTER_WIDTH: f32 = 91.0;
pub(crate) const SOURCE_MARKERS_WIDTH: f32 = 35.0;
pub(crate) const DIFF_GUTTER_WIDTH: f32 = 65.0;
pub(crate) const DIFF_MARKERS_WIDTH: f32 = 11.0;
pub(crate) const GIT_PANE_SPACING: f32 = 1.0;
pub(crate) const GIT_PANE_MIN_SIZE: f32 = 120.0;
pub(crate) const BRANCH_HEADER_INSET: f32 = 8.0;
pub(crate) const SIDEBAR_TOP_INSET: f32 = 32.0;
pub(crate) const FILE_TAB_MIN_WIDTH: f32 = 200.0;
pub(crate) const FILE_TAB_TEXT_SIZE: f32 = 12.0;
pub(crate) const FILE_TAB_NAME_LIMIT: usize = 35;
pub(crate) const FILE_TAB_ICON_WIDTH: f32 = 16.0;
pub(crate) const FILE_TAB_LABEL_GAP: f32 = 6.0;
pub(crate) const FILE_TAB_PADDING: f32 = 8.0;
pub(crate) const FILE_TAB_CLOSE_WIDTH: f32 = 28.0;
pub(crate) const FILE_TAB_END_GAP: f32 = 4.0;
pub(crate) const FILE_TAB_TOOL_SIZE: f32 = 36.0;
pub(crate) const FILE_TAB_TOOL_GAP: f32 = 6.0;
pub(crate) const TAB_MENU_WIDTH: f32 = 380.0;
pub(crate) const TAB_MENU_ROW_HEIGHT: f32 = 36.0;
pub(crate) const TAB_MENU_MAX_HEIGHT: f32 = 324.0;
pub(crate) const FILE_TREE_ROW_HEIGHT: f32 = 24.0;
pub(crate) const FILE_TREE_TEXT_SIZE: f32 = 14.0;
pub(crate) const FILE_TREE_INDENT: f32 = 20.0;
pub(crate) const SPACE_PROJECT_HEIGHT: f32 = 56.0;
pub(crate) const PROJECT_STATUS_ICON_SIZE: f32 = 16.0;
pub(crate) const SPACE_PROJECT_GAP: f32 = 4.0;
pub(crate) const PIPELINE_ROW_HEIGHT: f32 = 72.0;
pub(crate) const PIPELINE_BADGE_SIZE: f32 = 30.0;
pub(crate) const PIPELINE_CONNECTOR_WIDTH: f32 = 8.0;
pub(crate) const PIPELINE_POPUP_WIDTH: f32 = 380.0;
pub(crate) const PIPELINE_POPUP_JOBS_HEIGHT: f32 = 240.0;
pub(crate) const PIPELINE_JOB_HEIGHT: f32 = 40.0;

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

pub(crate) fn code_font() -> Font {
    if cfg!(target_os = "macos") {
        Font::with_name("Menlo")
    } else if cfg!(target_os = "windows") {
        Font::with_name("Consolas")
    } else {
        Font::MONOSPACE
    }
}

pub(crate) fn toggle(theme: &Theme, status: toggler::Status) -> toggler::Style {
    let active = match status {
        toggler::Status::Active { is_toggled }
        | toggler::Status::Hovered { is_toggled }
        | toggler::Status::Disabled { is_toggled } => is_toggled,
    };
    let mut style = toggler::default(theme, status);
    style.background = if active {
        TEXT.resolve(theme)
    } else {
        LINE.resolve(theme)
    }
    .into();
    style.foreground = if active {
        PRIMARY_TEXT.resolve(theme)
    } else {
        MUTED.resolve(theme)
    }
    .into();
    style
}

pub(crate) fn font_slider(theme: &Theme, status: slider::Status) -> slider::Style {
    let mut style = slider::default(theme, status);
    style.rail.backgrounds = (TEXT.resolve(theme).into(), LINE.resolve(theme).into());
    style.handle.background = TEXT.resolve(theme).into();
    style
}

pub(crate) fn select(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    pick_list::Style {
        text_color: TEXT.resolve(theme),
        placeholder_color: FAINT.resolve(theme),
        handle_color: MUTED.resolve(theme),
        background: ELEVATED.resolve(theme).into(),
        border: Border {
            color: if matches!(status, pick_list::Status::Active) {
                LINE.resolve(theme)
            } else {
                MUTED.resolve(theme)
            },
            width: 1.,
            radius: 10.0.into(),
        },
    }
}

pub(crate) fn select_menu(theme: &Theme) -> menu::Style {
    menu::Style {
        background: ELEVATED.resolve(theme).into(),
        border: Border {
            color: LINE.resolve(theme),
            width: 1.,
            radius: 10.0.into(),
        },
        text_color: TEXT.resolve(theme),
        selected_text_color: TEXT.resolve(theme),
        selected_background: LINE.resolve(theme).into(),
        shadow: Default::default(),
    }
}

pub(crate) fn surface(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(BACKGROUND.resolve(theme).into()),
        text_color: Some(TEXT.resolve(theme)),
        ..Default::default()
    }
}

pub(crate) fn code_surface(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(code_background(theme).into()),
        ..surface(theme)
    }
}

pub(crate) fn code_background(theme: &Theme) -> Color {
    CODE_BACKGROUND.resolve(theme)
}

pub(crate) fn sidebar(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(SIDEBAR.resolve(theme).into()),
        text_color: Some(TEXT.resolve(theme)),
        ..Default::default()
    }
}

pub(crate) fn panel(theme: &Theme) -> container::Style {
    surface(theme)
}

pub(crate) fn file_tab(active: bool) -> impl Fn(&Theme) -> container::Style {
    move |theme| container::Style {
        background: Some(
            if active {
                BACKGROUND.resolve(theme)
            } else {
                SIDEBAR.resolve(theme)
            }
            .into(),
        ),
        text_color: Some(if active {
            TEXT.resolve(theme)
        } else {
            MUTED.resolve(theme)
        }),
        ..Default::default()
    }
}

pub(crate) fn file_tab_button(theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: matches!(status, button::Status::Hovered | button::Status::Pressed)
            .then_some(ELEVATED.resolve(theme).into()),
        text_color: TEXT.resolve(theme),
        ..Default::default()
    }
}

pub(crate) fn file_tab_indicator(active: bool) -> impl Fn(&Theme) -> container::Style {
    move |theme| container::Style {
        background: Some(
            if active {
                BLUE.resolve(theme)
            } else {
                LINE.resolve(theme)
            }
            .into(),
        ),
        ..Default::default()
    }
}

pub(crate) fn separator(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(LINE.resolve(theme).into()),
        ..Default::default()
    }
}

pub(crate) fn notification_progress(
    theme: &Theme,
    tone: Tone,
) -> iced::widget::progress_bar::Style {
    iced::widget::progress_bar::Style {
        background: HOVER.resolve(theme).into(),
        bar: tone.resolve(theme).into(),
        border: Border {
            radius: 2.0.into(),
            ..Default::default()
        },
    }
}

pub(crate) fn source_progress(theme: &Theme) -> iced::widget::progress_bar::Style {
    iced::widget::progress_bar::Style {
        background: Color::TRANSPARENT.into(),
        bar: BLUE.resolve(theme).into(),
        border: Border::default(),
    }
}

pub(crate) fn panes(theme: &Theme) -> pane_grid::Style {
    pane_grid::Style {
        hovered_region: pane_grid::Highlight {
            background: TEXT.resolve(theme).scale_alpha(0.06).into(),
            border: Border {
                color: FAINT.resolve(theme),
                width: 1.0,
                ..Default::default()
            },
        },
        hovered_split: pane_grid::Line {
            color: FAINT.resolve(theme),
            width: 1.0,
        },
        picked_split: pane_grid::Line {
            color: MUTED.resolve(theme),
            width: 2.0,
        },
    }
}

pub(crate) fn tooltip(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(ELEVATED.resolve(theme).into()),
        text_color: Some(TEXT.resolve(theme)),
        border: Border {
            color: LINE.resolve(theme),
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    }
}

pub(crate) fn notification(theme: &Theme) -> container::Style {
    container::Style {
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.18),
            offset: iced::Vector::new(0., 3.),
            blur_radius: 12.,
        },
        ..tooltip(theme)
    }
}

pub(crate) fn badge(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(ELEVATED.resolve(theme).into()),
        border: Border {
            radius: 999.0.into(),
            ..Default::default()
        },
        text_color: Some(MUTED.resolve(theme)),
        ..Default::default()
    }
}

pub(crate) fn branch_badge(theme: &Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(
            if hovered { SELECTED } else { ELEVATED }
                .resolve(theme)
                .into(),
        ),
        text_color: if hovered { TEXT } else { MUTED }.resolve(theme),
        border: Border {
            radius: 999.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(crate) fn branch_attention(active: bool) -> impl Fn(&Theme) -> container::Style {
    move |theme| container::Style {
        border: Border {
            width: if active { 2.0 } else { 0.0 },
            color: BLUE.resolve(theme),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(crate) fn dialog(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(DIALOG.resolve(theme).into()),
        border: Border {
            color: LINE.resolve(theme),
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
    move |theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: (selected || hovered).then_some(
                (if selected { SELECTED } else { HOVER })
                    .resolve(theme)
                    .into(),
            ),
            text_color: if matches!(status, button::Status::Disabled) {
                FAINT.resolve(theme)
            } else {
                TEXT.resolve(theme)
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
    style.background.get_or_insert(FOLD.resolve(theme).into());
    style.border.radius = 2.0.into();
    style.text_color = MUTED.resolve(theme);
    style
}

pub(crate) fn icon_action(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let mut style = action(selected)(theme, status);
        style.border.radius = 999.0.into();
        style
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn window_control(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = action(false)(theme, status);
    style.border.radius = 0.0.into();
    style
}

pub(crate) fn file_tree_text_width(name: &str) -> f32 {
    name.chars()
        .map(|c| {
            if matches!(c, 'W' | 'M' | '@') || !c.is_ascii() {
                12.
            } else if c == 'm' {
                10.5
            } else if c == 'w' {
                9.
            } else if c.is_ascii_uppercase() {
                8.5
            } else if matches!(c, '.' | '-' | 'i' | 'l' | 'r') {
                4.5
            } else {
                6.8
            }
        })
        .sum::<f32>()
        * (FILE_TREE_TEXT_SIZE / 12.)
}

pub(crate) fn tree_item(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let mut style = action(selected)(theme, status);
        style.border.radius = 3.0.into();
        if selected {
            style.background = Some(TREE_SELECTED.resolve(theme).into());
        }
        style
    }
}

pub(crate) fn primary(theme: &Theme, status: button::Status) -> button::Style {
    let disabled = matches!(status, button::Status::Disabled);
    let background = if disabled {
        PRIMARY_DISABLED
    } else if matches!(status, button::Status::Hovered) {
        PRIMARY_HOVER
    } else {
        PRIMARY
    }
    .resolve(theme);
    button::Style {
        background: Some(background.into()),
        text_color: if disabled {
            MUTED.resolve(theme)
        } else {
            PRIMARY_TEXT.resolve(theme)
        },
        border: Border {
            radius: 999.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(crate) fn input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let focused = matches!(status, text_input::Status::Focused { .. });
    text_input::Style {
        background: ELEVATED.resolve(theme).into(),
        border: Border {
            color: if focused {
                MUTED.resolve(theme)
            } else {
                INPUT_BORDER.resolve(theme)
            },
            width: 1.0,
            radius: 12.0.into(),
        },
        icon: MUTED.resolve(theme),
        placeholder: FAINT.resolve(theme),
        value: if matches!(status, text_input::Status::Disabled) {
            FAINT.resolve(theme)
        } else {
            TEXT.resolve(theme)
        },
        selection: INPUT_SELECTION.resolve(theme),
    }
}

pub(crate) fn secondary(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = action(false)(theme, status);
    style.border = Border {
        color: OUTLINE.resolve(theme),
        width: 1.0,
        radius: 999.0.into(),
    };
    style
}

pub(crate) fn code_scroll(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let mut style = scroll(theme, status);
    style.vertical_rail.background = Some(SIDEBAR.resolve(theme).into());
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
    let thumb = if matches!(status, scrollable::Status::Active { .. }) {
        THUMB
    } else {
        THUMB_HOVER
    }
    .resolve(theme);
    for (rail, disabled) in [
        (&mut style.horizontal_rail, horizontal_disabled),
        (&mut style.vertical_rail, vertical_disabled),
    ] {
        rail.background = None;
        rail.border = Border::default();
        rail.scroller.background = if disabled { Color::TRANSPARENT } else { thumb }.into();
        rail.scroller.border = Border {
            radius: 8.0.into(),
            ..Default::default()
        };
    }
    style.gap = None;
    style.auto_scroll.background = ELEVATED.resolve(theme).into();
    style.auto_scroll.border = Border {
        color: LINE.resolve(theme),
        width: 1.0,
        radius: 999.0.into(),
    };
    style.auto_scroll.icon = TEXT.resolve(theme);
    style
}

pub(crate) fn application_style(glass_ready: bool, theme: &Theme) -> theme::Style {
    theme::Style {
        background_color: if glass_ready {
            Color::TRANSPARENT
        } else {
            BACKGROUND.resolve(theme)
        },
        text_color: TEXT.resolve(theme),
    }
}

#[cfg(test)]
mod tests;

pub(crate) fn success_tooltip(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(GREEN.resolve(theme).into()),
        text_color: Some(if theme.extended_palette().is_dark {
            Color::BLACK
        } else {
            Color::WHITE
        }),
        border: Border {
            color: GREEN.resolve(theme),
            ..tooltip(theme).border
        },
        ..tooltip(theme)
    }
}
