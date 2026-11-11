use super::modal;
use crate::{
    app::{App, Message, project_add::Message as Event},
    i18n::Key,
    ui::theme,
};
use iced::{
    Element, Length,
    widget::{button, column, container, row, space, text, text_input},
};

pub(super) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(draft) = &app.project_add else {
        return base;
    };
    let available = !draft.pending && !draft.picking;
    let mut content = column![
        text(app.i18n.text(if draft.cloning {
            Key::CloneRepository
        } else {
            Key::AddProject
        }))
        .size(24)
        .font(theme::semibold()),
    ]
    .spacing(16);
    if draft.cloning {
        let input = text_input(&app.i18n.text(Key::RepositoryUrlPlaceholder), &draft.remote)
            .id("clone-url")
            .padding([10, 14])
            .size(14)
            .style(theme::input);
        let destination = text_input(&app.i18n.text(Key::CloneDestination), &draft.destination)
            .id("clone-destination")
            .padding([10, 14])
            .size(14)
            .style(theme::input);
        content = content
            .push(
                column![
                    text(app.i18n.text(Key::RepositoryUrl))
                        .size(13)
                        .style(theme::MUTED.text()),
                    if available {
                        input.on_input(|value| Message::ProjectAdd(Event::Url(value)))
                    } else {
                        input
                    },
                ]
                .spacing(8),
            )
            .push(
                column![
                    text(app.i18n.text(Key::CloneDestination))
                        .size(13)
                        .style(theme::MUTED.text()),
                    if available {
                        destination.on_input(|value| Message::ProjectAdd(Event::Destination(value)))
                    } else {
                        destination
                    },
                    container(
                        button(text(app.i18n.text(Key::ChooseCloneParent)).size(14))
                            .padding([10, 14])
                            .style(theme::secondary)
                            .on_press_maybe(
                                available.then_some(Message::ProjectAdd(Event::PickParent))
                            )
                    )
                    .id("clone-pick-parent"),
                ]
                .spacing(8),
            );
        if let Some(error) = draft.error {
            content = content.push(
                text(app.i18n.text(error_key(error)))
                    .size(13)
                    .style(theme::RED.text()),
            );
        }
        if draft.pending {
            content = content.push(
                text(app.i18n.text(Key::CloningRepository))
                    .size(14)
                    .style(theme::MUTED.text()),
            );
        }
    } else {
        content = content.push(
            text(app.i18n.text(Key::AddProjectSource))
                .size(14)
                .style(theme::MUTED.text()),
        );
        for (id, key, event) in [
            (
                "add-project-directories",
                Key::AddProjectFolder,
                Event::Directories,
            ),
            ("add-project-remote", Key::CloneRepository, Event::Remote),
        ] {
            content = content.push(
                container(
                    button(text(app.i18n.text(key)).size(14))
                        .on_press(Message::ProjectAdd(event))
                        .padding([14, 18])
                        .width(Length::Fill)
                        .style(theme::secondary),
                )
                .id(id),
            );
        }
    }
    let mut actions = row![
        space().width(Length::Fill),
        container(
            button(text(app.i18n.text(Key::Cancel)).size(14))
                .on_press_maybe((!draft.pending).then_some(Message::ProjectAdd(Event::Cancel)))
                .padding([10, 18])
                .style(theme::secondary)
        )
        .id("add-project-cancel"),
    ]
    .spacing(8);
    if draft.cloning {
        actions = actions.push(
            container(
                button(text(app.i18n.text(Key::CloneRepository)).size(14))
                    .on_press_maybe(
                        (available
                            && !draft.remote.trim().is_empty()
                            && !draft.destination.trim().is_empty())
                        .then_some(Message::ProjectAdd(Event::Clone)),
                    )
                    .padding([10, 18])
                    .style(theme::primary),
            )
            .id("clone-submit"),
        );
    }
    modal::overlay(
        base,
        content.push(actions).into(),
        600.,
        Message::ProjectAdd(Event::Cancel),
    )
}

fn error_key(error: crate::app::project_add::Error) -> Key {
    use crate::app::project_add::Error;
    match error {
        Error::InvalidRemote => Key::CloneInvalidRemote,
        Error::Destination => Key::CloneInvalidDestination,
        Error::Exists => Key::CloneDestinationExists,
        Error::Failed => Key::CloneFailed,
    }
}
