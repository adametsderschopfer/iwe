//! Keep opened source files and their scroll positions in memory.

use super::{App, Message};
#[cfg(test)]
use crate::ui::theme;
use crate::{
    domain::{
        Failure,
        changes::{TextCursor, TextPage},
        repository::{ChangedFile, Commit},
        workspace::DiffView,
    },
    infrastructure::repository,
    ui::code::{Document, comparison::Side, folding::Folding, syntax_for},
};
use iced::{Task, widget::operation};
use std::{path::PathBuf, sync::Arc};

const CLOSED_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
enum Source {
    Remote {
        path: PathBuf,
        remote: String,
        label: String,
        identity: String,
        local_root: Option<PathBuf>,
        text: Arc<String>,
        previous: Option<Arc<String>>,
    },
    Working {
        root: PathBuf,
        path: PathBuf,
        compare: bool,
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
            Self::Remote { path, .. } => syntax_for(path),
            Self::Working { path, .. } => syntax_for(path),
            Self::Commit { file, .. } => syntax_for(std::path::Path::new(&file.path)),
        }
    }

    async fn read(self) -> Result<Document, Failure> {
        let text = match self.clone() {
            Self::Remote {
                local_root: Some(root),
                path,
                ..
            } => repository::working_text(root, path).await,
            Self::Remote { text, .. } => Ok((*text).clone()),
            Self::Working {
                root,
                path,
                compare: false,
            } => crate::infrastructure::workspace::read(root, path).await,
            Self::Working {
                root,
                path,
                compare: true,
            } => repository::working_text(root, path).await,
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

    async fn read_page(
        self,
        cursor: TextCursor,
        version: Option<(u64, u128)>,
    ) -> Result<Document, Failure> {
        let (text, page) = match self {
            Self::Remote { .. } => return Err(Failure::TooLarge),
            Self::Working {
                root,
                path,
                compare: false,
            } => crate::infrastructure::workspace::pages::read(root, path, cursor, version).await?,
            Self::Working {
                root,
                path,
                compare: true,
            } => repository::working_page(root, path, cursor, version).await?,
            Self::Commit { root, commit, file } => {
                let revision = if file.status == 'D' {
                    commit.parents.first().cloned().ok_or(Failure::Read)?
                } else {
                    commit.id
                };
                repository::blob_page(root, revision, file.path, cursor).await?
            }
        };
        tokio::task::spawn_blocking(move || Document::plain(text).with_page(Some(page)))
            .await
            .map_err(|_| Failure::Read)
    }

    async fn read_line(self, line: usize) -> Result<Option<(Document, Vec<TextCursor>)>, Failure> {
        tokio::time::timeout(std::time::Duration::from_secs(45), async move {
            let mut cursor = TextCursor::default();
            let mut version = None;
            let mut history = Vec::new();
            loop {
                let document = self.clone().read_page(cursor, version).await?;
                let page = document.page.ok_or(Failure::Read)?;
                if page.cursor.offset != cursor.offset {
                    history.clear();
                }
                if document.line_info.iter().any(|info| info.number == line) {
                    return Ok(Some((document, history)));
                }
                let Some(next) = page.next else {
                    return Ok(None);
                };
                if next.offset <= page.cursor.offset {
                    return Err(Failure::Read);
                }
                history.push(page.cursor);
                cursor = next;
                version = page.version;
            }
        })
        .await
        .map_err(|_| Failure::Unavailable)?
    }

    async fn compare(self, document: Arc<Document>) -> Result<Document, Failure> {
        let extension = self.extension();
        let text = document.text.clone();
        let explicit = matches!(
            &self,
            Self::Remote {
                previous: Some(_),
                ..
            }
        );
        let source = match self {
            Self::Remote { previous, .. } => crate::domain::changes::Source {
                text,
                previous: previous.map(|p| (*p).clone()),
                ..Default::default()
            },
            Self::Working { root, path, .. } => {
                repository::working_comparison(root, path, text).await
            }
            Self::Commit { root, commit, file } => {
                repository::commit_comparison(root, *commit, file, text).await
            }
        };
        tokio::task::spawn_blocking(move || {
            let document = Arc::unwrap_or_clone(document);
            if explicit {
                document.with_explicit_comparison(source, &extension)
            } else {
                document.with_comparison(source, &extension)
            }
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
pub(crate) enum PageDirection {
    Previous,
    Next,
    Refresh,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Preview {
    pub editor: Option<super::editor::Buffer>,
    pub remote: bool,
    pub pending_comparison: Option<(String, PathBuf)>,
    pub selection: crate::ui::code::selection::Selection,
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
    suspended: Option<Suspended>,
}

impl Preview {
    pub(crate) fn working_comparison(&self) -> bool {
        matches!(self.source, Some(Source::Working { compare: true, .. }))
    }

    pub(crate) fn comparison_received(&self) -> bool {
        matches!(
            &self.source,
            Some(Source::Remote {
                local_root: None,
                previous: Some(_),
                ..
            })
        )
    }
}

#[derive(Debug, Clone)]
struct Suspended {
    page: Option<TextPage>,
    fingerprint: [u8; 32],
    folding: Folding,
}

pub(crate) struct State {
    pub files: Vec<Preview>,
    pub menu: Option<iced::Point>,
    closed: Vec<Preview>,
    active: usize,
    root: Option<PathBuf>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            // Keep an empty preview when no file is open.
            files: vec![Preview::default()],
            menu: None,
            closed: Vec::new(),
            active: 0,
            root: None,
        }
    }
}

impl State {
    fn trim_closed_documents(&mut self, budget: usize) {
        let mut retained = 0;
        for preview in self.closed.iter_mut().rev() {
            let Some(document) = &preview.document else {
                continue;
            };
            let bytes = document.memory_bytes();
            if bytes <= budget - retained {
                retained += bytes;
            } else {
                let mut folding = std::mem::take(&mut preview.folding);
                folding.release_layout();
                preview.suspended = Some(Suspended {
                    page: document.page,
                    fingerprint: document.fingerprint,
                    folding,
                });
                preview.document = None;
                preview.editor = None;
            }
        }
    }
}

impl App {
    pub(crate) fn active_local_file(&self) -> Option<PathBuf> {
        match self.preview().source.as_ref()? {
            Source::Working { path, .. } => Some(path.clone()),
            Source::Remote {
                local_root: Some(_),
                path,
                ..
            } => Some(path.clone()),
            Source::Remote {
                path,
                previous: Some(_),
                ..
            } => Some(path.clone()),
            _ => None,
        }
    }

    pub(super) fn open_remote_preview(
        &mut self,
        path: PathBuf,
        remote: String,
        origin: (String, String),
        text: Arc<String>,
        previous: Option<Arc<String>>,
    ) -> Task<Message> {
        if previous.is_some() {
            let Some(index) = self.tabs.files.iter().position(|preview| {
                preview.pending_comparison.as_ref() == Some(&(origin.0.clone(), path.clone()))
            }) else {
                return Task::none();
            };
            let prepared = self.tabs.files[index]
                .document
                .as_ref()
                .filter(|document| {
                    document.has_syntax() && document.page.is_none() && document.text == *text
                })
                .cloned();
            self.file_request += 1;
            let id = self.file_request;
            let source = Source::Remote {
                path,
                remote,
                identity: origin.0,
                label: origin.1,
                local_root: None,
                text,
                previous,
            };
            let preview = &mut self.tabs.files[index];
            preview.id = id;
            preview.source = Some(source.clone());
            preview.loading = true;
            preview.preparation = Preparation::Text;
            preview.error = None;
            // Reuse the exact local snapshot; a changed file follows normal preparation.
            if let Some(document) = prepared {
                preview.preparation = Preparation::Changes;
                let generation = self.generation;
                return Task::perform(source.compare(document), move |result| {
                    Message::FileLoaded(generation, id, result)
                });
            }
            // Keep the local snapshot visible until comparison preparation finishes.
            return self.read_preview(id, source);
        }
        self.navigation = super::Navigation::Files;
        self.open_source(Source::Remote {
            path,
            remote,
            identity: origin.0,
            label: origin.1,
            local_root: None,
            text,
            previous,
        })
    }
    pub(super) fn begin_remote_comparison(
        &mut self,
        profile: String,
        label: String,
        path: PathBuf,
        remote: String,
    ) -> Task<Message> {
        let Some(root) = self.workspace.directory().cloned() else {
            return Task::none();
        };
        if let Some(preview) = self.tabs.files.iter().find(|preview| {
            preview.pending_comparison.as_ref() == Some(&(profile.clone(), path.clone()))
        }) {
            return self.select_tab(preview.id);
        }
        self.navigation = super::Navigation::Files;
        let task = self.open_source(Source::Remote {
            path: path.clone(),
            remote,
            identity: profile.clone(),
            label,
            local_root: Some(root),
            text: Arc::new(String::new()),
            previous: None,
        });
        self.tabs.files[self.tabs.active].pending_comparison = Some((profile, path));
        task
    }

    pub(super) fn load_line_preview(&self, id: u64, request: u64, line: usize) -> Task<Message> {
        let Some(source) = self
            .preview()
            .source
            .clone()
            .filter(|_| self.preview().id == id)
        else {
            return Task::none();
        };
        let generation = self.generation;
        Task::perform(source.read_line(line), move |result| {
            Message::GoToLine(super::line_navigation::Event::Loaded(
                generation,
                id,
                request,
                result.map(|page| {
                    page.map(|(document, history)| super::line_navigation::LoadedPage {
                        document: Box::new(document),
                        history,
                    })
                }),
            ))
        })
    }

    pub(super) fn accept_line_preview(
        &mut self,
        document: Document,
        history: Vec<TextCursor>,
    ) -> Task<Message> {
        self.file_request += 1;
        let id = self.file_request;
        let preview = &mut self.tabs.files[self.tabs.active];
        preview.id = id;
        preview.page_history = history;
        preview.loading = true;
        preview.preparation = Preparation::Text;
        preview.scroll_x = 0.;
        preview.scroll_y = 0.;
        self.continue_preview(id, Preparation::Text, Ok(document))
    }
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
        self.open_working_file(path, false)
    }

    pub(super) fn open_working_change(&mut self, path: PathBuf) -> Task<Message> {
        self.open_working_file(path, true)
    }

    fn open_working_file(&mut self, path: PathBuf, compare: bool) -> Task<Message> {
        let Some(root) = self.workspace.directory().cloned() else {
            return Task::none();
        };
        self.navigation = super::Navigation::Files;
        self.files.selected_path = Some(path.clone());
        self.open_source(Source::Working {
            root,
            path,
            compare,
        })
    }

    pub(super) fn open_commit_file(&mut self, index: usize) -> Task<Message> {
        let Some(file) = self.modules.git.changed_files.get(index) else {
            return Task::none();
        };
        let Some(repo) = &self.repository else {
            return Task::none();
        };
        let Some(key) = self.modules.git.selected_commit.as_ref() else {
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
            Source::Remote {
                path,
                remote,
                label,
                identity,
                ..
            } => (
                path.clone(),
                format!("{label}: {remote}"),
                Some(format!("{identity}:{remote}")),
            ),
            Source::Working { root, path, .. } => (
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
            preview.path.as_ref() == Some(&path)
                && preview.revision == revision
                && preview.working_comparison()
                    == matches!(source, Source::Working { compare: true, .. })
        });
        if let Some(index) = existing
            && (!matches!(source, Source::Remote { .. })
                || self.tabs.files[index]
                    .editor
                    .as_ref()
                    .is_some_and(super::editor::Buffer::protected))
            && self.tabs.files[index].error.is_none()
        {
            return self.select_tab(self.tabs.files[index].id);
        }

        self.modules.deployment.target = None;
        self.file_request += 1;
        let preview = Preview {
            remote: matches!(source, Source::Remote { .. }),
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
        if self.preview().id != id {
            return Task::none();
        }
        let Some(source) = self.preview().source.clone() else {
            return Task::none();
        };
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
        if self.preview().id != id || self.preview().loading {
            return Task::none();
        }
        let Some(page) = self
            .preview()
            .document
            .as_ref()
            .and_then(|document| document.page)
        else {
            return Task::none();
        };
        let preview = &self.tabs.files[self.tabs.active];
        let cursor = match direction {
            PageDirection::Next => page.next,
            PageDirection::Previous => preview
                .page_history
                .iter()
                .rev()
                .find(|cursor| cursor.offset < page.cursor.offset)
                .copied(),
            PageDirection::Refresh => Some(page.cursor),
        };
        let (Some(cursor), Some(source)) = (cursor, preview.source.clone()) else {
            return Task::none();
        };
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
        let git_enabled = self.module_enabled(crate::modules::ModuleId::Git);
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
            if preview
                .suspended
                .as_ref()
                .and_then(|suspended| suspended.page)
                .is_some_and(|previous| previous.cursor != page.cursor)
            {
                preview.scroll_x = 0.;
                preview.scroll_y = 0.;
            }
            preview
                .page_history
                .retain(|cursor| cursor.offset < page.cursor.offset);
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
                            Document::new(document.text.clone(), &extension)
                                .with_page(document.page)
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
                if document.page.is_some()
                    || matches!(source, Source::Working { compare: false, .. })
                    || (!git_enabled && matches!(source, Source::Working { .. }))
                {
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
        if preview
            .editor
            .as_ref()
            .is_some_and(super::editor::Buffer::protected)
        {
            return Task::none();
        }
        if result.is_ok() && preview.comparison_received() {
            preview.pending_comparison = None;
        }
        match result {
            Ok(document) => {
                let suspended = preview.suspended.take().filter(|suspended| {
                    suspended.fingerprint == document.fingerprint
                        && suspended.folding.collapsed == (collapse && document.can_fold())
                });
                if let Some(mut suspended) = suspended {
                    suspended.folding.restore_layout(&document);
                    preview.folding = suspended.folding;
                } else if preview.document.as_ref().map(|old| old.fingerprint)
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
                if document.page.is_none() && document.text.len() <= crate::domain::editor::LIMIT {
                    let target = match &preview.source {
                        Some(Source::Working {
                            path,
                            compare: false,
                            ..
                        }) => Some(super::editor::Target::Local(path.clone())),
                        Some(Source::Remote {
                            identity,
                            remote,
                            previous: None,
                            local_root: None,
                            ..
                        }) if preview.pending_comparison.is_none() => {
                            Some(super::editor::Target::Remote {
                                profile: identity.clone(),
                                path: remote.clone(),
                            })
                        }
                        _ => None,
                    };
                    if let Some(target) = target {
                        if preview
                            .editor
                            .as_ref()
                            .is_none_or(|buffer| buffer.document.text != document.text)
                        {
                            preview.editor =
                                Some(super::editor::Buffer::new(document.text.clone(), target));
                        }
                    } else {
                        preview.editor = None;
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

    pub(super) fn refresh_saved_editor(&mut self, id: u64) -> Task<Message> {
        let Some(preview) = self.preview_for_request(id) else {
            return Task::none();
        };
        if let (Some(Source::Remote { text, .. }), Some(buffer)) =
            (&mut preview.source, &preview.editor)
        {
            *text = Arc::new(buffer.document.saved.clone());
        }
        let Some(source) = preview.source.clone() else {
            return Task::none();
        };
        preview.loading = true;
        preview.preparation = Preparation::Text;
        self.read_preview(id, source)
    }

    pub(super) fn toggle_unchanged(&mut self, id: u64) -> Task<Message> {
        if self.preview().id != id || self.preview().editor.is_some() {
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
            || self.preview().editor.is_some()
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
        self.modules.deployment.target = None;
        self.tabs.menu = None;
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
        self.modules.deployment.target = None;
        self.tabs.menu = None;
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
            self.tabs.trim_closed_documents(CLOSED_DOCUMENT_BYTES);
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
            if self.tabs.files.iter().any(|open| {
                open.path == preview.path
                    && open.revision == preview.revision
                    && open.working_comparison() == preview.working_comparison()
            }) {
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
                if let Some(page) = preview
                    .document
                    .as_ref()
                    .and_then(|document| document.page)
                    .or_else(|| {
                        preview
                            .suspended
                            .as_ref()
                            .and_then(|suspended| suspended.page)
                    })
                {
                    let (generation, id) = (self.generation, preview.id);
                    Task::perform(source.read_page(page.cursor, page.version), move |result| {
                        Message::FilePrepared(generation, id, Preparation::Text, result)
                    })
                } else {
                    self.read_preview(preview.id, source)
                }
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
            let drafts: Vec<_> = self
                .tabs
                .files
                .drain(..)
                .filter(|tab| {
                    tab.editor
                        .as_ref()
                        .is_some_and(super::editor::Buffer::protected)
                })
                .collect();
            self.tabs = State {
                root,
                ..State::default()
            };
            // Keep drafts if an asynchronous project update bypassed a navigation prompt.
            if !drafts.is_empty() {
                self.tabs.files = drafts;
            }
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

    pub(super) fn restore_preview_scroll(&self) -> Task<Message> {
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
        let offset = self.tabs.files[..self.tabs.active]
            .iter()
            .filter(|preview| !preview.title.is_empty())
            .map(|preview| {
                let label =
                    crate::ui::tab_label::label(&preview.title, preview.revision.as_deref());
                crate::ui::tab_label::width(&label)
            })
            .sum();
        operation::scroll_to("file-tabs", operation::AbsoluteOffset { x: offset, y: 0. })
    }
}

#[cfg(test)]
mod tests;
