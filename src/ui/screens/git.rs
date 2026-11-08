use super::workspace::{center_message, divider, icon_button, short};
use crate::{
    app::{App, GitPanel, Message, Panel, error_key},
    domain::repository::{Branch, ReferenceKind},
    i18n::Key,
    ui::{
        ellipsis, file_icons, graph,
        icons::{Icon, icon},
        theme,
    },
};
use iced::{
    Alignment, Element, Font, Length,
    widget::{
        Column, Row, button, column, container, pane_grid, responsive, row, scrollable, space,
        text, text_input, tooltip,
    },
};

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let mut header = row![
        icon(Icon::Branch, theme::MUTED),
        text(app.i18n.text(Key::Log))
            .size(14)
            .font(theme::semibold()),
        text_input(&app.i18n.text(Key::SearchCommits), &app.query)
            .on_input(Message::Search)
            .size(13)
            .padding([8, 12])
            .width(240)
            .style(theme::input),
        space().width(Length::Fill),
        icon_button(app, Icon::Refresh, Key::Refresh, Message::Refresh, false),
        icon_button(
            app,
            Icon::Close,
            Key::HidePanel,
            Message::TogglePanel(Panel::Git),
            false
        )
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    if app.loading {
        header = header.push(
            text(app.i18n.text(Key::Loading))
                .size(11)
                .color(theme::FAINT),
        );
    }
    let content: Element<'_, Message> = if let Some(error) = app.repository_error {
        center_message(app, error_key(error))
    } else if let Some(repo) = &app.repository {
        if repo.commits.is_empty() {
            center_message(app, Key::NoCommits)
        } else {
            container(
                pane_grid(&app.git_panes, |_, panel, _| {
                    let id = match panel {
                        GitPanel::Branches => "git-branches",
                        GitPanel::Commits => "git-commits",
                        GitPanel::Files => "git-files",
                    };
                    pane_grid::Content::new(
                        container(responsive(move |size| match panel {
                            GitPanel::Branches => branches(app, size.width),
                            GitPanel::Commits => commits(app),
                            GitPanel::Files => changed_files(app, size.width),
                        }))
                        .id(id)
                        .width(Length::Fill)
                        .height(Length::Fill),
                    )
                })
                .spacing(1)
                .min_size(120)
                .on_resize(6, Message::ResizeGit)
                .style(theme::panes)
                .height(Length::Fill),
            )
            .style(theme::separator)
            .height(Length::Fill)
            .into()
        }
    } else {
        center_message(app, Key::Loading)
    };
    container(column![
        container(header).padding([8, 24]),
        divider(),
        content
    ])
    .height(Length::Fill)
    .style(theme::panel)
    .into()
}

