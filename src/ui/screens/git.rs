use super::workspace::{center_message, divider, icon_button, short};
use crate::ui::tooltip;
use crate::{
    app::{
        App, GitPanel, Message,
        branch_selection::{Event as SelectionEvent, Target as SelectionTarget},
        error_key,
    },
    domain::repository::{Branch, ReferenceKind, project::BranchKey},
    i18n::Key,
    ui::{
        changed_tree, context_menu, ellipsis, file_icons, graph,
        icons::{Icon, icon},
        theme,
    },
};
use iced::{
    Alignment, Element, Font, Length, Size,
    widget::{
        Column, Row, button, column, container, pane_grid, responsive, row, scrollable, space,
        text, text_input,
    },
};

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let content: Element<'_, Message> = if let Some(error) = app.repository_error {
        center_message(app, error_key(error))
    } else if app.repository.is_some() {
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
            .spacing(theme::GIT_PANE_SPACING)
            .min_size(theme::GIT_PANE_MIN_SIZE)
            .on_resize(6, Message::ResizeGit)
            .style(theme::panes)
            .height(Length::Fill),
        )
        .style(theme::separator)
        .height(Length::Fill)
        .into()
    } else {
        center_message(app, Key::Loading)
    };
    let tools = container(
        scrollable(
            column![
                container(icon_button(
                    app,
                    Icon::Refresh,
                    Key::Refresh,
                    Message::Refresh,
                    false
                ))
                .id("git-refresh"),
                branch_tools(app),
            ]
            .spacing(theme::FILE_PANEL_TOOL_INSET)
            .width(theme::FILE_PANEL_TOOL_SIZE),
        )
        .id("git-toolbar-scroll")
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::hidden(),
        ))
        .width(theme::FILE_PANEL_TOOL_SIZE)
        .height(Length::Fill)
        .style(theme::scroll),
    )
    .id("git-toolbar")
    .padding(theme::FILE_PANEL_TOOL_INSET)
    .height(Length::Fill)
    .style(theme::sidebar);
    let separator = container(space())
        .width(1)
        .height(Length::Fill)
        .style(theme::separator);
    let content = row![tools, separator, content].height(Length::Fill);
    container(column![
        responsive(move |size| header(app, size.width)).height(Length::Shrink),
        divider(),
        content
    ])
    .height(Length::Fill)
    .style(theme::panel)
    .into()
}

fn header(app: &App, width: f32) -> Element<'_, Message> {
    let toolbar_width = theme::FILE_PANEL_TOOL_SIZE + 2. * theme::FILE_PANEL_TOOL_INSET + 1.;
    let regions = app.git_panes.layout().pane_regions(
        theme::GIT_PANE_SPACING,
        theme::GIT_PANE_MIN_SIZE,
        Size::new((width - toolbar_width).max(0.), 1.),
    );
    let commits = app
        .git_panes
        .iter()
        .find_map(|(pane, panel)| {
            matches!(panel, GitPanel::Commits)
                .then(|| regions.get(pane))
                .flatten()
        })
        .copied()
        .unwrap_or_default();
    let commits_x = commits.x + toolbar_width;
    let status = if app.git_actions.pending {
        Some(Key::GitWorking)
    } else {
        app.loading.then_some(Key::Loading)
    };
    let mut controls = Row::new().spacing(12).align_y(Alignment::Center);
    if let Some(status) = status {
        controls = controls.push(container(ellipsis::text(app.i18n.text(status), 11.)).width(132));
    }
    let controls_width = if status.is_some() { 144. } else { 0. };
    // Keep search usable when the branches column occupies most of the panel.
    let controls_on_left = width - commits_x < controls_width + 136.;
    let mut title = row![
        icon(Icon::Branch, theme::MUTED),
        text(app.i18n.text(Key::Log))
            .size(14)
            .font(theme::semibold())
            .wrapping(text::Wrapping::None),
        space().width(Length::Fill),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let mut controls = Some(controls);
    if controls_on_left {
        title = title.push(controls.take().unwrap());
    }
    let title = container(title)
        .width(commits_x)
        .height(56)
        .padding(iced::Padding {
            top: 8.,
            bottom: 8.,
            left: 24.,
            right: 8.,
        })
        .align_y(Alignment::Center);
    let reserved = if controls_on_left {
        0.
    } else {
        controls_width + 24.
    };
    let search_width = (commits.width - 24.)
        .min(width - commits_x - reserved - 24.)
        .clamp(0., 560.);
    let search = container(
        text_input(&app.i18n.text(Key::SearchCommits), &app.query)
            .id("git-search")
            .on_input(Message::Search)
            .size(13)
            .padding([8, 12])
            .width(search_width)
            .style(theme::input),
    )
    .padding([8, 12]);
    let mut header = row![title, search, space().width(Length::Fill)].align_y(Alignment::Center);
    if let Some(controls) = controls {
        header = header.push(container(controls).padding(iced::Padding {
            right: 24.,
            ..iced::Padding::ZERO
        }));
    }
    header.into()
}

