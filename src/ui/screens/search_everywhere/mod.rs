//! Present a keyboard-driven search popup above the workspace.

use crate::{
    app::{
        App, Message,
        search_everywhere::{Event, INPUT_ID, RESULTS_ID, Row, Scope, visible_window},
    },
    i18n::Key,
    ui::{context_menu, ellipsis, file_icons, icons, theme},
};
use iced::{
    Alignment, Element, Length,
    alignment::Horizontal,
    widget::{
        button, column, container, mouse_area, opaque, row, scrollable, space, stack, text,
        text_input,
    },
};

pub(super) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(dialog) = &app.search_everywhere.dialog else {
        return base;
    };
    let input = text_input(&app.i18n.text(Key::SearchPlaceholder), &dialog.query)
        .id(INPUT_ID)
        .on_input(|value| Message::SearchEverywhere(Event::Changed(value)))
        .on_submit(Message::SearchEverywhere(Event::Selected))
        .size(14)
        .padding([10, 12])
        .style(theme::input);

    let mut results = column(Vec::new()).spacing(theme::SEARCH_ROW_SPACING);
    if dialog.query.is_empty() {
        results = results.push(hint(app, Key::SearchStart));
    } else if dialog.rows.is_empty() {
        results = results.push(hint(app, Key::SearchNoResults));
    } else {
        let mut files_header = false;
        let mut projects_header = false;
        let visible = visible_window(dialog.rows.len(), dialog.selection);
        for (offset, entry) in dialog.rows[visible.clone()].iter().enumerate() {
            let index = visible.start + offset;
            match entry {
                Row::File {
                    name,
                    directory,
                    path,
                } => {
                    if !files_header {
                        files_header = true;
                        results = results.push(group(app, Key::SearchGroupFiles));
                    }
                    results = results.push(file_row(
                        app,
                        index,
                        dialog.selection,
                        name,
                        directory,
                        path,
                    ));
                }
                Row::Project { space, project } => {
                    if !projects_header {
                        projects_header = true;
                        results = results.push(group(app, Key::SearchGroupProjects));
                    }
                    results =
                        results.push(project_row(app, index, dialog.selection, *space, *project));
                }
            }
        }
    }

    let mut footer = row(Vec::new()).spacing(6).align_y(Alignment::Center);
    for scope in [Scope::All, Scope::Files, Scope::Projects] {
        footer = footer.push(scope_chip(app, scope, dialog.scope));
    }
    footer = footer.push(
        text(app.i18n.text(Key::SearchHint))
            .width(Length::Fill)
            .align_x(Horizontal::Right)
            .size(11)
            .style(theme::MUTED.text()),
    );

    let content = column![
        row![icons::icon(icons::Icon::Search, theme::MUTED), input]
            .spacing(10)
            .align_y(Alignment::Center),
        container(scrollable(results).id(RESULTS_ID).style(theme::scroll),)
            .max_height(theme::SEARCH_RESULTS_HEIGHT),
        footer,
    ]
    .spacing(12);

    let popup = container(content)
        .width(theme::SEARCH_OVERLAY_WIDTH)
        .padding(20)
        .style(theme::dialog);
    let area = container(popup)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Center)
        .padding(iced::Padding {
            top: theme::SEARCH_OVERLAY_TOP_INSET,
            ..Default::default()
        });
    let backdrop = container(stack![
        mouse_area(container(space()).width(Length::Fill).height(Length::Fill))
            .on_press(Message::SearchEverywhere(Event::Cancel)),
        area,
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_| container::Style {
        background: Some(iced::Color::from_rgba(0., 0., 0., 0.5).into()),
        ..Default::default()
    });
    let layered = opaque(stack![base, opaque(backdrop)]);
    context_menu::on_key(layered, |event| {
        key_event(event).map(Message::SearchEverywhere)
    })
}

