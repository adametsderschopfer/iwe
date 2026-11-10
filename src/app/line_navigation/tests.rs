use super::*;
use crate::i18n::{I18n, Language};
use std::{rc::Rc, sync::Arc};

fn app() -> App {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.preview_mut().document = Some(Arc::new(Document::new(
        "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight".into(),
        "txt",
    )));
    app
}

#[test]
fn navigation_reveals_and_selects_an_existing_line_without_changing_source() {
    let mut app = app();
    let fingerprint = app.preview().document.as_ref().unwrap().fingerprint;
    let _ = app.update_line_navigation(Event::Open);
    let _ = app.update_line_navigation(Event::Changed("8".into()));
    assert!(app.update_line_navigation(Event::Submit).units() > 0);
    assert!(app.line_navigation.is_none());
    assert!(app.preview().scroll_y > 0.);
    assert_eq!(app.preview().selection.lane(), Some(Lane::Unified));
    let document = app.preview().document.as_ref().unwrap();
    assert!(
        app.preview()
            .selection
            .highlights_line(document, Lane::Unified, 7)
    );
    assert!(
        !app.preview()
            .selection
            .highlights_line(document, Lane::Unified, 6)
    );
    assert!(
        !app.preview()
            .selection
            .highlights_line(document, Lane::Before, 7)
    );
    assert_eq!(
        app.preview().document.as_ref().unwrap().fingerprint,
        fingerprint
    );
}

#[test]
fn invalid_and_missing_lines_keep_the_dialog_open_without_scrolling() {
    let mut app = app();
    let _ = app.update_line_navigation(Event::Open);
    for value in ["0", "-1", "abc", ""] {
        let _ = app.update_line_navigation(Event::Changed(value.into()));
        let _ = app.update_line_navigation(Event::Submit);
        assert_eq!(
            app.line_navigation.as_ref().unwrap().error,
            Some(Key::InvalidLineNumber)
        );
    }
    let _ = app.update_line_navigation(Event::Changed("999".into()));
    let _ = app.update_line_navigation(Event::Submit);
    assert_eq!(
        app.line_navigation.as_ref().unwrap().error,
        Some(Key::LineNumberNotFound)
    );
    assert_eq!(app.preview().scroll_y, 0.);
}

#[test]
fn navigation_highlights_empty_lines_and_clears_the_previous_target() {
    let mut app = app();
    app.preview_mut().document = Some(Arc::new(Document::new("one\n\nthree".into(), "txt")));
    assert!(app.reveal_source_line(2));
    let document = app.preview().document.clone().unwrap();
    assert!(
        app.preview()
            .selection
            .highlights_line(&document, Lane::Unified, 1)
    );
    assert!(app.reveal_source_line(3));
    assert!(
        !app.preview()
            .selection
            .highlights_line(&document, Lane::Unified, 1)
    );
    assert!(
        app.preview()
            .selection
            .highlights_line(&document, Lane::Unified, 2)
    );
    app.preview_mut().selection.update(Action::Clear, &document);
    assert!(
        !app.preview()
            .selection
            .highlights_line(&document, Lane::Unified, 2)
    );
}

#[test]
fn clicking_source_clears_the_navigation_highlight() {
    let mut app = app();
    assert!(app.reveal_source_line(1));
    let document = app.preview().document.clone().unwrap();
    app.preview_mut().selection.update(
        Action::Start(
            Lane::Unified,
            crate::ui::code::selection::Position { row: 0, byte: 0 },
        ),
        &document,
    );
    assert!(
        !app.preview()
            .selection
            .highlights_line(&document, Lane::Unified, 0)
    );
}
