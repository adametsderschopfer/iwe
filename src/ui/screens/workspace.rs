use super::{auth, git, pipelines, settings};
use crate::{
    app::{App, Message, Navigation, Panel, error_key},
    i18n::{FluentArgs, Key},
    ui::{
        code, file_icons,
        icons::{Icon, icon},
        theme,
    },
};
use iced::{
    Alignment, Element, Font, Length,
    widget::{
        self, Column, Row, button, column, container, mouse_area, pane_grid, responsive, rich_text,
        row, scrollable, space, text, tooltip,
    },
};
use std::path::Path;

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let content: Element<'_, Message> = if app.navigation == Navigation::Settings {
        settings::view(app)
    } else {
        let panes = pane_grid(&app.panes, |_, panel, _| {
            pane_grid::Content::new(
                container(match panel {
                    Panel::Workspace => preview(app),
                    Panel::Git => git::view(app),
                    Panel::Pipelines => pipelines::view(app),
                })
                .width(Length::Fill)
                .height(Length::Fill)
                .style(theme::surface),
            )
        })
        .spacing(1)
        .style(theme::panes)
        .on_resize(6, Message::Resize);
        let mut body = column![
            container(panes)
                .style(theme::separator)
                .height(Length::Fill)
        ];
        if app.workspace.directory().is_some() {
            body = body.push(footer(app));
        }
        body.into()
    };
    let mut main = column![content].height(Length::Fill).width(Length::Fill);
    if let Some(notice) = app.notice {
        main = main.push(
            container(
                row![
                    text(app.i18n.text(notice)).size(13).width(Length::Fill),
                    icon_button(app, Icon::Close, Key::Dismiss, Message::ClearNotice, false)
                ]
                .align_y(Alignment::Center)
                .spacing(12),
            )
            .padding([8, 16])
            .style(theme::badge),
        );
    }
    let base = container(row![
        column![sidebar_header(), sidebar(app)]
            .width(theme::SIDEBAR_WIDTH)
            .height(Length::Fill),
        column![header(app), main]
            .width(Length::Fill)
            .height(Length::Fill),
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::surface)
    .into();
    super::space_form::overlay(app, auth::overlay(app, base))
}

fn header(app: &App) -> Element<'_, Message> {
    let is_settings = app.navigation == Navigation::Settings;
    let selected_space = app
        .workspace
        .selected_space
        .and_then(|index| app.workspace.spaces.get(index));
    let title = if is_settings {
        app.i18n.text(Key::Settings)
    } else if let Some(space) = selected_space {
        if app.navigation == Navigation::Files {
            app.workspace
                .project()
                .map(|project| format!("{} • {}", short(&space.name, 24), short(&project.name, 28)))
                .unwrap_or_else(|| short(&space.name, 42))
        } else {
            short(&space.name, 42)
        }
    } else {
        app.i18n.text(Key::Workspace)
    };
    let mut heading = row![
        container(
            text(title)
                .size(18)
                .wrapping(text::Wrapping::None)
                .font(theme::semibold())
        )
        .id("workspace-heading")
        .clip(true)
        .width(Length::Fill),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    if let Some(repo) = &app.repository
        && !is_settings
    {
        heading = heading.push(
            container(
                row![
                    icon(Icon::Branch, theme::MUTED),
                    text(if repo.head.is_empty() {
                        app.i18n.text(Key::DetachedHead)
                    } else {
                        short(&repo.head, 24)
                    })
                    .size(12)
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([4, 10])
            .style(theme::badge),
        );
    }
    let action = if is_settings {
        icon_button(
            app,
            Icon::Close,
            Key::BackToWorkspace,
            Message::Navigate(Navigation::Projects),
            false,
        )
    } else {
        icon_button(app, Icon::Refresh, Key::Refresh, Message::Refresh, false)
    };
    heading = heading.push(container(action).id("window-header-action"));

    mouse_area(
        container(heading)
            .height(56)
            .width(Length::Fill)
            .padding([8, 24])
            .style(theme::surface),
    )
    .on_press(Message::DragWindow)
    .on_double_click(Message::ToggleWindowMaximized)
    .into()
}

fn sidebar_header() -> Element<'static, Message> {
    mouse_area(
        container(space())
            .id("window-header-drag-area")
            .width(theme::SIDEBAR_WIDTH)
            .height(theme::SIDEBAR_TOP_INSET)
            .style(theme::sidebar),
    )
    .on_press(Message::DragWindow)
    .on_double_click(Message::ToggleWindowMaximized)
    .into()
}

fn footer(app: &App) -> Element<'_, Message> {
    let path = app
        .preview()
        .path
        .as_ref()
        .or(app.workspace.directory())
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| app.i18n.text(Key::LocalWorkspace));
    let mut content = row![
        tooltip(
            container(
                button(
                    text(short_path(&path, 100))
                        .size(12)
                        .color(theme::FAINT)
                        .wrapping(text::Wrapping::None)
                )
                .on_press(Message::CopyPath)
                .padding([4, 8])
                .width(Length::Fill)
                .style(theme::action(false))
            )
            .id("footer-path")
            .width(Length::Fill),
            text(format!("{}\n{}", path, app.i18n.text(Key::CopyPath))).size(12),
            tooltip::Position::Top
        )
        .style(theme::tooltip),
    ]
    .spacing(16);
    if app.loading {
        content = content.push(
            text(app.i18n.text(Key::Loading))
                .size(12)
                .color(theme::MUTED),
        );
    }
    container(content).padding([6, 16]).into()
}

