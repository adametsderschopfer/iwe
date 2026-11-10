use super::*;

#[test]
fn undo_redo_and_saved_snapshot_keep_newer_changes() {
    let mut document = Document::new("first\r\n".into());
    document.edit("second\r\n".into());
    let snapshot = document.text.clone();
    document.edit("third\r\n".into());
    document.saved = snapshot;
    assert!(document.dirty());
    document.undo(false);
    assert!(!document.dirty());
    document.undo(true);
    assert_eq!(document.text, "third\r\n");
    assert!(document.dirty());
    document.undo(false);
    document.edit("replacement".into());
    document.undo(true);
    assert_eq!(document.text, "replacement");
}

#[test]
fn oversized_edits_do_not_change_text_or_history() {
    let mut document = Document::new("original".into());
    assert!(!document.edit("x".repeat(LIMIT + 1)));
    assert!(!document.dirty());
    document.undo(false);
    assert_eq!(document.text, "original");
}