fn selectable_group<'a>(
    app: &App,
    target: SelectionTarget,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let menu_target = target.clone();
    let selectable = app
        .repository
        .as_ref()
        .is_some_and(|repos| !target.keys(repos).is_empty());
    context_menu::trigger(
        crate::ui::branch_selection::target(
            content,
            app.branch_selection.command() && selectable,
            target,
        ),
        move |point| Message::BranchSelection(SelectionEvent::Menu(menu_target.clone(), point)),
    )
}

fn group_selected(app: &App, target: &SelectionTarget) -> bool {
    app.repository
        .as_ref()
        .is_some_and(|repos| app.branch_selection.covers(&target.keys(repos)))
}

fn branch_tools(app: &App) -> Element<'_, Message> {
    use crate::app::git_actions::{Action, Message as GitMessage};
    let selected = app.branch.as_ref().and_then(|key| {
        app.repository
            .as_ref()?
            .get(&key.root)?
            .branches
            .iter()
            .find(|branch| branch.reference == key.reference)
            .map(|branch| (key, branch))
    });
    let tool_size = theme::FILE_PANEL_TOOL_SIZE;
    let mut tools = Column::new().spacing(theme::FILE_PANEL_TOOL_INSET);
    for (id, symbol, key, message) in [
        (
            "git-fetch",
            Icon::Fetch,
            Key::GitFetchAll,
            Some(Message::Git(GitMessage::Fetch)),
        ),
        (
            "git-new-branch",
            Icon::Plus,
            Key::GitNewBranch,
            selected.map(|(key, _)| Message::Git(GitMessage::Choose(Action::Create, key.clone()))),
        ),
        (
            "git-update",
            Icon::UpdateBranch,
            Key::GitUpdateSelected,
            selected
                .filter(|(_, branch)| branch.kind != ReferenceKind::Tag)
                .map(|(key, _)| Message::Git(GitMessage::Choose(Action::Update, key.clone()))),
        ),
        (
            "git-push",
            Icon::Upload,
            Key::GitPush,
            selected
                .filter(|(_, branch)| branch.kind == ReferenceKind::Local)
                .map(|(key, _)| Message::Git(GitMessage::Choose(Action::Push, key.clone()))),
        ),
    ] {
        tools = tools.push(tooltip::view(
            container(
                button(container(icon(symbol, theme::MUTED)).center(Length::Fill))
                    .width(tool_size)
                    .height(tool_size)
                    .padding(0)
                    .style(theme::action(false))
                    .on_press_maybe(message.filter(|_| !app.git_actions.pending)),
            )
            .id(id),
            text(app.i18n.text(key)).size(12),
            tooltip::Position::Right,
        ));
    }
    container(tools)
        .id("git-branch-tools")
        .width(theme::FILE_PANEL_TOOL_SIZE)
        .into()
}