fn sidebar(app: &App) -> Element<'_, Message> {
    let is_files = app.navigation == Navigation::Files;
    let navigation: Element<'_, Message> = if is_files {
        navigation_button(
            app,
            Icon::Back,
            Key::BackToSpaces,
            Some(Message::Navigate(Navigation::Projects)),
            false,
            "back-to-spaces",
        )
    } else {
        container(
            row![
                text(app.i18n.text(Key::Spaces))
                    .size(13)
                    .color(theme::FAINT)
                    .width(Length::Fill),
                tooltip(
                    container(
                        button(icon(Icon::Plus, theme::MUTED))
                            .padding(4)
                            .width(28)
                            .height(28)
                            .style(theme::action(false))
                            .on_press_maybe(
                                (app.loaded && !app.picking)
                                    .then_some(Message::Space(crate::app::spaces::Message::New))
                            )
                    )
                    .id("new-space"),
                    text(app.i18n.text(Key::NewSpace)).size(12),
                    tooltip::Position::Right,
                )
                .style(theme::tooltip),
            ]
            .align_y(Alignment::Center)
            .spacing(8),
        )
        .padding([4, 8])
        .into()
    };
    let mut body = Column::new()
        .spacing(if is_files { 0 } else { 4 })
        .width(Length::Fill);
    if is_files {
        if let Some(root) = app.workspace.directory() {
            let width = app
                .tree
                .get(root)
                .into_iter()
                .flatten()
                .map(|entry| tree_width(app, &entry.path, 0))
                .fold(theme::SIDEBAR_WIDTH - 24., f32::max);
            body = file_tree(app, root, 0, body).width(width);
        } else {
            body = body.push(
                text(app.i18n.text(Key::NoProject))
                    .size(13)
                    .color(theme::MUTED),
            );
        }
    } else {
        for (space_index, group) in app.workspace.spaces.iter().enumerate() {
            body = body.push(
                container(
                    row![
                        button(
                            row![
                                file_icons::chevron(!app.collapsed_spaces.contains(&space_index)),
                                text(short(&group.name, 22))
                                    .size(13)
                                    .font(theme::semibold())
                                    .wrapping(text::Wrapping::None)
                                    .width(Length::Fill),
                                text(group.projects.len()).size(12).color(theme::FAINT),
                            ]
                            .spacing(6)
                            .align_y(Alignment::Center)
                            .height(28)
                        )
                        .padding([2, 10])
                        .width(Length::Fill)
                        .on_press(Message::ToggleSpace(space_index))
                        .style(theme::action(false)),
                        tooltip(
                            container(
                                button(icon(Icon::Settings, theme::FAINT))
                                    .padding(6)
                                    .width(32)
                                    .height(32)
                                    .style(theme::action(false))
                                    .on_press_maybe((!app.picking).then_some(Message::Space(
                                        crate::app::spaces::Message::Edit(space_index)
                                    )))
                            )
                            .id(format!("edit-space-{space_index}")),
                            text(app.i18n.text(Key::EditSpace)).size(12),
                            tooltip::Position::Right
                        )
                        .style(theme::tooltip),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                )
                .id(format!("space-group-{space_index}"))
                .padding([3, 0])
                .width(Length::Fill),
            );
            if app.collapsed_spaces.contains(&space_index) {
                continue;
            }
            for (project_index, project) in group.projects.iter().enumerate() {
                let path = project.directory.to_string_lossy().into_owned();
                let active = app.workspace.selected_space == Some(space_index)
                    && app.workspace.selected_project == Some(project_index);
                let project = tooltip(
                    container(
                        button(
                            mouse_area(
                                container(
                                    row![
                                        icon(
                                            Icon::Folder,
                                            if active { theme::TEXT } else { theme::MUTED }
                                        ),
                                        text(short(&project.name, 20)).size(14).width(Length::Fill),
                                    ]
                                    .spacing(10)
                                    .align_y(Alignment::Center),
                                )
                                .padding([9, 12])
                                .width(Length::Fill),
                            )
                            .on_press(Message::SelectProject(space_index, project_index))
                            .on_double_click(Message::OpenProjectFiles(space_index, project_index)),
                        )
                        .padding(0)
                        .width(Length::Fill)
                        .style(theme::action(active))
                        .on_press(Message::SelectProject(space_index, project_index)),
                    )
                    .id(format!("space-project-{space_index}-{project_index}"))
                    .width(Length::Fill),
                    text(path).size(12),
                    tooltip::Position::Right,
                )
                .style(theme::tooltip);
                let files = tooltip(
                    container(
                        button(icon(Icon::FolderOpen, theme::MUTED))
                            .padding(6)
                            .width(32)
                            .height(32)
                            .style(theme::action(false))
                            .on_press(Message::OpenProjectFiles(space_index, project_index)),
                    )
                    .id(format!("project-files-{space_index}-{project_index}")),
                    text(app.i18n.text(Key::ProjectFiles)).size(12),
                    tooltip::Position::Right,
                )
                .style(theme::tooltip);
                body = body.push(row![project, files].spacing(4).align_y(Alignment::Center));
            }
            if group.projects.is_empty() {
                body = body.push(
                    container(
                        text(app.i18n.text(Key::NoProjects))
                            .size(12)
                            .color(theme::FAINT),
                    )
                    .padding([6, 12]),
                );
            }
            body = body.push(
                button(
                    row![
                        icon(Icon::Plus, theme::MUTED),
                        text(app.i18n.text(Key::AddProject)).size(12)
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .on_press_maybe((!app.picking).then_some(Message::Space(
                    crate::app::spaces::Message::PickProject(Some(space_index)),
                )))
                .style(theme::action(false))
                .padding([8, 12])
                .width(Length::Fill),
            );
            body = body.push(space().height(8));
        }
        if app.workspace.spaces.is_empty() {
            body = body.push(
                container(
                    text(app.i18n.text(Key::NoSpaces))
                        .size(13)
                        .color(theme::MUTED),
                )
                .padding(12),
            );
        }
    }
    let mut tools = Column::new().spacing(4);
    if app.workspace.directory().is_some() {
        tools = tools.push(
            container(
                text(app.i18n.text(Key::Panels))
                    .size(12)
                    .color(theme::FAINT),
            )
            .padding([8, 12]),
        );
        tools = tools.push(navigation_button(
            app,
            Icon::Branch,
            Key::Git,
            Some(Message::TogglePanel(Panel::Git)),
            app.workspace.preferences.show_git,
            "nav-git",
        ));
    }
    if app.has_remote() {
        tools = tools.push(navigation_button(
            app,
            Icon::Play,
            Key::Pipelines,
            Some(Message::TogglePanel(Panel::Pipelines)),
            app.workspace.preferences.show_pipelines,
            "nav-pipelines",
        ));
    }
    tools = tools.push(space().height(8)).push(navigation_button(
        app,
        Icon::Settings,
        Key::Settings,
        Some(Message::Navigate(Navigation::Settings)),
        app.navigation == Navigation::Settings,
        "nav-settings",
    ));
    container(column![
        navigation,
        space().height(8),
        scrollable(body)
            .direction(if is_files {
                scrollable::Direction::Both {
                    vertical: scrollable::Scrollbar::default(),
                    horizontal: scrollable::Scrollbar::default(),
                }
            } else {
                scrollable::Direction::Vertical(scrollable::Scrollbar::default())
            })
            .id("project-browser")
            .style(theme::scroll)
            .height(Length::Fill),
        space().height(16),
        tools,
    ])
    .width(theme::SIDEBAR_WIDTH)
    .height(Length::Fill)
    .padding(iced::Padding {
        top: 0.,
        right: 12.,
        bottom: 12.,
        left: 12.,
    })
    .style(theme::sidebar)
    .into()
}

fn navigation_button<'a>(
    app: &'a App,
    symbol: Icon,
    label: Key,
    message: Option<Message>,
    selected: bool,
    id: &'static str,
) -> Element<'a, Message> {
    let panel_control = matches!(&message, Some(Message::TogglePanel(_)));
    let mut content = row![
        icon(symbol, if selected { theme::TEXT } else { theme::MUTED }),
        text(app.i18n.text(label)).size(14).width(Length::Fill),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    if panel_control {
        content = content.push(icon(
            if selected { Icon::Close } else { Icon::Plus },
            theme::FAINT,
        ));
    }
    container(
        button(content)
            .padding([10, 12])
            .width(Length::Fill)
            .on_press_maybe(message)
            .style(theme::action(selected && !panel_control)),
    )
    .id(id)
    .into()
}

fn status_badge(
    app: &App,
    status: Option<crate::domain::changes::FileStatus>,
    directory: bool,
) -> Element<'_, Message> {
    use crate::domain::changes::FileStatus;
    let Some(status) = status else {
        return space().width(14).into();
    };
    let label = match status {
        FileStatus::Untracked => Key::FileUntracked,
        FileStatus::Added => Key::FileAdded,
        FileStatus::Modified => Key::FileModified,
        FileStatus::Deleted => Key::FileDeleted,
        FileStatus::Renamed => Key::FileRenamed,
        FileStatus::Conflict => Key::FileConflict,
    };
    tooltip(
        text(if directory { "●" } else { status.badge() })
            .size(if directory { 8 } else { 11 })
            .color(theme::file_status(status))
            .width(14)
            .align_x(Alignment::Center),
        text(app.i18n.text(label)).size(12),
        tooltip::Position::Right,
    )
    .style(theme::tooltip)
    .into()
}

