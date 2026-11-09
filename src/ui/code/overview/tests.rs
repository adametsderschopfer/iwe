use super::*;
use crate::{domain::changes::Source, ui::code::LineChange};

#[test]
fn overview_positions_span_the_document_and_preserve_tiny_end_changes() {
    let rail = Size::new(4., 500.);
    let first = mark_bounds(&(0..20), 20., 2000., rail).unwrap();
    let middle = mark_bounds(&(50..51), 20., 2000., rail).unwrap();
    let last = mark_bounds(&(99..100), 20., 2000., rail).unwrap();
    assert_eq!(first, Rectangle::new(Point::ORIGIN, Size::new(4., 100.)));
    assert_eq!(middle.y, 250.);
    assert_eq!(last.y + last.height, rail.height);
    let tiny = mark_bounds(&(999_999..1_000_000), 20., 20_000_000., rail).unwrap();
    assert!(tiny.height >= theme::CODE_CHANGE_MARK_MIN_HEIGHT);
    assert_eq!(tiny.y + tiny.height, rail.height);
    let anchor = mark_bounds(&(100..100), 20., 2000., rail).unwrap();
    assert_eq!(
        anchor,
        Rectangle::new(Point::new(0., 497.), Size::new(4., 3.))
    );
    assert!(mark_bounds(&(0..1), 20., 2000., Size::ZERO).is_none());
}

#[test]
fn overview_marks_follow_folding_expansion_and_each_comparison_side() {
    let before = (0..100)
        .map(|index| format!("line {index}\n"))
        .collect::<String>();
    let after = before
        .replace("line 20\n", "changed\n")
        .replace("line 50\n", "")
        .replace("line 90\n", "line 90\nadded\n");
    let document = Document::from_source(
        Source {
            text: after,
            previous: Some(before),
            ..Default::default()
        },
        "txt",
    );
    let comparison = document.comparison.as_ref().unwrap();
    let mut folding = Folding::default();
    let unified = folding.unified_marks(comparison);
    assert_eq!(
        unified.iter().map(|mark| mark.change).collect::<Vec<_>>(),
        vec![
            LineChange::Modified,
            LineChange::Added,
            LineChange::Removed,
            LineChange::Added
        ]
    );
    for mark in unified {
        assert!(
            mark.rows
                .clone()
                .all(|index| comparison.unified_line(index).unwrap().change == Some(mark.change))
        );
    }
    let before_marks: Vec<_> = Marks::Side(folding.side_hunks(comparison), Side::Before)
        .iter()
        .collect();
    let after_marks: Vec<_> = Marks::Side(folding.side_hunks(comparison), Side::After)
        .iter()
        .collect();
    assert!(
        before_marks[2].rows.is_empty(),
        "Insertions leave an anchor on the previous side"
    );
    assert!(
        after_marks[1].rows.is_empty(),
        "Deletions leave an anchor on the current side"
    );
    folding.toggle(&document);
    assert_eq!(folding.unified_marks(comparison)[0].rows.start, 1);
    assert_eq!(folding.side_hunks(comparison)[0].start, 1);
    let super::super::folding::Row::Fold(range) = folding.row(0) else {
        panic!("Opening context must fold")
    };
    folding.expand(&document, range);
    assert_eq!(folding.unified_marks(comparison)[0].rows.start, 20);
    assert_eq!(folding.side_hunks(comparison)[0].start, 20);
    assert!(
        folding
            .unified_marks(comparison)
            .iter()
            .all(|mark| mark.rows.end <= folding.len(&document))
    );
}

#[test]
fn long_adjacent_changes_use_one_overview_mark() {
    let document = Document::from_source(
        Source {
            text: "new\n".repeat(10_000),
            previous: Some(String::new()),
            ..Default::default()
        },
        "txt",
    );
    let comparison = document.comparison.unwrap();
    assert_eq!(
        comparison.unified_marks,
        vec![Mark {
            rows: 0..10_000,
            change: LineChange::Added
        }]
    );
}
