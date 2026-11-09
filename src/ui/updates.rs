//! Present update actions without exposing transport or installer details.

use crate::{
    app::{
        App, Message,
        updates::{Event, State},
    },
    i18n::{FluentArgs, Key},
    ui::{icons::Icon, theme},
};
use iced::{
    Element,
    widget::{button, column, container, text},
};

pub(crate) fn sidebar(app: &App) -> Option<Element<'_, Message>> {
    let (label, action) = match &app.updates {
        State::Available(_) => (Key::DownloadUpdate, Some(Event::Download)),
        State::Downloading => (Key::DownloadingUpdate, None),
        State::Ready(_) => (Key::InstallUpdate, Some(Event::Install)),
        State::Saving(_) | State::Installing(_) => (Key::InstallingUpdate, None),
        _ => return None,
    };
    Some(crate::ui::screens::workspace::navigation_button(
        app,
        Icon::Update,
        label,
        action.map(Message::Update),
        false,
        "sidebar-update",
    ))
}

pub(crate) fn settings(app: &App) -> Element<'_, Message> {
    let (label, action) = match &app.updates {
        State::Idle | State::Current | State::Failed(_) => (Key::CheckUpdates, Some(Event::Check)),
        State::Checking => (Key::CheckingUpdates, None),
        State::Available(_) => (Key::DownloadUpdate, Some(Event::Download)),
        State::Downloading => (Key::DownloadingUpdate, None),
        State::Ready(_) => (Key::InstallUpdate, Some(Event::Install)),
        State::Saving(_) | State::Installing(_) => (Key::InstallingUpdate, None),
    };
    let description = match &app.updates {
        State::Current => app.i18n.text(Key::UpToDate),
        State::Available(release) => version(app, &release.version),
        State::Ready(prepared) | State::Saving(prepared) | State::Installing(prepared) => {
            version(app, &prepared.version)
        }
        State::Failed(error) => app.i18n.text(crate::app::updates::error_key(*error)),
        _ => app.i18n.text(Key::UpdatesDescription),
    };
    column![
        text(app.i18n.text(Key::Updates)).size(15),
        text(description).size(13).style(theme::MUTED.text()),
        container(
            button(text(app.i18n.text(label)).size(13))
                .padding([8, 14])
                .on_press_maybe(action.map(Message::Update))
                .style(theme::primary)
        )
        .id("settings-update")
    ]
    .spacing(10)
    .into()
}

fn version(app: &App, version: &str) -> String {
    let mut args = FluentArgs::new();
    args.set("version", version);
    app.i18n.format(Key::UpdateAvailable, &args)
}