fn tree_width(app: &App, path: &Path, depth: usize) -> f32 {
    let name = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy();
    let label_width: f32 = name
        .chars()
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
        .sum();
    let width = 72.
        + depth as f32 * theme::FILE_TREE_INDENT
        + label_width * (theme::FILE_TREE_TEXT_SIZE / 12.);
    if depth <= 24 && app.expanded.contains(path) {
        app.tree
            .get(path)
            .into_iter()
            .flatten()
            .map(|entry| tree_width(app, &entry.path, depth + 1))
            .fold(width, f32::max)
    } else {
        width
    }
}

fn file_tree<'a>(
    app: &'a App,
    directory: &Path,
    depth: usize,
    mut body: Column<'a, Message>,
) -> Column<'a, Message> {
    if depth > 25 {
        return body;
    }
    if let Some(entries) = app.tree.get(directory) {
        for entry in entries {
            let expanded = app.expanded.contains(&entry.path);
            let event = if entry.directory {
                Message::ToggleFolder(entry.path.clone())
            } else {
                Message::OpenFile(entry.path.clone())
            };
            let relative = app
                .workspace
                .directory()
                .and_then(|root| entry.path.strip_prefix(root).ok());
            let active = app.files.selected_path.as_ref() == Some(&entry.path)
                || (app.files.selected_path.is_none()
                    && app.preview().revision.is_none()
                    && relative.is_some_and(|path| path.to_string_lossy() == app.preview().title));
            let marker: Element<'_, Message> = if entry.directory {
                file_icons::chevron(expanded)
            } else {
                space().width(10).into()
            };
            let symbol = if entry.directory {
                let package = app
                    .tree
                    .get(&entry.path)
                    .is_some_and(|entries| entries.iter().any(|item| item.name == "__init__.py"));
                file_icons::folder(expanded, package)
            } else {
                file_icons::file(&entry.path)
            };
            let status = app.changes.status.status(&entry.path, entry.directory);
            let item = row![
                marker,
                symbol,
                text(&entry.name)
                    .size(theme::FILE_TREE_TEXT_SIZE)
                    .color(status.map(theme::file_status).unwrap_or(theme::TEXT))
                    .wrapping(text::Wrapping::None),
                space().width(Length::Fill),
                status_badge(app, status, entry.directory),
            ]
            .spacing(4)
            .padding(iced::Padding {
                left: depth as f32 * theme::FILE_TREE_INDENT,
                right: 0.,
                top: 0.,
                bottom: 0.,
            })
            .height(theme::FILE_TREE_ROW_HEIGHT)
            .align_y(Alignment::Center);
            body = body.push(
                container(
                    button(item)
                        .on_press(event)
                        .height(theme::FILE_TREE_ROW_HEIGHT)
                        .padding([0, 6])
                        .width(Length::Fill)
                        .style(theme::tree_item(active)),
                )
                .id(format!("file-tree-{}", entry.path.to_string_lossy()))
                .width(Length::Fill),
            );
            if entry.directory && expanded {
                body = file_tree(app, &entry.path, depth + 1, body);
            }
        }
    } else if app.files.loading.contains(directory) {
        body = body.push(
            text(app.i18n.text(Key::Loading))
                .size(12)
                .color(theme::MUTED),
        );
    }
    if let Some(error) = app.files.errors.get(directory) {
        body = body.push(
            column![
                text(app.i18n.text(error_key(*error)))
                    .size(12)
                    .color(theme::YELLOW),
                button(text(app.i18n.text(Key::Retry)).size(12))
                    .on_press(Message::Files(crate::app::files::Event::Retry(
                        directory.to_path_buf()
                    )))
                    .padding([6, 10])
                    .style(theme::secondary),
            ]
            .spacing(6),
        );
    }
    body
}

