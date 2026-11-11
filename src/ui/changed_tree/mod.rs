//! Display changed files with compact folders, file counts, and status colors.

use super::{ellipsis, file_icons, theme};
use crate::ui::tooltip;
use crate::{
    domain::changes::FileStatus,
    i18n::{FluentArgs, I18n, Key},
};
use iced::{
    Alignment, Element, Length,
    widget::{Column, button, row, space, text},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub(crate) struct Entry<Message> {
    pub path: String,
    pub status: FileStatus,
    pub selected: bool,
    pub open: Message,
}

struct Node<Message> {
    folders: BTreeMap<String, Self>,
    files: Vec<Entry<Message>>,
}

impl<Message> Default for Node<Message> {
    fn default() -> Self {
        Self {
            folders: BTreeMap::new(),
            files: Vec::new(),
        }
    }
}

impl<Message> Node<Message> {
    fn count(&self) -> usize {
        self.files.len() + self.folders.values().map(Self::count).sum::<usize>()
    }
}

pub(crate) fn view<'a, Message: Clone + 'a>(
    entries: Vec<Entry<Message>>,
    collapsed: &BTreeSet<String>,
    i18n: &I18n,
    on_toggle: impl Fn(String) -> Message,
    minimum_width: f32,
    depth: usize,
) -> Element<'a, Message> {
    let mut root = Node::default();
    for entry in entries {
        let mut node = &mut root;
        let folders: Vec<_> = entry.path.split('/').collect();
        for folder in folders.iter().take(folders.len().saturating_sub(1).min(32)) {
            node = node.folders.entry((*folder).to_owned()).or_default();
        }
        node.files.push(entry);
    }
    let (body, width) = rows(
        root,
        "",
        depth,
        collapsed,
        i18n,
        &on_toggle,
        (Column::new().spacing(2), minimum_width),
    );
    body.width(width).into()
}

fn rows<'a, Message: Clone + 'a>(
    node: Node<Message>,
    parent: &str,
    depth: usize,
    collapsed: &BTreeSet<String>,
    i18n: &I18n,
    on_toggle: &impl Fn(String) -> Message,
    (mut result, mut width): (Column<'a, Message>, f32),
) -> (Column<'a, Message>, f32) {
    for (mut name, mut child) in node.folders {
        while child.files.is_empty() && child.folders.len() == 1 {
            let (next, descendant) = child.folders.pop_first().unwrap();
            name.push('/');
            name.push_str(&next);
            child = descendant;
        }
        let path = if parent.is_empty() {
            name.clone()
        } else {
            format!("{parent}/{name}")
        };
        let expanded = !collapsed.contains(&path);
        let mut args = FluentArgs::new();
        args.set("count", child.count());
        let count = i18n.format(Key::FileCount, &args);
        width = width.max(
            depth as f32 * theme::FILE_TREE_INDENT
                + ellipsis::measured_width(&name, theme::FILE_TREE_TEXT_SIZE).ceil()
                + ellipsis::measured_width(&count, 12.).ceil()
                + 50.,
        );
        let label = row![
            space().width(depth as f32 * theme::FILE_TREE_INDENT),
            file_icons::chevron(expanded),
            file_icons::folder(expanded, false),
            text(name)
                .size(theme::FILE_TREE_TEXT_SIZE)
                .style(theme::MUTED.text())
                .wrapping(text::Wrapping::None),
            text(count)
                .size(12)
                .style(theme::FAINT.text())
                .wrapping(text::Wrapping::None),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .height(theme::FILE_TREE_ROW_HEIGHT);
        result = result.push(
            button(label)
                .on_press(on_toggle(path.clone()))
                .padding([0, 4])
                .style(theme::tree_item(false))
                .width(Length::Fill),
        );
        if expanded {
            (result, width) = rows(
                child,
                &path,
                depth + 1,
                collapsed,
                i18n,
                on_toggle,
                (result, width),
            );
        }
    }
    let mut files = node.files;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    for entry in files {
        let filename = entry
            .path
            .rsplit('/')
            .next()
            .unwrap_or(&entry.path)
            .to_owned();
        let key = status_key(entry.status);
        width = width.max(
            depth as f32 * theme::FILE_TREE_INDENT
                + ellipsis::measured_width(&filename, theme::FILE_TREE_TEXT_SIZE).ceil()
                + 42.,
        );
        let label = row![
            space().width(depth as f32 * theme::FILE_TREE_INDENT + 10.),
            file_icons::file(Path::new(&entry.path)),
            text(filename)
                .size(theme::FILE_TREE_TEXT_SIZE)
                .style(theme::file_status(entry.status).text())
                .wrapping(text::Wrapping::None),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .height(theme::FILE_TREE_ROW_HEIGHT);
        result = result.push(tooltip::view(
            button(label)
                .on_press(entry.open)
                .padding([0, 4])
                .style(theme::tree_item(entry.selected))
                .width(Length::Fill),
            text(format!("{}\n{}", entry.path, i18n.text(key))).size(12),
            tooltip::Position::FollowCursor,
        ));
    }
    (result, width)
}

pub(crate) fn status_key(status: FileStatus) -> Key {
    match status {
        FileStatus::Untracked => Key::FileUntracked,
        FileStatus::Added => Key::FileAdded,
        FileStatus::Modified => Key::FileModified,
        FileStatus::Deleted => Key::FileDeleted,
        FileStatus::Renamed => Key::FileRenamed,
        FileStatus::Conflict => Key::FileConflict,
    }
}

#[cfg(test)]
mod tests;
