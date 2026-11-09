//! Keep opened source files and their scroll positions in memory.

use super::{App, Message};
use crate::{
    domain::{
        Failure,
        changes::TextCursor,
        repository::{ChangedFile, Commit},
        workspace::DiffView,
    },
    infrastructure::repository,
    ui::{
        code::{Document, comparison::Side, folding::Folding, syntax_for},
        theme,
    },
};
use iced::{Task, widget::operation};
use std::{path::PathBuf, sync::Arc};

#[derive(Debug, Clone)]
enum Source {
    Working {
        root: PathBuf,
        path: PathBuf,
    },
    Commit {
        root: PathBuf,
        commit: Box<Commit>,
        file: ChangedFile,
    },
}

impl Source {
    fn extension(&self) -> String {
        match self {
            Self::Working { path, .. } => syntax_for(path),
            Self::Commit { file, .. } => syntax_for(std::path::Path::new(&file.path)),
        }
    }

    async fn read(self) -> Result<Document, Failure> {
        let text = match self.clone() {
            Self::Working { root, path } => repository::working_text(root, path).await,
            Self::Commit { root, commit, file } => {
                repository::commit_text(root, *commit, file).await
            }
        };
        let text = match text {
            Ok(text) => text,
            Err(Failure::TooLarge) => return self.read_page(TextCursor::default(), None).await,
            Err(error) => return Err(error),
        };
        tokio::task::spawn_blocking(move || Document::plain(text))
            .await
            .map_err(|_| Failure::Read)
    }

    async fn read_page(self, cursor: TextCursor, version: Option<(u64, u128)>) -> Result<Document, Failure> {
        let (text, page) = match self {
            Self::Working { root, path } => repository::working_page(root, path, cursor, version).await?,
            Self::Commit { root, commit, file } => {
                let revision = if file.status == 'D' {
                    commit.parents.first().cloned().ok_or(Failure::Read)?
                } else { commit.id };
                repository::blob_page(root, revision, file.path, cursor).await?
            }
        };
        tokio::task::spawn_blocking(move || Document::plain(text).with_page(Some(page)))
            .await.map_err(|_| Failure::Read)
    }

    async fn compare(self, document: Arc<Document>) -> Result<Document, Failure> {
        let extension = self.extension();
        let text = document.text.clone();
        let source = match self {
            Self::Working { root, path } => repository::working_comparison(root, path, text).await,
            Self::Commit { root, commit, file } => {
                repository::commit_comparison(root, *commit, file, text).await
            }
        };
        tokio::task::spawn_blocking(move || {
            Arc::unwrap_or_clone(document).with_comparison(source, &extension)
        })
        .await
        .map_err(|_| Failure::Read)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) enum Preparation {
    #[default]
    Text,
    Syntax,
    Changes,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum PageDirection { Previous, Next, Refresh }

#[derive(Debug, Clone, Default)]
pub(crate) struct Preview {
    pub id: u64,
    pub title: String,
    pub path: Option<PathBuf>,
    pub revision: Option<String>,
    pub document: Option<Arc<Document>>,
    pub error: Option<Failure>,
    pub loading: bool,
    pub preparation: Preparation,
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub diff_scroll_x: [f32; 2],
    pub diff_scroll_y: f32,
    pub folding: Folding,
    unfolded_scroll: (f32, f32),
    diff_unfolded_scroll: ([f32; 2], f32),
    source: Option<Source>,
    page_history: Vec<TextCursor>,
}

pub(crate) struct State {
    pub files: Vec<Preview>,
    closed: Vec<Preview>,
    active: usize,
    root: Option<PathBuf>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            // Keep an empty preview when no file is open.
            files: vec![Preview::default()],
            closed: Vec::new(),
            active: 0,
            root: None,
        }
    }
}

impl App {
    pub(crate) fn preview(&self) -> &Preview {
        &self.tabs.files[self.tabs.active]
    }

    #[cfg(test)]
    pub(super) fn preview_mut(&mut self) -> &mut Preview {
        &mut self.tabs.files[self.tabs.active]
    }

    pub(super) fn preview_for_request(&mut self, id: u64) -> Option<&mut Preview> {
        self.tabs.files.iter_mut().find(|preview| preview.id == id)
    }

    pub(super) fn open_file(&mut self, path: PathBuf) -> Task<Message> {
        let Some(root) = self.workspace.directory().cloned() else {
            return Task::none();
        };
        self.files.selected_path = Some(path.clone());
        self.open_source(Source::Working { root, path })
    }

