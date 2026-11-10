use super::{
    modal,
    workspace::{short, short_path},
};
use crate::ui::tooltip;
use crate::{
    app::{App, Message, spaces},
    i18n::Key,
    ui::{
        icons::{Icon, icon},
        reorder, theme,
    },
};
use iced::{
    Alignment, Element, Length,
    widget::{Column, button, column, container, row, scrollable, space, text, text_input},
};

pub(super) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(draft) = &app.space_draft else {
        return base;
    };
    let name = column![
        text(app.i18n.text(Key::SpaceName))
            .size(13)
            .style(theme::MUTED.text()),
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
        row![
            text(app.i18n.text(Key::Projects))
                .size(14)
                .font(theme::semibold())
                .width(Length::Fill),
            text(app.i18n.text(Key::ReorderProjectsHint))
                .size(12)
                .style(theme::MUTED.text()),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
    ]
    .spacing(10);
    if draft.directories.is_empty() {
        projects = projects.push(
            text(app.i18n.text(Key::SpaceProjectsHint))
                .size(13)
                .style(theme::MUTED.text()),
        );
    } else {
        let mut folders = Column::new().spacing(theme::SPACE_PROJECT_GAP);
        for (index, directory) in draft.directories.iter().enumerate() {
            let path = directory.to_string_lossy();
            let title = directory
                .file_name()
                .unwrap_or(directory.as_os_str())
                .to_string_lossy();
            let remove = tooltip::view(
                container(
                    button(icon(Icon::Close, theme::MUTED))
                        .padding(8)
                        .style(theme::icon_action(false))
                        .on_press_maybe(
                            (!app.picking)
                                .then_some(Message::Space(spaces::Message::RemoveProject(index))),
                        ),
                )
                .id(format!("space-remove-project-{index}")),
                text(app.i18n.text(Key::RemoveProject)).size(12),
                tooltip::Position::Bottom,
            );
            folders = folders.push(
                container(
                    row![
                        icon(Icon::Drag, theme::FAINT),
                        icon(Icon::Folder, theme::MUTED),
                        column![
                            text(short(&title, 32)).size(14),
                            text(short_path(&path, 56))
                                .size(11)
                                .style(theme::FAINT.text())
                        ]
                        .spacing(3)
                        .width(Length::Fill),
                        remove,
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                )
                .id(format!("space-draft-project-{index}"))
                .height(theme::SPACE_PROJECT_HEIGHT)
                .padding([8, 10])
                .style(theme::tooltip),
            );
        }
        let count = draft.directories.len() as f32;
        let height = count * theme::SPACE_PROJECT_HEIGHT + (count - 1.) * theme::SPACE_PROJECT_GAP;
        let folders: Element<'_, Message> = if app.picking {
            folders.into()
        } else {
            reorder::column(
                folders,
                46.,
                |from, to| Message::Space(spaces::Message::MoveProject(None, from, to)),
                |offset| Message::Space(spaces::Message::ScrollProjects(None, offset)),
            )
        };
        projects = projects.push(
            scrollable(folders)
                .id("space-project-list")
                .style(theme::scroll)
                .height(
                    height.min(theme::SPACE_PROJECT_HEIGHT * 5.5 + theme::SPACE_PROJECT_GAP * 5.),
                ),
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
    let mut actions = iced::widget::Row::new()
        .spacing(8)
        .align_y(Alignment::Center);
    if let Some(index) = draft.editing {
        actions = actions.push(
            container(
                button(
                    text(app.i18n.text(Key::DeleteSpace))
                        .size(14)
                        .style(theme::GIT_REMOVED.text()),
                )
                .padding([10, 18])
                .style(theme::secondary)
                .on_press_maybe(
                    (!app.picking && !app.git_actions.pending)
                        .then_some(Message::Space(spaces::Message::Delete(index))),
                ),
            )
            .id("space-delete"),
        );
    }
    let actions = actions.extend([
        space().width(Length::Fill).into(),
        container(
            button(text(app.i18n.text(Key::Cancel)).size(14))
                .padding([10, 18])
                .style(theme::secondary)
                .on_press(Message::Space(spaces::Message::Cancel)),
        )
        .id("space-cancel")
        .into(),
        container(
            button(
                text(app.i18n.text(if draft.editing.is_some() {
                    Key::SaveChanges
                } else {
                    Key::CreateSpace
                }))
                .size(14),
            )
            .padding([10, 18])
            .style(theme::primary)
            .on_press_maybe(
                (!app.picking && !draft.name.trim().is_empty())
                    .then_some(Message::Space(spaces::Message::Create)),
            ),
        )
        .id("space-create")
        .into(),
    ]);
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
            .style(theme::MUTED.text()),
        name,
        projects,
        actions,
    ]
    .spacing(14);
    modal::overlay(base, content.into(), 560.)
}