fn branches(app: &App, width: f32) -> Element<'_, Message> {
    let label_limit = ((width - 76.) / 7.).max(6.) as usize;
    let mut items = column![
        text(app.i18n.text(Key::Branches))
            .size(11)
            .color(theme::FAINT),
    ]
    .spacing(6);
    if let Some(repo) = &app.repository {
        for (kind, group, label) in [
            (ReferenceKind::Local, "false", Key::LocalBranches),
            (ReferenceKind::Remote, "true", Key::RemoteBranches),
            (ReferenceKind::Tag, "tags", Key::Tags),
        ] {
            let mut branches: Vec<_> = repo
                .branches
                .iter()
                .filter(|branch| branch.kind == kind)
                .collect();
            branches.sort_by_key(|branch| branch_order(branch));
            if branches.is_empty() {
                continue;
            }
            let group = group.to_owned();
            let collapsed = !app.expanded_branch_folders.contains(&group);
            items = items.push(
                button(
                    row![
                        file_icons::chevron(!collapsed),
                        text(app.i18n.text(label)).size(11).color(theme::FAINT),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::ToggleBranchFolder(group.clone()))
                .width(Length::Fill)
                .padding([6, 4])
                .style(theme::action(false)),
            );
            if collapsed {
                continue;
            }
            let mut previous: Vec<&str> = Vec::new();
            for branch in branches {
                let components: Vec<_> = if branch.current {
                    vec![branch.name.as_str()]
                } else {
                    branch.name.split('/').collect()
                };
                let folders = &components[..components.len().saturating_sub(1)];
                let common = folders
                    .iter()
                    .zip(&previous)
                    .take_while(|(a, b)| a == b)
                    .count();
                let mut hidden = false;
                for (depth, folder) in folders.iter().enumerate() {
                    let visual_depth = depth.min(5);
                    let path = format!("{group}/{}", folders[..=depth].join("/"));
                    let collapsed = !app.expanded_branch_folders.contains(&path);
                    if depth >= common {
                        items = items.push(
                            button(
                                row![
                                    space().width((visual_depth * 10) as f32),
                                    icon(
                                        if collapsed { Icon::Right } else { Icon::Down },
                                        theme::FAINT
                                    ),
                                    icon(Icon::Folder, theme::MUTED),
                                    text(short(
                                        folder,
                                        label_limit.saturating_sub(visual_depth * 2).max(4)
                                    ))
                                    .size(12)
                                ]
                                .spacing(3)
                                .align_y(Alignment::Center),
                            )
                            .on_press(Message::ToggleBranchFolder(path))
                            .width(Length::Fill)
                            .padding([3, 0])
                            .style(theme::action(false)),
                        );
                    }
                    if collapsed {
                        hidden = true;
                        break;
                    }
                }
                previous = folders.to_vec();
                if hidden {
                    continue;
                }
                let active = app.branch.as_ref() == Some(&branch.reference);
                let line = row![
                    space().width((folders.len().min(5) * 10) as f32),
                    icon(
                        if kind == ReferenceKind::Tag {
                            Icon::Tag
                        } else {
                            Icon::Branch
                        },
                        if branch.current {
                            theme::GREEN
                        } else {
                            theme::MUTED
                        }
                    ),
                    text(short(
                        components.last().copied().unwrap_or(&branch.name),
                        label_limit.saturating_sub(folders.len().min(5) * 2).max(4)
                    ))
                    .size(12)
                ]
                .spacing(7)
                .align_y(Alignment::Center);
                items = items.push(
                    tooltip(
                        container(
                            button(line)
                                .width(Length::Fill)
                                .padding([5, 6])
                                .style(theme::action(active))
                                .on_press(Message::SelectBranch(branch.reference.clone())),
                        )
                        .id(format!("branch-{}", branch.reference))
                        .width(Length::Fill),
                        text(&branch.name).size(12),
                        tooltip::Position::Right,
                    )
                    .style(theme::tooltip),
                );
            }
        }
    }
    container(scrollable(items).style(theme::scroll))
        .width(width)
        .height(Length::Fill)
        .padding(16)
        .style(theme::surface)
        .into()
}

fn branch_order(branch: &Branch) -> (bool, Vec<(u8, &str)>) {
    let name = match branch.kind {
        ReferenceKind::Local => Some(branch.name.as_str()),
        ReferenceKind::Remote => branch.name.split_once('/').map(|(_, name)| name),
        ReferenceKind::Tag => None,
    };
    let priority = match name {
        Some("main") => 0,
        Some("master") => 1,
        _ => 2,
    };
    let mut components: Vec<_> = branch.name.split('/').map(|name| (2, name)).collect();
    if let Some(last) = components.last_mut() {
        last.0 = priority;
    }
    (!branch.current, components)
}

fn commits(app: &App) -> Element<'_, Message> {
    responsive(move |size| commit_table(app, size.width, size.height)).into()
}