/// Map popup keys before the focused input can consume them.
fn key_event(event: &iced::keyboard::Event) -> Option<Event> {
    use iced::keyboard::{Event as KeyEvent, Key, key::Named};
    match event {
        KeyEvent::KeyPressed {
            key: Key::Named(Named::ArrowDown),
            ..
        } => Some(Event::Moved(true)),
        KeyEvent::KeyPressed {
            key: Key::Named(Named::ArrowUp),
            ..
        } => Some(Event::Moved(false)),
        KeyEvent::KeyPressed {
            key: Key::Named(named),
            repeat: false,
            ..
        } if matches!(named, Named::Enter | Named::Tab | Named::Escape) => match named {
            Named::Enter => Some(Event::Selected),
            Named::Tab => Some(Event::CycleScope),
            _ => Some(Event::Cancel),
        },
        _ => None,
    }
}

fn hint<'a>(app: &'a App, key: Key) -> Element<'a, Message> {
    container(text(app.i18n.text(key)).size(13).style(theme::MUTED.text()))
        .padding([10, 10])
        .into()
}

fn group<'a>(app: &'a App, key: Key) -> Element<'a, Message> {
    container(
        text(app.i18n.text(key))
            .size(11)
            .font(theme::semibold())
            .style(theme::MUTED.text()),
    )
    .padding(iced::Padding {
        top: 10.,
        right: 10.,
        bottom: 0.,
        left: 10.,
    })
    .height(Length::Fixed(theme::SEARCH_GROUP_HEIGHT))
    .into()
}

fn file_row<'a>(
    _app: &'a App,
    index: usize,
    selection: usize,
    name: &'a str,
    directory: &'a str,
    path: &'a std::path::Path,
) -> Element<'a, Message> {
    let mut label = row![file_icons::file(path), ellipsis::text(name, 14.)]
        .spacing(10)
        .align_y(Alignment::Center);
    if !directory.is_empty() {
        label = label.push(
            text(directory)
                .width(Length::Fill)
                .align_x(Horizontal::Right)
                .size(12)
                .style(theme::MUTED.text()),
        );
    }
    container(
        button(label)
            .width(Length::Fill)
            .height(Length::Fixed(theme::SEARCH_ROW_HEIGHT))
            .padding([0, 10])
            .style(move |theme, status| theme::action(index == selection)(theme, status))
            .on_press(Message::SearchEverywhere(Event::Activate(index))),
    )
    .id(format!("search-everywhere-row-{index}"))
    .into()
}

fn project_row<'a>(
    app: &'a App,
    index: usize,
    selection: usize,
    space: usize,
    project: usize,
) -> Element<'a, Message> {
    let entry = app
        .workspace
        .spaces
        .get(space)
        .and_then(|space| space.projects.get(project));
    let (name, space_name) = match entry {
        Some(project) => (
            project.name.as_str(),
            app.workspace.spaces[space].name.as_str(),
        ),
        None => ("", ""),
    };
    let label = row![
        icons::icon(icons::Icon::Folder, theme::MUTED),
        ellipsis::text(name, 14.),
        text(space_name)
            .width(Length::Fill)
            .align_x(Horizontal::Right)
            .size(12)
            .style(theme::MUTED.text()),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    container(
        button(label)
            .width(Length::Fill)
            .height(Length::Fixed(theme::SEARCH_ROW_HEIGHT))
            .padding([0, 10])
            .style(move |theme, status| theme::action(index == selection)(theme, status))
            .on_press(Message::SearchEverywhere(Event::Activate(index))),
    )
    .id(format!("search-everywhere-row-{index}"))
    .into()
}

fn scope_chip<'a>(app: &'a App, scope: Scope, active: Scope) -> Element<'a, Message> {
    let key = match scope {
        Scope::All => Key::SearchScopeAll,
        Scope::Files => Key::SearchScopeFiles,
        Scope::Projects => Key::SearchScopeProjects,
    };
    button(text(app.i18n.text(key)).size(12))
        .on_press(Message::SearchEverywhere(Event::ChangeScope(scope)))
        .padding([4, 12])
        .style(move |theme, status| {
            if scope == active {
                theme::primary(theme, status)
            } else {
                theme::action(false)(theme, status)
            }
        })
        .into()
}

#[cfg(test)]
mod tests;
