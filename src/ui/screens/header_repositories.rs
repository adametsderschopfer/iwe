//! Show project repository heads without hiding additional repositories.

use crate::{
    app::{App, Message},
    domain::repository::Repository,
    i18n::Key,
    ui::{
        context_menu, ellipsis,
        icons::{Icon, icon},
        theme, tooltip,
    },
};
use iced::{
    Alignment, Element, Length, Size,
    widget::{Column, Row, button, column, container, row, scrollable, text},
};

fn head(app: &App, repo: &Repository) -> String {
    if repo.head.is_empty() {
        app.i18n.text(Key::DetachedHead)
    } else {
        repo.head.clone()
    }
}

pub(super) fn controls(app: &App) -> Element<'_, Message> {
    let Some(repos) = app.repository.as_ref() else {
        return Row::new().into();
    };
    let multiple = repos.repositories.len() > 1;
    let mut badges = Row::new().spacing(8).align_y(Alignment::Center);
    for (index, repo) in repos.repositories.iter().take(2).enumerate() {
        let branch = head(app, repo);
        let label = if multiple {
            format!(
                "{}: {}",
                super::workspace::short(
                    &repo.root.file_name().unwrap_or_default().to_string_lossy(),
                    14
                ),
                super::workspace::short(&branch, 18)
            )
        } else {
            super::workspace::short(&branch, 24)
        };
        let root = repo.root.clone();
        badges = badges.push(tooltip::view(
            container(
                button(
                    row![
                        icon(Icon::Branch, theme::MUTED),
                        ellipsis::compact_text(label, 12.)
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .on_press(if multiple {
                    Message::RevealRepositoryBranches(root)
                } else {
                    Message::RevealGitBranches
                })
                .padding([4, 10])
                .style(theme::branch_badge),
            )
            .max_width(theme::HEADER_REPOSITORY_WIDTH)
            .id(if index == 0 {
                "window-branch-button".to_owned()
            } else {
                format!("window-branch-button-{index}")
            }),
            column![
                text(repo.root.display().to_string()).size(12),
                text(branch).size(12),
                text(app.i18n.text(Key::ShowBranches)).size(12)
            ]
            .spacing(3),
            tooltip::Position::Bottom,
        ));
    }
    if repos.repositories.len() > 2 {
        badges = badges.push(tooltip::view(
            context_menu::anchored(
                container(
                    button(icon(Icon::Down, theme::MUTED))
                        .padding([4, 8])
                        .style(theme::branch_badge),
                )
                .id("window-repositories-menu"),
                Message::OpenRepositoryMenu,
            ),
            text(app.i18n.text(Key::ProjectRepositories)).size(12),
            tooltip::Position::Bottom,
        ));
    }
    badges.into()
}

pub(super) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let (Some(point), Some(repos)) = (app.repository_menu, app.repository.as_ref()) else {
        return base;
    };
    let count = repos.repositories.len().saturating_sub(2);
    if count == 0 {
        return base;
    }
    let height = (count as f32 * theme::HEADER_REPOSITORY_ROW_HEIGHT)
        .min(theme::HEADER_REPOSITORY_MENU_HEIGHT);
    context_menu::overlay(
        base,
        point,
        Size::new(theme::HEADER_REPOSITORY_MENU_WIDTH, height + 16.),
        move || {
            let mut items = Column::new();
            for (index, repo) in repos.repositories.iter().enumerate().skip(2) {
                let name = repo.root.file_name().unwrap_or_default().to_string_lossy();
                let branch = head(app, repo);
                items = items.push(tooltip::view(
                    container(
                        button(
                            row![
                                icon(Icon::Branch, theme::MUTED),
                                column![
                                    ellipsis::text(name.into_owned(), 13.),
                                    ellipsis::text(branch.clone(), 12.)
                                ]
                                .spacing(3)
                                .width(Length::Fill)
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center),
                        )
                        .on_press(Message::RevealRepositoryBranches(repo.root.clone()))
                        .padding([6, 10])
                        .width(Length::Fill)
                        .height(theme::HEADER_REPOSITORY_ROW_HEIGHT)
                        .style(theme::action(repos.active_root == repo.root)),
                    )
                    .id(format!("header-repository-menu-{index}")),
                    column![
                        text(repo.root.display().to_string()).size(12),
                        text(branch).size(12)
                    ]
                    .spacing(3),
                    tooltip::Position::Left,
                ));
            }
            scrollable(items)
                .spacing(4)
                .height(height)
                .width(Length::Fill)
                .style(theme::scroll)
                .into()
        },
        Message::CloseRepositoryMenu,
    )
}