fn commit_table(app: &App, available_width: f32, available_height: f32) -> Element<'_, Message> {
    let Some(repo) = &app.repository else {
        return space().into();
    };
    let query = app.query.to_lowercase();
    let matching = repo
        .commits
        .iter()
        .filter(|commit| {
            query.is_empty()
                || [
                    &commit.id,
                    &commit.subject,
                    &commit.author,
                    &commit.refs,
                    &commit.date,
                ]
                .iter()
                .any(|value| value.to_lowercase().contains(&query))
        })
        .collect::<Vec<_>>();
    if matching.is_empty() {
        return column![
            center_message(
                app,
                if app.history_loading {
                    Key::LoadingHistory
                } else {
                    Key::NoResults
                }
            ),
            history_footer(app),
        ]
        .height(Length::Fill)
        .into();
    }
    let widest_graph = if query.is_empty() {
        matching
            .iter()
            .map(|commit| graph::width(&commit.graph))
            .fold(0., f32::max)
    } else {
        68.
    };
    let table_width = available_width.max(widest_graph + 450.);
    let start = (((app.history_scroll_y - 28.).max(0.) / 30.) as usize)
        .saturating_sub(8)
        .min(matching.len());
    let end = (start + (available_height / 30.).ceil() as usize + 16).min(matching.len());
    let mut rows = Column::new().push(space().height(start as f32 * 30.));
    for commit in &matching[start..end] {
        let selected = app.selected_commit.as_ref() == Some(&commit.id);
        let graph_width = if query.is_empty() {
            graph::width(&commit.graph)
        } else {
            68.
        };
        let subject_width = table_width - graph_width - 140. - 78. - 36.;
        let graph: Element<'_, Message> = if query.is_empty() {
            graph::view(&commit.graph, graph_width)
        } else {
            text(&commit.id[..7])
                .font(Font::MONOSPACE)
                .size(11)
                .color(theme::BLUE)
                .width(68)
                .into()
        };
        let has_refs = !commit.refs.is_empty() && subject_width > 270.;
        let active = app.active_pipelines(&commit.id);
        let mut subject = Row::new();
        if !active.is_empty() {
            let running = active
                .iter()
                .any(|run| matches!(run.status.as_str(), "running" | "in_progress"));
            let description = format!(
                "{}: {} ({})",
                app.i18n.text(Key::Pipelines),
                app.i18n
                    .text(if running { Key::Running } else { Key::Pending }),
                active.len()
            );
            subject = subject.push(
                tooltip(
                    container(icon(
                        Icon::Play,
                        if running { theme::GREEN } else { theme::YELLOW },
                    ))
                    .id(format!("commit-pipeline-{}", commit.id)),
                    text(description).size(12),
                    tooltip::Position::Top,
                )
                .style(theme::tooltip),
            );
        }
        let mut subject = subject
            .push(ellipsis::text(&commit.subject, 13.))
            .spacing(8)
            .align_y(Alignment::Center)
            .width(subject_width);
        if has_refs {
            subject = subject.push(
                container(
                    text(short(&commit.refs, 15))
                        .size(10)
                        .color(theme::BLUE)
                        .wrapping(text::Wrapping::None),
                )
                .padding([2, 5])
                .style(theme::badge),
            );
        }
        let item = row![
            graph,
            subject,
            text(short(&commit.author, 20))
                .size(11)
                .wrapping(text::Wrapping::None)
                .color(theme::MUTED)
                .width(140),
            text(&commit.date).size(11).color(theme::FAINT).width(78)
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .height(30);
        rows = rows.push(
            tooltip(
                button(item)
                    .on_press(Message::SelectCommit(commit.id.clone()))
                    .width(Length::Fill)
                    .padding([0, 6])
                    .style(theme::action(selected)),
                text(format!(
                    "{}\n{} · {}",
                    commit.subject, commit.author, commit.id
                ))
                .size(12),
                tooltip::Position::Bottom,
            )
            .style(theme::tooltip),
        );
    }
    rows = rows.push(space().height((matching.len() - end) as f32 * 30.));
    let heading = row![
        text(app.i18n.text(Key::Commit))
            .size(10)
            .color(theme::FAINT)
            .width(Length::Fill),
        text(app.i18n.text(Key::Author))
            .size(10)
            .color(theme::FAINT)
            .width(140),
        text(app.i18n.text(Key::Date))
            .size(10)
            .color(theme::FAINT)
            .width(78)
    ]
    .spacing(8)
    .padding([7, 12])
    .height(28);
    container(
        scrollable(
            column![
                heading,
                rows,
                container(history_footer(app)).width(available_width),
                space().height(16),
            ]
            .width(table_width),
        )
        .id("commit-history")
        .on_scroll(|viewport| Message::HistoryScrolled {
            offset: viewport.absolute_offset().y,
            remaining: viewport.absolute_offset_reversed().y,
        })
        .style(theme::scroll)
        .direction(scrollable::Direction::Both {
            vertical: scrollable::Scrollbar::default(),
            horizontal: scrollable::Scrollbar::default(),
        })
        .height(Length::Fill)
        .width(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::surface)
    .into()
}

fn history_footer(app: &App) -> Element<'_, Message> {
    if app.history_loading {
        return container(
            text(app.i18n.text(Key::LoadingHistory))
                .size(12)
                .color(theme::MUTED),
        )
        .padding(12)
        .center_x(Length::Fill)
        .into();
    }
    if app
        .repository
        .as_ref()
        .is_none_or(|repo| repo.next_history.is_none())
    {
        return space().height(0).into();
    }
    let mut content = Column::new().spacing(8).align_x(Alignment::Center);
    if app.history_error.is_some() {
        content = content.push(
            text(app.i18n.text(Key::HistoryLoadFailed))
                .size(12)
                .color(theme::MUTED),
        );
    }
    content = content.push(
        button(
            text(app.i18n.text(if app.history_error.is_some() {
                Key::Retry
            } else {
                Key::LoadOlderCommits
            }))
            .size(12),
        )
        .on_press_maybe((!app.loading).then_some(Message::LoadMoreHistory))
        .style(theme::secondary)
        .padding([8, 16]),
    );
    container(content).padding(12).center_x(Length::Fill).into()
}