fn file_tabs(app: &App) -> Element<'_, Message> {
    let tabs = app
        .tabs
        .files
        .iter()
        .filter(|preview| !preview.title.is_empty())
        .map(|preview| {
            let active = preview.id == app.preview().id;
            let filename = Path::new(&preview.title)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&preview.title);
            let label = if let Some(revision) = &preview.revision {
                format!("{} · {}", short(filename, 12), short(revision, 7))
            } else {
                short(filename, 22)
            };
            let mut description = if let Some(revision) = &preview.revision {
                format!("{}\n{}", preview.title, revision)
            } else {
                preview.title.clone()
            };
            if preview
                .document
                .as_ref()
                .is_some_and(|document| document.comparison_failed)
            {
                description.push_str(&format!("\n{}", app.i18n.text(Key::ComparisonFailed)));
            }
            let select = tooltip(
                container(
                    button(
                        row![
                            file_icons::file(Path::new(&preview.title)),
                            text(label).size(12).wrapping(text::Wrapping::None),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    )
                    .on_press(Message::SelectTab(preview.id))
                    .padding([7, 8])
                    .width(Length::Fill)
                    .height(34)
                    .style(theme::file_tab_button),
                )
                .id(format!("file-tab-{}", preview.id))
                .width(Length::Fill),
                text(description).size(12),
                tooltip::Position::Bottom,
            )
            .style(theme::tooltip);
            let close = tooltip(
                container(
                    button(icon(Icon::Close, theme::FAINT))
                        .on_press(Message::CloseTab(preview.id))
                        .padding(4)
                        .width(28)
                        .height(28)
                        .style(theme::action(false)),
                )
                .id(format!("close-file-tab-{}", preview.id)),
                text(app.i18n.text(Key::CloseTab)).size(12),
                tooltip::Position::Bottom,
            )
            .style(theme::tooltip);
            mouse_area(
                container(column![
                    row![select, close, space().width(4)].align_y(Alignment::Center),
                    container(space())
                        .height(2)
                        .width(Length::Fill)
                        .style(theme::file_tab_indicator(active)),
                ])
                .width(theme::FILE_TAB_WIDTH)
                .style(theme::file_tab(active)),
            )
            .on_middle_press(Message::CloseTab(preview.id))
            .into()
        });
    let mut bar = row![
        scrollable(Row::with_children(tabs))
            .id("file-tabs")
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::default().width(4).scroller_width(4),
            ))
            .width(Length::Fill)
            .height(36)
            .style(theme::scroll),
    ]
    .align_y(Alignment::Center);
    if app.preview().error.is_none()
        && !app.preview().loading
        && app
            .preview()
            .document
            .as_ref()
            .is_some_and(|document| document.can_fold())
    {
        let collapsed = app.preview().folding.collapsed;
        bar = bar.push(
            container(
                tooltip(
                    button(icon(
                        if collapsed {
                            Icon::ExpandLines
                        } else {
                            Icon::CollapseLines
                        },
                        theme::MUTED,
                    ))
                    .padding(8)
                    .width(36)
                    .height(36)
                    .style(theme::icon_action(collapsed))
                    .on_press(Message::ToggleUnchanged(app.preview().id)),
                    text(app.i18n.text(if collapsed {
                        Key::ExpandUnchanged
                    } else {
                        Key::CollapseUnchanged
                    }))
                    .size(12),
                    tooltip::Position::Bottom,
                )
                .style(theme::tooltip),
            )
            .id("fold-unchanged"),
        );
    }
    container(bar).style(theme::sidebar).into()
}