fn branches(app: &App, width: f32) -> Element<'_, Message> {
    let mut items = Column::new().spacing(6).width(Length::Fill);
    if let Some(repos) = &app.repository {
        let multiple = repos.repositories.len() > 1;
        if multiple {
            items = items.push(
                button(text(app.i18n.text(Key::AllRepositories)).size(12))
                    .on_press(Message::AllRepositories)
                    .padding([6, 8])
                    .width(Length::Fill)
                    .style(theme::action(app.branch.is_none())),
            );
        }
        for (index, repo) in repos.repositories.iter().enumerate() {
            let indent = if multiple { 10. } else { 0. };
            if multiple {
                let group = format!("repo:{}", repo.root.display());
                let expanded = app.expanded_branch_folders.contains(&group);
                let target = SelectionTarget::Group {
                    root: repo.root.clone(),
                    kind: None,
                    prefix: None,
                };
                let selected = group_selected(app, &target);
                items = items.push(selectable_group(
                    app,
                    target,
                    button(
                        row![
                            file_icons::chevron(expanded),
                            icon(Icon::Branch, theme::repository_color(index)),
                            ellipsis::text(
                                repo.root
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .into_owned(),
                                12.
                            )
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    )
                    .padding([6, 4])
                    .width(Length::Fill)
                    .style(theme::action(selected))
                    .on_press(Message::ToggleBranchFolder(group)),
                ));
                if !expanded {
                    continue;
                }
            }
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
                let group = if multiple {
                    format!("{}:{group}", repo.root.display())
                } else {
                    group.to_owned()
                };
                let collapsed = !app.expanded_branch_folders.contains(&group);
                let target = SelectionTarget::Group {
                    root: repo.root.clone(),
                    kind: Some(kind),
                    prefix: None,
                };
                let selected = group_selected(app, &target);
                items = items.push(selectable_group(
                    app,
                    target,
                    button(
                        row![
                            space().width(indent),
                            file_icons::chevron(!collapsed),
                            container(ellipsis::text(app.i18n.text(label), 11.)).style(|theme| {
                                container::Style {
                                    text_color: Some(theme::FAINT.resolve(theme)),
                                    ..Default::default()
                                }
                            }),
                        ]
                        .spacing(4)
                        .align_y(Alignment::Center),
                    )
                    .on_press(Message::ToggleBranchFolder(group.clone()))
                    .width(Length::Fill)
                    .padding([6, 4])
                    .style(theme::action(selected)),
                ));
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
                            let target = SelectionTarget::Group {
                                root: repo.root.clone(),
                                kind: Some(kind),
                                prefix: Some(format!("{}/", folders[..=depth].join("/"))),
                            };
                            let selected = group_selected(app, &target);
                            items = items.push(selectable_group(
                                app,
                                target,
                                button(
                                    row![
                                        space().width(indent + (visual_depth * 10) as f32),
                                        icon(
                                            if collapsed { Icon::Right } else { Icon::Down },
                                            theme::FAINT
                                        ),
                                        icon(Icon::Folder, theme::MUTED),
                                        ellipsis::text(*folder, 12.)
                                    ]
                                    .spacing(3)
                                    .align_y(Alignment::Center),
                                )
                                .on_press(Message::ToggleBranchFolder(path))
                                .width(Length::Fill)
                                .padding([3, 0])
                                .style(theme::action(selected)),
                            ));
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
                    let key = BranchKey {
                        root: repo.root.clone(),
                        reference: branch.reference.clone(),
                    };
                    let active = if app.branch_selection.selected.is_empty() {
                        app.branch.as_ref() == Some(&key)
                    } else {
                        app.branch_selection.selected.contains(&key)
                    };
                    let line = row![
                        space().width(indent + (folders.len().min(5) * 10) as f32),
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
                        ellipsis::text(components.last().copied().unwrap_or(&branch.name), 12.)
                    ]
                    .spacing(7)
                    .align_y(Alignment::Center);
                    items = items.push(tooltip::view(
                        context_menu::trigger(
                            crate::ui::branch_selection::target(
                                container(
                                    button(line)
                                        .width(Length::Fill)
                                        .padding([5, 6])
                                        .style(theme::action(active))
                                        .on_press(Message::SelectBranch(key.clone())),
                                )
                                .id(if multiple {
                                    format!("branch-{}-{}", repo.root.display(), branch.reference)
                                } else {
                                    format!("branch-{}", branch.reference)
                                })
                                .width(Length::Fill),
                                app.branch_selection.command() && kind != ReferenceKind::Tag,
                                SelectionTarget::Branch(key.clone()),
                            ),
                            move |position| {
                                Message::Git(crate::app::git_actions::Message::Menu(
                                    key.clone(),
                                    position,
                                ))
                            },
                        ),
                        text(&branch.name).size(12),
                        tooltip::Position::Right,
                    ));
                }
            }
        }
        for (root, error) in &repos.failures {
            items = items.push(
                text(format!(
                    "{} · {}",
                    root.file_name().unwrap_or_default().to_string_lossy(),
                    app.i18n.text(error_key(*error))
                ))
                .size(11)
                .style(theme::YELLOW.text()),
            );
        }
    }
    let header = container(
        text(app.i18n.text(Key::Branches))
            .size(11)
            .style(theme::FAINT.text()),
    )
    .id("git-branches-heading")
    .width(Length::Fill)
    .padding(theme::BRANCH_HEADER_INSET);
    let list = scrollable(container(items).padding(iced::Padding {
        left: theme::BRANCH_HEADER_INSET,
        ..iced::Padding::ZERO
    }))
    .id("git-branch-scroll")
    .direction(scrollable::Direction::Vertical(
        scrollable::Scrollbar::default().spacing(0),
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::scroll);
    let mut contents = column![header, list];
    if !app.branch_selection.selected.is_empty() {
        contents = contents.push(column![
            divider(),
            container(
                text(super::git_bulk::count(
                    app,
                    app.branch_selection.selected.len()
                ))
                .size(12)
                .style(theme::MUTED.text())
            )
            .id("branch-selection-count")
            .padding([8, 12])
            .width(Length::Fill),
        ]);
    }
    let content = container(contents)
        .width(width)
        .height(Length::Fill)
        .style(theme::surface);
    iced::widget::stack![
        content,
        container(space())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::branch_attention(app.branch_attention.is_some())),
    ]
    .into()
}

