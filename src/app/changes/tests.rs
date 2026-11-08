use super::*;
use crate::i18n::{I18n, Language};
use std::{path::PathBuf, rc::Rc};

#[test]
fn status_results_cannot_cross_project_or_request_boundaries() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.generation = 4;
    app.changes.request = 2;
    app.changes.pending = true;
    let mut status = Worktree::default();
    status.folders.insert(
        PathBuf::from("/old"),
        crate::domain::changes::FileStatus::Modified,
    );
    let _ = app.update_changes(Event::Loaded(3, 2, Ok(status.clone())));
    let _ = app.update_changes(Event::Loaded(4, 1, Ok(status)));
    assert!(app.changes.status.folders.is_empty());
    assert!(app.changes.pending);
    let _ = app.update_changes(Event::Loaded(4, 2, Ok(Worktree::default())));
    assert!(!app.changes.pending);
}

#[test]
fn background_previews_preserve_scroll_and_reject_old_file_results() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.generation = 2;
    app.file_request = 5;
    app.preview_mut().id = 5;
    app.preview_mut().scroll_y = 240.;
    app.preview_mut().document = Some(Document::new("current".to_owned(), "txt").into());
    app.changes.preview_pending = Some(4);
    let _ = app.update_changes(Event::PreviewLoaded(
        2,
        4,
        Ok(Some(Document::new("stale".to_owned(), "txt"))),
    ));
    assert_eq!(app.preview().document.as_ref().unwrap().text, "current");
    assert!(app.changes.preview_pending.is_none());
    let _ = app.update_changes(Event::PreviewLoaded(
        2,
        5,
        Ok(Some(Document::new("updated".to_owned(), "txt"))),
    ));
    assert_eq!(app.preview().document.as_ref().unwrap().text, "updated");
    assert_eq!(app.preview().scroll_y, 240.);
}