fn preview(app: &App) -> Element<'_, Message> {
    let has_tabs = app
        .tabs
        .files
        .iter()
        .any(|preview| !preview.title.is_empty());
    let content: Element<'_, Message> = if app.preview().loading && app.preview().document.is_none()
    {
        center_message(app, Key::Loading)
    } else if let Some(error) = app.preview().error {
        center_message(app, error_key(error))
    } else if let Some(document) = &app.preview().document {
        responsive(move |viewport| {
            let size = app.workspace.preferences.code_size as f32;
            let height = size * 1.65;
            let gutter = if app.workspace.preferences.line_numbers {
                91.
            } else {
                35.
            };
            let width = (document.columns as f32 * size * 0.65 + gutter + 12.).max(viewport.width);
            let count = app.preview().folding.len(document);
            // Keep a stable row block while the viewport moves inside the buffer.
            let start =
                (((app.preview().scroll_y.max(0.) / height) as usize).saturating_sub(16) / 32 * 32)
                    .min(count);
            let end = (start + (viewport.height / height).ceil() as usize + 64).min(count);
            let lines = (start..end).map(|row| {
                let index = match app.preview().folding.row(row) {
                    code::folding::Row::Line(index) => index,
                    code::folding::Row::Fold(range) => {
                        let mut args = FluentArgs::new();
                        args.set("start", document.line_info[range.start].number);
                        args.set("end", document.line_info[range.end - 1].number);
                        return container(
                            button(
                                row![
                                    space().width(48),
                                    file_icons::chevron(false),
                                    text(app.i18n.format(Key::UnchangedLines, &args))
                                        .size(12)
                                        .wrapping(text::Wrapping::None),
                                ]
                                .spacing(8)
                                .height(height)
                                .align_y(Alignment::Center),
                            )
                            .padding([0, 8])
                            .width(Length::Fill)
                            .height(height)
                            .style(theme::fold_row)
                            .on_press(Message::ExpandUnchanged(app.preview().id, range.clone())),
                        )
                        .id(format!("unchanged-{}-{}", app.preview().id, range.start))
                        .width(Length::Fill)
                        .into();
                    }
                };
                let spans = &document.lines[index];
                let code = rich_text::<(), Message, _, _>(
                    spans
                        .iter()
                        .map(|(content, color)| widget::span(content.as_str()).color(*color))
                        .collect::<Vec<_>>(),
                )
                .font(Font::MONOSPACE)
                .size(size)
                .line_height(1.65)
                .wrapping(text::Wrapping::None);
                let info = document.line_info[index];
                let color = info
                    .change
                    .map(theme::line_change)
                    .unwrap_or(iced::Color::TRANSPARENT);
                let mut line = Row::new()
                    .spacing(10)
                    .height(height)
                    .width(Length::Fill)
                    .push(container(space()).width(3).height(height).style(move |_| {
                        container::Style {
                            background: Some(color.into()),
                            ..Default::default()
                        }
                    }));
                if app.workspace.preferences.line_numbers {
                    line = line.push(
                        text(info.number)
                            .size(size)
                            .line_height(1.65)
                            .font(Font::MONOSPACE)
                            .color(theme::FAINT)
                            .width(46)
                            .align_x(Alignment::End),
                    );
                }
                let sign = match info.change {
                    Some(code::LineChange::Added) => "+",
                    Some(code::LineChange::Modified) => "~",
                    Some(code::LineChange::Removed) => "−",
                    None => "",
                };
                container(
                    line.push(
                        text(sign)
                            .font(Font::MONOSPACE)
                            .size(size)
                            .line_height(1.65)
                            .color(color)
                            .width(12),
                    )
                    .push(code),
                )
                .width(Length::Fill)
                .style(theme::diff_row(info.change))
                .into()
            });
            scrollable(
                container(column![
                    space().height(start as f32 * height),
                    code::rows(
                        (
                            app.preview().id,
                            document.fingerprint,
                            app.preview().preparation,
                            app.preview().folding.revision,
                            app.workspace.preferences.code_size,
                            app.workspace.preferences.line_numbers,
                            app.i18n.text(Key::Loading),
                        ),
                        start..end,
                        lines
                    ),
                    space().height((count - end) as f32 * height)
                ])
                .padding(iced::Padding {
                    top: 0.,
                    right: 0.,
                    bottom: 12.,
                    left: 0.,
                })
                .width(width),
            )
            .direction(scrollable::Direction::Both {
                vertical: scrollable::Scrollbar::default(),
                horizontal: scrollable::Scrollbar::default(),
            })
            .id("source-code")
            .style(theme::scroll)
            .height(Length::Fill)
            .on_scroll(move |viewport| {
                let offset = viewport.absolute_offset();
                Message::CodeScrolled {
                    id: app.preview().id,
                    x: offset.x,
                    y: offset.y,
                }
            })
            .width(Length::Fill)
            .into()
        })
        .into()
    } else {
        let no_project = app.workspace.directory().is_none();
        let has_spaces = !app.workspace.spaces.is_empty();
        let mut content = column![
            text(app.i18n.text(if no_project {
                if has_spaces {
                    Key::ChooseProject
                } else {
                    Key::Greeting
                }
            } else {
                Key::ChooseFile
            }))
            .size(30)
            .font(theme::semibold())
            .align_x(Alignment::Center),
            text(app.i18n.text(if no_project {
                if has_spaces {
                    Key::ChooseProjectDescription
                } else {
                    Key::WelcomeDescription
                }
            } else {
                Key::ChooseFileDescription
            }))
            .size(16)
            .line_height(1.5)
            .color(theme::MUTED)
            .align_x(Alignment::Center),
        ]
        .spacing(16)
        .align_x(Alignment::Center)
        .max_width(520);
        if !no_project || !has_spaces {
            content = content.push(space().height(8)).push(
                button(
                    text(app.i18n.text(if no_project {
                        Key::NewSpace
                    } else {
                        Key::ProjectFiles
                    }))
                    .size(14),
                )
                .on_press_maybe(if no_project {
                    (app.loaded && !app.picking)
                        .then_some(Message::Space(crate::app::spaces::Message::New))
                } else {
                    Some(Message::Navigate(Navigation::Files))
                })
                .padding([12, 24])
                .style(theme::primary),
            );
        }
        container(content).padding(40).center(Length::Fill).into()
    };
    if has_tabs {
        let progress = if app.preview().loading {
            match app.preview().preparation {
                crate::app::tabs::Preparation::Text => 15.,
                crate::app::tabs::Preparation::Syntax => 45.,
                crate::app::tabs::Preparation::Changes => 80.,
            }
        } else {
            0.
        };
        // Keep the scroll widget in the same tree position when preparation finishes.
        let content = widget::stack![
            content,
            widget::progress_bar(0.0..=100., progress)
                .girth(2)
                .style(theme::source_progress)
        ]
        .height(Length::Fill);
        column![file_tabs(app), content].height(Length::Fill).into()
    } else {
        content
    }
}