    pub(super) fn open_commit_file(&mut self, index: usize) -> Task<Message> {
        let Some(file) = self.changed_files.get(index) else {
            return Task::none();
        };
        let Some(repo) = &self.repository else {
            return Task::none();
        };
        let Some(key) = self.selected_commit.as_ref() else {
            return Task::none();
        };
        let Some(commit) = repo.commit(key) else {
            return Task::none();
        };
        self.open_source(Source::Commit {
            root: key.root.clone(),
            commit: Box::new(commit.clone()),
            file: file.clone(),
        })
    }

    fn open_source(&mut self, source: Source) -> Task<Message> {
        let (path, title, revision) = match &source {
            Source::Working { root, path } => (
                path.clone(),
                path.strip_prefix(root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .into_owned(),
                None,
            ),
            Source::Commit { root, commit, file } => (
                root.join(&file.path),
                file.path.clone(),
                Some(commit.id.clone()),
            ),
        };
        let existing = self.tabs.files.iter().position(|preview| {
            preview.path.as_ref() == Some(&path) && preview.revision == revision
        });
        if let Some(index) = existing
            && self.tabs.files[index].error.is_none()
        {
            return self.select_tab(self.tabs.files[index].id);
        }

        self.file_request += 1;
        let preview = Preview {
            id: self.file_request,
            title,
            path: Some(path),
            revision,
            loading: true,
            source: Some(source.clone()),
            ..Preview::default()
        };
        let index = existing.or_else(|| {
            (self.tabs.files.len() == 1 && self.tabs.files[0].source.is_none()).then_some(0)
        });
        if let Some(index) = index {
            self.tabs.files[index] = preview;
            self.tabs.active = index;
        } else {
            self.tabs.active = self.tabs.files.len();
            self.tabs.files.push(preview);
        }
        self.tabs.root = self.workspace.directory().cloned();
        Task::batch([
            self.read_preview(self.file_request, source),
            self.reveal_tab(),
        ])
    }

    fn read_preview(&self, id: u64, source: Source) -> Task<Message> {
        let generation = self.generation;
        Task::perform(source.read(), move |result| {
            Message::FilePrepared(generation, id, Preparation::Text, result)
        })
    }

    pub(super) fn reload_large_preview(&mut self, id: u64) -> Task<Message> {
        if self.preview().id != id { return Task::none(); }
        let Some(source) = self.preview().source.clone() else { return Task::none(); };
        self.file_request += 1;
        let request = self.file_request;
        let preview = &mut self.tabs.files[self.tabs.active];
        preview.id = request;
        preview.loading = true;
        preview.preparation = Preparation::Text;
        preview.error = None;
        self.read_preview(request, source)
    }

    pub(super) fn read_file_part(&mut self, id: u64, direction: PageDirection) -> Task<Message> {
        if self.preview().id != id || self.preview().loading { return Task::none(); }
        let Some(page) = self.preview().document.as_ref().and_then(|document| document.page) else {
            return Task::none();
        };
        let preview = &self.tabs.files[self.tabs.active];
        let cursor = match direction {
            PageDirection::Next => page.next,
            PageDirection::Previous => preview.page_history.iter().rev()
                .find(|cursor| cursor.offset < page.cursor.offset).copied(),
            PageDirection::Refresh => Some(page.cursor),
        };
        let (Some(cursor), Some(source)) = (cursor, preview.source.clone()) else { return Task::none(); };
        self.file_request += 1;
        let request = self.file_request;
        let preview = &mut self.tabs.files[self.tabs.active];
        if cursor.offset > page.cursor.offset && preview.page_history.last() != Some(&page.cursor) {
            preview.page_history.push(page.cursor);
        }
        preview.id = request;
        preview.loading = true;
        preview.preparation = Preparation::Text;
        preview.error = None;
        preview.scroll_x = 0.;
        preview.scroll_y = 0.;
        let generation = self.generation;
        Task::perform(source.read_page(cursor, page.version), move |result| {
            Message::FilePrepared(generation, request, Preparation::Text, result)
        })
    }

    pub(super) fn continue_preview(
        &mut self,
        id: u64,
        stage: Preparation,
        result: Result<Document, Failure>,
    ) -> Task<Message> {
        let generation = self.generation;
        let Some(preview) = self.preview_for_request(id) else {
            return Task::none();
        };
        if !preview.loading || preview.preparation != stage {
            return Task::none();
        }
        let document = match result {
            Ok(document) => Arc::new(document),
            Err(error) => return self.finish_preview(id, Err(error)),
        };
        if let Some(page) = document.page {
            preview.page_history.retain(|cursor| cursor.offset < page.cursor.offset);
        }
        let Some(source) = preview.source.clone() else {
            return Task::none();
        };
        preview.folding.refresh(&document);
        preview.document = Some(Arc::clone(&document));
        preview.error = None;
        match stage {
            Preparation::Text => {
                preview.preparation = Preparation::Syntax;
                let extension = source.extension();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            Document::new(document.text.clone(), &extension).with_page(document.page)
                        })
                        .await
                        .map_err(|_| Failure::Read)
                    },
                    move |result| {
                        Message::FilePrepared(generation, id, Preparation::Syntax, result)
                    },
                )
            }
            Preparation::Syntax => {
                if document.page.is_some() {
                    return self.finish_preview(id, Ok(Arc::unwrap_or_clone(document)));
                }
                preview.preparation = Preparation::Changes;
                Task::perform(source.compare(document), move |result| {
                    Message::FileLoaded(generation, id, result)
                })
            }
            Preparation::Changes => Task::none(),
        }
    }

    pub(super) fn finish_preview(
        &mut self,
        id: u64,
        result: Result<Document, Failure>,
    ) -> Task<Message> {
        let collapse = self.workspace.preferences.collapse_unchanged;
        let Some(preview) = self.preview_for_request(id) else {
            return Task::none();
        };
        preview.loading = false;
        match result {
            Ok(document) => {
                if preview.document.as_ref().map(|old| old.fingerprint)
                    != Some(document.fingerprint)
                    || preview.folding.collapsed != (collapse && document.can_fold())
                {
                    preview.folding.collapsed = collapse;
                    preview.folding.refresh(&document);
                    if preview.folding.collapsed {
                        preview.scroll_x = 0.;
                        preview.scroll_y = 0.;
                        preview.unfolded_scroll = (0., 0.);
                        preview.diff_scroll_x = [0.; 2];
                        preview.diff_scroll_y = 0.;
                        preview.diff_unfolded_scroll = ([0.; 2], 0.);
                    }
                }
                preview.document = Some(Arc::new(document));
                preview.error = None;
            }
            Err(error) => preview.error = Some(error),
        }
        if self.preview().id == id {
            self.restore_preview_scroll()
        } else {
            Task::none()
        }
    }

    pub(super) fn toggle_unchanged(&mut self, id: u64) -> Task<Message> {
        if self.preview().id != id {
            return Task::none();
        }
        let Some(_) = self
            .preview()
            .document
            .as_ref()
            .filter(|document| document.can_fold())
        else {
            return Task::none();
        };
        let collapse = !self.preview().folding.collapsed;
        self.workspace.preferences.collapse_unchanged = collapse;
        for preview in self
            .tabs
            .files
            .iter_mut()
            .chain(self.tabs.closed.iter_mut())
        {
            if let Some(document) = &preview.document
                && document.can_fold()
                && preview.folding.collapsed != collapse
            {
                if collapse {
                    preview.unfolded_scroll = (preview.scroll_x, preview.scroll_y);
                    preview.diff_unfolded_scroll = (preview.diff_scroll_x, preview.diff_scroll_y);
                    preview.scroll_x = 0.;
                    preview.scroll_y = 0.;
                    preview.diff_scroll_x = [0.; 2];
                    preview.diff_scroll_y = 0.;
                } else {
                    (preview.scroll_x, preview.scroll_y) = preview.unfolded_scroll;
                    (preview.diff_scroll_x, preview.diff_scroll_y) = preview.diff_unfolded_scroll;
                }
                preview.folding.toggle(document);
            }
        }
        Task::batch([self.restore_preview_scroll(), self.save()])
    }

    pub(super) fn toggle_diff_view(&mut self, id: u64) -> Task<Message> {
        if self.preview().id != id
            || self
                .preview()
                .document
                .as_ref()
                .is_none_or(|document| document.comparison.is_none())
        {
            return Task::none();
        }
        self.workspace.preferences.diff_view = match self.workspace.preferences.diff_view {
            DiffView::Unified => DiffView::SideBySide,
            DiffView::SideBySide => DiffView::Unified,
        };
        Task::batch([self.restore_preview_scroll(), self.save()])
    }

    pub(super) fn scroll_diff(&mut self, id: u64, side: Side, x: f32, y: f32) -> Task<Message> {
        if self.preview().id != id || self.workspace.preferences.diff_view != DiffView::SideBySide {
            return Task::none();
        }
        let preview = &mut self.tabs.files[self.tabs.active];
        preview.diff_scroll_x[side.index()] = x;
        let changed = (preview.diff_scroll_y - y).abs() > 0.1;
        preview.diff_scroll_y = y;
        if changed {
            let other = side.other();
            operation::scroll_to(
                other.scroll_id(),
                operation::AbsoluteOffset {
                    x: preview.diff_scroll_x[other.index()],
                    y,
                },
            )
        } else {
            Task::none()
        }
    }

    pub(super) fn expand_unchanged(
        &mut self,
        id: u64,
        range: std::ops::Range<usize>,
    ) -> Task<Message> {
        if self.preview().id == id {
            let preview = &mut self.tabs.files[self.tabs.active];
            if let Some(document) = &preview.document {
                preview.folding.expand(document, range);
            }
        }
        Task::none()
    }

    pub(super) fn select_tab(&mut self, id: u64) -> Task<Message> {
        let Some(index) = self.tabs.files.iter().position(|preview| preview.id == id) else {
            return Task::none();
        };
        self.tabs.active = index;
        if self.preview().revision.is_none() {
            self.files.selected_path = self.preview().path.clone();
        }
        Task::batch([
            self.restore_preview_scroll(),
            self.reveal_tab(),
            self.refresh_preview(),
        ])
    }

    pub(super) fn close_tab(&mut self, id: u64) -> Task<Message> {
        let Some(index) = self.tabs.files.iter().position(|preview| preview.id == id) else {
            return Task::none();
        };
        let active = self.preview().id;
        let closed = self.tabs.files.remove(index);
        if closed.source.is_some() {
            // Bound cached documents while retaining recent closures in order.
            if self.tabs.closed.len() == 20 {
                self.tabs.closed.remove(0);
            }
            self.tabs.closed.push(closed);
        }
        if self.changes.preview_pending == Some(id) {
            self.changes.preview_pending = None;
        }
        if self.tabs.files.is_empty() {
            self.tabs.files.push(Preview::default());
            self.tabs.active = 0;
            return Task::none();
        }
        self.tabs.active = self
            .tabs
            .files
            .iter()
            .position(|preview| preview.id == active)
            .unwrap_or(index.min(self.tabs.files.len() - 1));
        self.select_tab(self.preview().id)
    }

    pub(super) fn reopen_closed_tab(&mut self) -> Task<Message> {
        while let Some(mut preview) = self.tabs.closed.pop() {
            if self
                .tabs
                .files
                .iter()
                .any(|open| open.path == preview.path && open.revision == preview.revision)
            {
                continue;
            }
            let Some(source) = preview.source.clone() else {
                continue;
            };
            self.file_request += 1;
            preview.id = self.file_request;
            preview.loading |= preview.document.is_none() || preview.error.is_some();
            preview.preparation = Preparation::Text;
            preview.error = None;
            let read = if preview.loading {
                self.read_preview(preview.id, source)
            } else {
                Task::none()
            };
            if self.tabs.files.len() == 1 && self.tabs.files[0].source.is_none() {
                self.tabs.files[0] = preview;
            } else {
                self.tabs.files.push(preview);
            }
            return Task::batch([read, self.select_tab(self.file_request)]);
        }
        Task::none()
    }

    pub(super) fn sync_preview_project(&mut self) -> Task<Message> {
        let root = self.workspace.directory().cloned();
        if self.tabs.root != root {
            self.tabs = State {
                root,
                ..State::default()
            };
            return Task::none();
        }
        for preview in &mut self.tabs.files {
            if preview.loading {
                preview.preparation = Preparation::Text;
            }
        }
        let reads = self
            .tabs
            .files
            .iter()
            .filter(|preview| preview.loading)
            .filter_map(|preview| {
                preview
                    .source
                    .clone()
                    .map(|source| self.read_preview(preview.id, source))
            });
        Task::batch(reads.chain([self.restore_preview_scroll(), self.reveal_tab()]))
    }

    fn restore_preview_scroll(&self) -> Task<Message> {
        if self.workspace.preferences.diff_view == DiffView::SideBySide
            && self
                .preview()
                .document
                .as_ref()
                .is_some_and(|document| document.comparison.is_some())
        {
            return Task::batch([Side::Before, Side::After].map(|side| {
                operation::scroll_to(
                    side.scroll_id(),
                    operation::AbsoluteOffset {
                        x: self.preview().diff_scroll_x[side.index()],
                        y: self.preview().diff_scroll_y,
                    },
                )
            }));
        }
        operation::scroll_to(
            "source-code",
            operation::AbsoluteOffset {
                x: self.preview().scroll_x,
                y: self.preview().scroll_y,
            },
        )
    }

    fn reveal_tab(&self) -> Task<Message> {
        operation::scroll_to(
            "file-tabs",
            operation::AbsoluteOffset {
                x: self.tabs.active as f32 * theme::FILE_TAB_WIDTH,
                y: 0.,
            },
        )
    }
}

#[cfg(test)]
mod tests;
