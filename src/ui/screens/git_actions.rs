use crate::{
    app::{
        App, Message,
        git_actions::{self, Action, Message as GitMessage},
    },
    i18n::{FluentArgs, Key},
    ui::{context_menu, ellipsis, theme},
};
use iced::{
    Alignment, Element, Length, Size,
    widget::{
        Column, button, column, container, pick_list, row, space, text, text_input, toggler,
        tooltip,
    },
};

pub(super) fn label(app: &App, action: Action, branch: &str) -> String {
    if action == Action::Create {
        let mut args = FluentArgs::new();
        args.set("branch", branch);
        return app.i18n.format(Key::GitNewBranchFrom, &args);
    }
    app.i18n.text(action_key(action))
}

pub(super) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let base = if let Some((key, branch, point)) = &app.git_actions.menu {
        context_menu::overlay(
            base,
            *point,
            Size::new(330., 282.),
            move || {
                let mut content = Column::new().spacing(2);
                for action in [
                    Action::Checkout,
                    Action::Create,
                    Action::Update,
                    Action::Push,
                    Action::Rename,
                    Action::Delete,
                ] {
                    content = content.push(
                        container(
                            button(ellipsis::text(label(app, action, &branch.name), 14.))
                                .padding([10, 12])
                                .width(Length::Fill)
                                .style(theme::action(false))
                                .on_press_maybe(
                                    (git_actions::allowed(action, branch)
                                        && !app.git_actions.pending)
                                        .then(|| {
                                            Message::Git(GitMessage::Choose(action, key.clone()))
                                        }),
                                ),
                        )
                        .id(format!("git-action-{action:?}")),
                    );
                }
                content.into()
            },
            Message::Git(GitMessage::Close),
        )
    } else {
        base
    };
    let Some(dialog) = &app.git_actions.dialog else {
        return base;
    };
    let mut content = column![
        text(app.i18n.text(action_key(dialog.action)))
            .size(22)
            .font(theme::semibold()),
        tooltip(
            ellipsis::text(&dialog.branch.name, 14.),
            text(&dialog.branch.name).size(12),
            tooltip::Position::Bottom
        )
        .style(theme::tooltip),
    ]
    .spacing(16);
    if app
        .repository
        .as_ref()
        .is_some_and(|repos| repos.repositories.len() > 1)
    {
        content = content.push(
            text(
                dialog
                    .root
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
            )
            .size(13)
            .style(theme::MUTED.text()),
        );
    }
    if dialog.action != Action::Delete {
        content = content
            .push(text(app.i18n.text(Key::GitBranchName)).size(13))
            .push(
                text_input(&app.i18n.text(Key::GitBranchName), &dialog.name)
                    .id("git-branch-name")
                    .on_input(|name| Message::Git(GitMessage::Name(name)))
                    .on_submit(Message::Git(GitMessage::Submit))
                    .padding(12)
                    .size(14)
                    .style(theme::input),
            );
    } else {
        content = content.push(
            text(app.i18n.text(
                if dialog.branch.kind == crate::domain::repository::ReferenceKind::Remote {
                    Key::GitDeleteRemoteDescription
                } else {
                    Key::GitDeleteDescription
                },
            ))
            .size(14),
        );
    }
    if dialog.action == Action::Create {
        content = content.push(
            toggler(dialog.checkout)
                .label(app.i18n.text(Key::GitSwitchAfterCreate))
                .style(theme::toggle)
                .on_toggle(|value| Message::Git(GitMessage::Checkout(value))),
        );
    }
    if dialog.action == Action::Push {
        let remotes = app
            .repository
            .as_ref()
            .and_then(|repos| repos.get(&dialog.root))
            .map(|repo| repo.remotes.clone())
            .unwrap_or_default();
        content = content
            .push(text(app.i18n.text(Key::GitRemote)).size(13))
            .push(
                pick_list(remotes, Some(dialog.remote.clone()), |remote| {
                    Message::Git(GitMessage::Remote(remote))
                })
                .padding(12)
                .text_size(14)
                .style(theme::select)
                .menu_style(theme::select_menu)
                .width(Length::Fill),
            );
    }
    if let Some(error) = dialog.error {
        content = content.push(
            text(app.i18n.text(git_actions::error_key(error)))
                .size(13)
                .style(theme::RED.text()),
        );
    }
    if app.git_actions.pending {
        content = content.push(
            text(app.i18n.text(Key::GitWorking))
                .size(13)
                .style(theme::MUTED.text()),
        );
    }
    content = content.push(
        row![
            space().width(Length::Fill),
            button(text(app.i18n.text(Key::Cancel)).size(14))
                .padding([10, 16])
                .style(theme::secondary)
                .on_press_maybe(
                    (!app.git_actions.pending).then_some(Message::Git(GitMessage::Close))
                ),
            container(
                button(text(app.i18n.text(action_key(dialog.action))).size(14))
                    .padding([10, 16])
                    .style(theme::primary)
                    .on_press_maybe(
                        (!app.git_actions.pending
                            && (dialog.action == Action::Delete || !dialog.name.trim().is_empty())
                            && (dialog.action != Action::Push || !dialog.remote.is_empty()))
                        .then_some(Message::Git(GitMessage::Submit))
                    )
            )
            .id("git-confirm"),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );
    super::modal::overlay(base, content.into(), 560.)
}

fn action_key(action: Action) -> Key {
    match action {
        Action::Checkout => Key::GitCheckout,
        Action::Create => Key::GitNewBranch,
        Action::Update => Key::GitUpdate,
        Action::Push => Key::GitPush,
        Action::Rename => Key::GitRename,
        Action::Delete => Key::GitDelete,
    }
}