pub(super) fn icon_button<'a>(
    app: &'a App,
    symbol: Icon,
    label: Key,
    message: Message,
    active: bool,
) -> Element<'a, Message> {
    tooltip(
        button(
            container(icon(
                symbol,
                if active { theme::TEXT } else { theme::MUTED },
            ))
            .center(32),
        )
        .on_press(message)
        .padding(4)
        .style(theme::icon_action(active)),
        text(app.i18n.text(label)).size(12),
        tooltip::Position::Bottom,
    )
    .gap(8)
    .style(theme::tooltip)
    .into()
}

pub(super) fn divider<'a>() -> Element<'a, Message> {
    container(space())
        .height(1)
        .width(Length::Fill)
        .style(theme::separator)
        .into()
}

pub(super) fn center_message(app: &App, key: Key) -> Element<'_, Message> {
    container(
        text(app.i18n.text(key))
            .size(14)
            .color(theme::MUTED)
            .align_x(Alignment::Center),
    )
    .padding(20)
    .center(Length::Fill)
    .into()
}

pub(super) fn short(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        value.to_owned()
    } else {
        format!("{}…", value.chars().take(limit).collect::<String>())
    }
}

pub(super) fn short_path(value: &str, limit: usize) -> String {
    let count = value.chars().count();
    if count <= limit {
        value.to_owned()
    } else {
        format!("…{}", value.chars().skip(count - limit).collect::<String>())
    }
}