fn changed_files(app: &App, width: f32) -> Element<'_, Message> {
    container(responsive(move |size| {
        changed_files_content(app, size.height)
    }))
    .width(width)
    .height(Length::Fill)
    .padding(16)
    .style(theme::surface)
    .into()
}

fn changed_files_content(app: &App, height: f32) -> Element<'_, Message> {
    let heading = row![
        text(app.i18n.text(Key::ChangedFiles))
            .size(11)
            .color(theme::MUTED)
            .width(Length::Fill),
        text(app.changed_files.len()).size(11).color(theme::FAINT),
        container(
            tooltip(
                button(icon(Icon::Close, theme::MUTED))
                    .padding(4)
                    .width(28)
                    .height(28)
                    .style(theme::action(false))
                    .on_press(Message::CloseCommitFiles),
                text(app.i18n.text(Key::HidePanel)).size(12),
                tooltip::Position::Left,
            )
            .style(theme::tooltip)
        )
        .id("close-commit-files"),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let mut files = Column::new().spacing(2);
    let mut sorted: Vec<_> = app.changed_files.iter().enumerate().collect();
    sorted.sort_by(|(_, a), (_, b)| a.path.cmp(&b.path));
    let mut previous: Vec<&str> = Vec::new();
    for (index, file) in sorted {
        let components: Vec<_> = file.path.split('/').collect();
        let folders = &components[..components.len().saturating_sub(1)];
        let common = folders
            .iter()
            .zip(&previous)
            .take_while(|(a, b)| a == b)
            .count();
        let mut hidden = false;
        for (depth, folder) in folders.iter().enumerate() {
            let path = folders[..=depth].join("/");
            let collapsed = app.collapsed_commit_folders.contains(&path);
            if depth >= common {
                files = files.push(
                    button(
                        row![
                            space().width((depth.min(5) * 8) as f32),
                            file_icons::chevron(!collapsed),
                            file_icons::folder(!collapsed, false),
                            text(short(folder, 20usize.saturating_sub(depth.min(5) * 2)))
                                .size(11)
                                .color(theme::MUTED)
                        ]
                        .spacing(3)
                        .align_y(Alignment::Center)
                        .height(22),
                    )
                    .on_press(Message::ToggleCommitFolder(path))
                    .padding(0)
                    .width(Length::Fill)
                    .style(theme::action(false)),
                );
            }
            if collapsed {
                hidden = true;
                break;
            }
        }
        previous = folders.to_vec();
        if hidden {
            continue;
        }
        let color = match file.status {
            'A' => theme::GREEN,
            'D' => theme::RED,
            _ => theme::BLUE,
        };
        let item = row![
            space().width((folders.len().min(5) * 8) as f32),
            text(file.status.to_string())
                .font(Font::MONOSPACE)
                .size(11)
                .color(color)
                .width(18),
            file_icons::file(std::path::Path::new(&file.path)),
            text(short(
                components.last().copied().unwrap_or(&file.path),
                24usize.saturating_sub(folders.len().min(5) * 2)
            ))
            .size(12)
            .color(color)
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        files = files.push(
            tooltip(
                button(item)
                    .on_press(Message::OpenCommitFile(index))
                    .padding([4, 4])
                    .width(Length::Fill)
                    .style(theme::action(false)),
                text(&file.path).size(12),
                tooltip::Position::Left,
            )
            .style(theme::tooltip),
        );
    }
    if app.changes_loading {
        files = files.push(
            text(app.i18n.text(Key::Loading))
                .size(12)
                .color(theme::MUTED),
        );
    }
    if let Some(error) = app.changes_error {
        files = files.push(
            text(app.i18n.text(error_key(error)))
                .size(12)
                .color(theme::YELLOW),
        );
    }
    let mut content = column![
        heading,
        divider(),
        scrollable(files)
            .style(theme::scroll)
            .height(Length::Fill)
            .width(Length::Fill)
    ]
    .spacing(6);
    if height >= 175.
        && let Some(commit) = app.repository.as_ref().and_then(|repo| {
            repo.commits
                .iter()
                .find(|commit| Some(&commit.id) == app.selected_commit.as_ref())
        })
    {
        content = content
            .push(divider())
            .push(text(short(&commit.subject, 32)).size(11))
            .push(
                text(format!("{} · {}", &commit.id[..8], commit.date))
                    .font(Font::MONOSPACE)
                    .size(10)
                    .color(theme::FAINT),
            );
    }
    content.height(Length::Fill).into()
}
