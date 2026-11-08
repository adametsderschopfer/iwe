use super::{
    modal,
    workspace::{short, short_path},
};
use crate::{
    app::{App, Message, spaces},
    i18n::Key,
    ui::{
        icons::{Icon, icon},
        theme,
    },
};
use iced::{
    Alignment, Element, Length,
    widget::{
        Column, button, column, container, row, scrollable, space, text, text_input, tooltip,
    },
};

pub(super) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(draft) = &app.space_draft else {
        return base;
    };
    let name = column![
        text(app.i18n.text(Key::SpaceName))
            .size(13)
            .color(theme::MUTED),
        text_input(&app.i18n.text(Key::SpaceNamePlaceholder), &draft.name)
            .id("space-name")
            .size(14)
            .padding([10, 14])
            .style(theme::input)
            .on_input(|value| Message::Space(spaces::Message::NameChanged(value)))
            .on_submit(Message::Space(spaces::Message::Create)),
    ]
    .spacing(8);
    let mut projects = column![
        text(app.i18n.text(Key::Projects))
            .size(14)
            .font(theme::semibold())
    ]
    .spacing(10);
    if draft.directories.is_empty() {
        projects = projects.push(
            text(app.i18n.text(Key::SpaceProjectsHint))
                .size(13)
                .color(theme::MUTED),
        );
    } else {
        let mut folders = Column::new().spacing(4);
        for (index, directory) in draft.directories.iter().enumerate() {
            let path = directory.to_string_lossy();
            let title = directory
                .file_name()
                .unwrap_or(directory.as_os_str())
                .to_string_lossy();
            let remove = tooltip(
                button(icon(Icon::Close, theme::MUTED))
                    .padding(8)
                    .style(theme::icon_action(false))
                    .on_press_maybe(
                        (!app.picking)
                            .then_some(Message::Space(spaces::Message::RemoveProject(index))),
                    ),
                text(app.i18n.text(Key::RemoveProject)).size(12),
                tooltip::Position::Bottom,
            )
            .style(theme::tooltip);
            folders = folders.push(
                container(
                    row![
                        icon(Icon::Folder, theme::MUTED),
                        column![
                            text(short(&title, 32)).size(14),
                            text(short_path(&path, 56)).size(11).color(theme::FAINT)
                        ]
                        .spacing(3)
                        .width(Length::Fill),
                        remove,
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                )
                .padding([8, 10])
                .style(theme::tooltip),
            );
        }
        projects = projects.push(
            scrollable(folders)
                .style(theme::scroll)
                .height((draft.directories.len() as f32 * 60.).min(160.)),
        );
    }
    projects = projects.push(
        container(
            button(
                row![
                    icon(Icon::Plus, theme::TEXT),
                    text(app.i18n.text(if app.picking {
                        Key::ChoosingDirectory
                    } else {
                        Key::AddProjectFolder
                    }))
                    .size(14),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([10, 14])
            .style(theme::secondary)
            .on_press_maybe(
                (!app.picking).then_some(Message::Space(spaces::Message::PickProject(None))),
            ),
        )
        .id("space-add-project"),
    );
    let actions = row![
        space().width(Length::Fill),
        container(
            button(text(app.i18n.text(Key::Cancel)).size(14))
                .padding([10, 18])
                .style(theme::secondary)
                .on_press(Message::Space(spaces::Message::Cancel))
        )
        .id("space-cancel"),
        container(
            button(
                text(app.i18n.text(if draft.editing.is_some() {
                    Key::SaveChanges
                } else {
                    Key::CreateSpace
                }))
                .size(14)
            )
            .padding([10, 18])
            .style(theme::primary)
            .on_press_maybe(
                (!app.picking && !draft.name.trim().is_empty())
                    .then_some(Message::Space(spaces::Message::Create))
            )
        )
        .id("space-create"),
    ]
    .spacing(8);
    let content = column![
        text(app.i18n.text(if draft.editing.is_some() {
            Key::EditSpace
        } else {
            Key::NewSpace
        }))
        .size(24)
        .font(theme::semibold()),
        text(app.i18n.text(Key::SpaceDescription))
            .size(14)
            .color(theme::MUTED),
        name,
        projects,
        actions,
    ]
    .spacing(20);
    modal::overlay(base, content.into(), 560.)
}
