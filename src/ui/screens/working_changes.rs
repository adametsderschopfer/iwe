//! Group working changes by repository without mixing identical relative paths.

use crate::{
    app::{App, Message, error_key},
    i18n::{FluentArgs, Key},
    ui::{
        changed_tree, file_icons,
        icons::{Icon, icon},
        theme,
    },
};
use iced::{
    Alignment, Element, Length,
    widget::{Column, button, column, container, row, text},
};
use std::{collections::BTreeSet, path::Path};

pub(super) fn view(app: &App, minimum_width: f32) -> Element<'_, Message> {
    let Some(directory) = app.workspace.directory() else {
        return Column::new().into();
    };
    let status = &app.changes.status;
    let grouped =
        status.roots.len() > 1 || status.roots.first().is_some_and(|root| root != directory);
    let mut content = Column::new().spacing(8);
    if let Some(error) = app.changes.error {
        content = content.push(
            text(app.i18n.text(error_key(error)))
                .size(13)
                .style(theme::MUTED.text()),
        );
    }
    if !app.changes.loaded {
        return content
            .push(
                text(app.i18n.text(Key::Loading))
                    .size(13)
                    .style(theme::MUTED.text()),
            )
            .into();
    }
    if !grouped {
        if status.files.is_empty() {
            content = content.push(
                text(app.i18n.text(Key::NoWorkingChanges))
                    .size(13)
                    .style(theme::MUTED.text()),
            );
        } else {
            content = content.push(tree(app, directory, "", minimum_width));
        }
    } else {
        for (index, root) in status.roots.iter().enumerate() {
            let prefix = format!("repo:{}:", root.display());
            let expanded = !app.collapsed_worktree_folders.contains(&prefix);
            let count = status
                .files
                .keys()
                .filter(|path| owner(app, path) == Some(root.as_path()))
                .count();
            let mut args = FluentArgs::new();
            args.set("count", count);
            let heading = button(
                row![
                    file_icons::chevron(expanded),
                    icon(Icon::Branch, theme::repository_color(index)),
                    text(
                        root.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned()
                    )
                    .size(14)
                    .style(theme::repository_color(index).text())
                    .wrapping(text::Wrapping::None),
                    text(app.i18n.format(Key::FileCount, &args))
                        .size(12)
                        .style(theme::FAINT.text())
                        .wrapping(text::Wrapping::None),
                ]
                .spacing(5)
                .align_y(Alignment::Center)
                .height(theme::FILE_TREE_ROW_HEIGHT),
            )
            .padding([0, 4])
            .style(theme::tree_item(false))
            .on_press(Message::ToggleWorktreeFolder {
                repository: Some(root.clone()),
                path: String::new(),
            });
            let mut group =
                column![container(heading).id(format!("changes-repository-{}", root.display()))];
            if expanded {
                let children: Element<'_, Message> = if count == 0 {
                    container(
                        text(app.i18n.text(Key::NoWorkingChanges))
                            .size(12)
                            .style(theme::FAINT.text()),
                    )
                    .padding([4, 22])
                    .into()
                } else {
                    tree(app, root, &prefix, minimum_width)
                };
                group = group.push(children);
            }
            content = content.push(group);
        }
    }
    for (root, error) in &status.failures {
        content = content.push(
            text(format!(
                "{} · {}",
                root.file_name().unwrap_or_default().to_string_lossy(),
                app.i18n.text(error_key(*error))
            ))
            .size(12)
            .style(theme::YELLOW.text()),
        );
    }
    content.width(Length::Shrink).into()
}

fn owner<'a>(app: &'a App, path: &Path) -> Option<&'a Path> {
    app.changes
        .status
        .roots
        .iter()
        .filter(|root| path.starts_with(root))
        .max_by_key(|root| root.components().count())
        .map(|root| root.as_path())
}

fn tree<'a>(app: &'a App, root: &Path, prefix: &str, minimum_width: f32) -> Element<'a, Message> {
    let entries = app
        .changes
        .status
        .files
        .iter()
        .filter(|(path, _)| prefix.is_empty() || owner(app, path) == Some(root))
        .map(|(path, change)| changed_tree::Entry {
            path: path
                .strip_prefix(root)
                .unwrap_or(path)
                .components()
                .map(|part| part.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/"),
            status: change.status,
            selected: app.preview().revision.is_none() && app.preview().path.as_ref() == Some(path),
            open: Message::OpenFile(path.clone()),
        })
        .collect();
    let collapsed: BTreeSet<_> = app
        .collapsed_worktree_folders
        .iter()
        .filter_map(|path| path.strip_prefix(prefix).map(str::to_owned))
        .collect();
    changed_tree::view(
        entries,
        &collapsed,
        &app.i18n,
        |path| Message::ToggleWorktreeFolder {
            repository: (!prefix.is_empty()).then(|| root.to_owned()),
            path,
        },
        minimum_width,
        usize::from(!prefix.is_empty()),
    )
}
