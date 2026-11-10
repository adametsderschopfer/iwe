//! List open source tabs when the tab strip does not fit the available width.

use crate::{
    app::{App, Message},
    ui::{context_menu, ellipsis, file_icons, theme, tooltip},
};
use iced::{
    Alignment, Element, Length, Size,
    widget::{Column, button, container, row, scrollable, text},
};
use std::path::Path;

pub(super) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(point) = app.tabs.menu else {
        return base;
    };
    let count = app
        .tabs
        .files
        .iter()
        .filter(|preview| !preview.title.is_empty())
        .count();
    if count == 0 {
        return base;
    }
    let height = (count as f32 * theme::TAB_MENU_ROW_HEIGHT).min(theme::TAB_MENU_MAX_HEIGHT);
    context_menu::overlay(
        base,
        point,
        Size::new(theme::TAB_MENU_WIDTH, height + 16.),
        move || {
            let mut files = Column::new();
            for preview in app
                .tabs
                .files
                .iter()
                .filter(|preview| !preview.title.is_empty())
            {
                let filename = Path::new(&preview.title)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or(&preview.title);
                let label = if let Some(revision) = &preview.revision {
                    format!(
                        "{filename} · {}",
                        revision.chars().take(7).collect::<String>()
                    )
                } else {
                    filename.to_owned()
                };
                let hint = if let Some(revision) = &preview.revision {
                    format!("{}\n{revision}", preview.title)
                } else {
                    preview.title.clone()
                };
                files = files.push(tooltip::view(
                    container(
                        button(
                            row![
                                file_icons::file(Path::new(&preview.title)),
                                ellipsis::text(label, 13.)
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center),
                        )
                        .padding([8, 10])
                        .width(Length::Fill)
                        .height(theme::TAB_MENU_ROW_HEIGHT)
                        .style(theme::action(preview.id == app.preview().id))
                        .on_press(Message::SelectTab(preview.id)),
                    )
                    .id(format!("open-file-tab-{}", preview.id)),
                    text(hint).size(12),
                    tooltip::Position::Left,
                ));
            }
            scrollable(files)
                .spacing(4)
                .height(height)
                .width(Length::Fill)
                .style(theme::scroll)
                .into()
        },
        Message::CloseTabMenu,
    )
}