fn branch_order(branch: &Branch) -> (bool, Vec<(bool, u8, &str)>) {
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
    let parts: Vec<_> = branch.name.split('/').collect();
    let components = parts
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let folder = index + 1 < parts.len();
            (folder, if folder { 2 } else { priority }, *name)
        })
        .collect();
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
        .filter(|entry| {
            let commit = &entry.commit;
            query.is_empty()
                || entry
                    .root
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase()
                    .contains(&query)
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
                } else if repo.commits.is_empty() {
                    Key::NoCommits
                } else {
                    Key::NoResults
                }
            ),
            history_footer(app),
        ]
        .height(Length::Fill)
        .into();
    }
    let graph_padding = if repo.repositories.len() > 1 {
        theme::COMMIT_REPOSITORY_GRAPH_PADDING
    } else {
        theme::COMMIT_GRAPH_PADDING
    };
    let widest_graph = if query.is_empty() {
        matching
            .iter()
            .map(|commit| graph::width(&commit.commit.graph, graph_padding))
            .fold(0., f32::max)
    } else {
        68.
    };
    let table_width = available_width
        .max(widest_graph + 372. + theme::COMMIT_DATE_WIDTH + theme::COMMIT_HASH_WIDTH + 8.);
    let start = (((app.history_scroll_y - 28.).max(0.) / 30.) as usize)
        .saturating_sub(8)
        .min(matching.len());
    let end = (start + (available_height / 30.).ceil() as usize + 16).min(matching.len());
    let mut rows = Column::new().push(space().height(start as f32 * 30.));
    for entry in &matching[start..end] {
        let commit = &entry.commit;
        let key = entry.key();
        let selected = app.selected_commit.as_ref() == Some(&key);
        let graph_width = if query.is_empty() {
            graph::width(&commit.graph, graph_padding)
        } else {
            68.
        };
        let subject_width = table_width - graph_width - 140. - theme::COMMIT_DATE_WIDTH - 36.;
        let graph: Element<'_, Message> = if query.is_empty() {
            container(graph::view(&commit.graph, graph_width))
                .id(format!(
                    "commit-graph-{}-{}",
                    entry.root.display(),
                    commit.id
                ))
                .into()
        } else {
            text(&commit.id[..7])
                .font(Font::MONOSPACE)
                .size(11)
                .style(theme::BLUE.text())
                .width(68)
                .into()
        };
        let reference_space = 270.
            + theme::COMMIT_HASH_WIDTH
            + if repo.repositories.len() > 1 {
                theme::COMMIT_REPOSITORY_MAX_WIDTH + 8.
            } else {
                0.
            };
        let has_refs = !commit.refs.is_empty() && subject_width > reference_space;
        let active = if entry.root == repo.active_root {
            app.active_pipelines(&commit.id)
        } else {
            Vec::new()
        };
        let mut subject = Row::new();
        if repo.repositories.len() > 1 {
            let index = repo
                .repositories
                .iter()
                .position(|member| member.root == entry.root)
                .unwrap_or_default();
            subject = subject.push(tooltip::view(
                container(ellipsis::compact_text(
                    entry
                        .root
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    10.,
                ))
                .id(format!(
                    "commit-repository-{}-{}",
                    entry.root.display(),
                    commit.id
                ))
                .max_width(theme::COMMIT_REPOSITORY_MAX_WIDTH)
                .padding([2, 5])
                .style(move |theme| iced::widget::container::Style {
                    text_color: Some(theme::repository_color(index).resolve(theme)),
                    ..theme::badge(theme)
                }),
                text(entry.root.display().to_string()).size(12),
                tooltip::Position::Top,
            ));
        }
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
            subject = subject.push(tooltip::view(
                container(icon(
                    Icon::Play,
                    if running { theme::GREEN } else { theme::YELLOW },
                ))
                .id(format!("commit-pipeline-{}", commit.id)),
                text(description).size(12),
                tooltip::Position::Top,
            ));
        }
        let hash = tooltip::view(
            container(
                button(
                    text(commit.id.chars().take(7).collect::<String>())
                        .font(Font::MONOSPACE)
                        .size(11)
                        .style(theme::FAINT.text())
                        .wrapping(text::Wrapping::None),
                )
                .on_press(Message::CopyCommitId(commit.id.clone()))
                .padding([3, 4])
                .width(theme::COMMIT_HASH_WIDTH)
                .style(theme::action(false)),
            )
            .id(format!(
                "copy-commit-id-{}-{}",
                entry.root.display(),
                commit.id
            )),
            column![
                text(app.i18n.text(Key::CopyCommitId)).size(12),
                text(&commit.id).size(11).font(Font::MONOSPACE),
            ]
            .spacing(4),
            tooltip::Position::Bottom,
        );
        let natural_width =
            ellipsis::measured_width(&commit.subject, 13.).ceil() + theme::COMMIT_HASH_WIDTH + 8.;
        let title = container(
            container(
                row![
                    tooltip::view(
                        ellipsis::text(&commit.subject, 13.),
                        text(format!(
                            "{}\n{} · {}",
                            commit.subject, commit.author, commit.id
                        ))
                        .size(12),
                        tooltip::Position::Bottom,
                    ),
                    hash
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::Fill),
            )
            .width(Length::Fill)
            .max_width(natural_width),
        )
        .width(Length::Fill);
        let mut subject = subject
            .push(title)
            .spacing(8)
            .align_y(Alignment::Center)
            .width(subject_width);
        if has_refs {
            subject = subject.push(
                container(
                    text(short(&commit.refs, 15))
                        .size(10)
                        .style(theme::BLUE.text())
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
                .style(theme::MUTED.text())
                .width(140),
            text(&commit.date)
                .size(11)
                .style(theme::FAINT.text())
                .wrapping(text::Wrapping::None)
                .width(theme::COMMIT_DATE_WIDTH)
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .height(30);
        rows = rows.push(
            button(item)
                .on_press(Message::SelectCommit(key))
                .width(Length::Fill)
                .padding([0, 6])
                .style(theme::action(selected)),
        );
    }
    rows = rows.push(space().height((matching.len() - end) as f32 * 30.));
    let heading = row![
        text(app.i18n.text(Key::Commit))
            .size(10)
            .style(theme::FAINT.text())
            .width(Length::Fill),
        text(app.i18n.text(Key::Author))
            .size(10)
            .style(theme::FAINT.text())
            .width(140),
        text(app.i18n.text(Key::Date))
            .size(10)
            .style(theme::FAINT.text())
            .width(theme::COMMIT_DATE_WIDTH)
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
                .style(theme::MUTED.text()),
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
                .style(theme::MUTED.text()),
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
            .style(theme::MUTED.text())
            .width(Length::Fill),
        text(app.changed_files.len())
            .size(11)
            .style(theme::FAINT.text()),
        container(tooltip::view(
            button(icon(Icon::Close, theme::MUTED))
                .padding(4)
                .width(28)
                .height(28)
                .style(theme::action(false))
                .on_press(Message::CloseCommitFiles),
            text(app.i18n.text(Key::HidePanel)).size(12),
            tooltip::Position::Left,
        ))
        .id("close-commit-files"),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let entries = app
        .changed_files
        .iter()
        .enumerate()
        .map(|(index, file)| changed_tree::Entry {
            path: file.path.clone(),
            status: crate::domain::changes::FileStatus::from_git(file.status),
            open: Message::OpenCommitFile(index),
        })
        .collect();
    let mut files = Column::new().push(changed_tree::view(
        entries,
        &app.collapsed_commit_folders,
        &app.i18n,
        Message::ToggleCommitFolder,
    ));
    if app.changes_loading {
        files = files.push(
            text(app.i18n.text(Key::Loading))
                .size(12)
                .style(theme::MUTED.text()),
        );
    }
    if let Some(error) = app.changes_error {
        files = files.push(
            text(app.i18n.text(error_key(error)))
                .size(12)
                .style(theme::YELLOW.text()),
        );
    }
    let mut content = column![
        heading,
        divider(),
        scrollable(files)
            .direction(scrollable::Direction::Both {
                vertical: scrollable::Scrollbar::default(),
                horizontal: scrollable::Scrollbar::default()
            })
            .style(theme::scroll)
            .height(Length::Fill)
            .width(Length::Fill)
    ]
    .spacing(6);
    if height >= 175.
        && let Some(commit) = app.repository.as_ref().and_then(|repo| {
            app.selected_commit
                .as_ref()
                .and_then(|key| repo.commit(key))
        })
    {
        content = content
            .push(divider())
            .push(text(short(&commit.subject, 32)).size(11))
            .push(
                text(format!("{} · {}", &commit.id[..8], commit.date))
                    .font(Font::MONOSPACE)
                    .size(10)
                    .style(theme::FAINT.text()),
            );
    }
    content.height(Length::Fill).into()
}
