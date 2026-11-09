use super::*;
use crate::domain::changes::Source;

fn changed() -> Document {
    Document::from_source(
        Source {
            text: "top\nnew\nkeep\nadded\nbottom\n".to_owned(),
            previous: Some("top\nold\nkeep\nbottom\n".to_owned()),
            ..Source::default()
        },
        "txt",
    )
}

#[test]
fn folding_keeps_every_changed_line_and_expands_only_the_selected_gap() {
    let document = changed();
    let mut folding = Folding::default();
    folding.toggle(&document);
    let visible = (0..folding.len(&document))
        .filter_map(|row| match folding.row(row) {
            Row::Line(index) => Some(index),
            Row::Fold(_) => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        visible,
        document
            .line_info
            .iter()
            .enumerate()
            .filter_map(|(index, info)| info.change.map(|_| index))
            .collect::<Vec<_>>()
    );
    assert!(
        visible
            .iter()
            .any(|&index| document.line_info[index].change
                == Some(super::super::LineChange::Removed))
    );
    assert_eq!(folding.row(0), Row::Fold(0..1));
    folding.expand(&document, 0..1);
    assert_eq!(folding.row(0), Row::Line(0));
    assert!(folding.rows.iter().any(|row| matches!(row, Row::Fold(_))));
    let rows = folding.rows.clone();
    folding.expand(&document, 99..100);
    assert_eq!(folding.rows, rows);
    folding.toggle(&document);
    assert_eq!(folding.len(&document), document.lines.len());
    assert_eq!(document.text, "top\nnew\nkeep\nadded\nbottom\n");
}

#[test]
fn plain_files_and_failed_comparisons_remain_readable() {
    for document in [
        Document::new("same\n".to_owned(), "txt"),
        Document::new(String::new(), "txt"),
        Document::from_source(
            Source {
                text: "new\n".to_owned(),
                previous: Some(String::new()),
                ..Source::default()
            },
            "txt",
        ),
        Document::from_source(
            Source {
                text: "top\nnew\n".to_owned(),
                previous: Some("top\nold\n".to_owned()),
                comparison_failed: true,
                ..Source::default()
            },
            "txt",
        ),
    ] {
        let mut folding = Folding::default();
        folding.toggle(&document);
        assert!(!folding.collapsed);
        assert_eq!(folding.len(&document), document.lines.len());
    }
    let mut folding = Folding::default();
    folding.toggle(&changed());
    let plain = Document::new("restored\n".to_owned(), "txt");
    folding.refresh(&plain);
    assert!(!folding.collapsed);
    assert_eq!(folding.len(&plain), 1);
}

#[test]
fn large_unchanged_ranges_use_single_rows() {
    let before = (0..20_000)
        .map(|n| format!("line {n}\n"))
        .collect::<String>();
    let after = before.replace("line 10000\n", "changed\n");
    let document = Document::from_source(
        Source {
            text: after,
            previous: Some(before),
            ..Source::default()
        },
        "txt",
    );
    let mut folding = Folding::default();
    folding.toggle(&document);
    assert_eq!(folding.len(&document), 4);
    assert_eq!(folding.row(0), Row::Fold(0..10_000));
    assert!(matches!(folding.row(3), Row::Fold(_)));
    let comparison = document.comparison.as_ref().unwrap();
    assert_eq!(folding.side_len(comparison), 3);
    assert_eq!(folding.side_hunks(comparison)[0].start, 1);
    assert!(matches!(folding.side_row(2), SideRow::Fold { .. }));
}

#[test]
fn side_folds_share_expansion_and_keep_each_versions_line_numbers() {
    let document = changed();
    let comparison = document.comparison.as_ref().unwrap();
    let mut folding = Folding::default();
    folding.toggle(&document);
    let index = folding.side_len(comparison) - 1;
    let SideRow::Fold { range, numbers } = folding.side_row(index) else {
        panic!("The final unchanged range must collapse")
    };
    assert_eq!(numbers, [4..5, 5..6]);
    assert!(folding.rows.contains(&Row::Fold(range.clone())));
    folding.expand(&document, range.clone());
    assert!(matches!(folding.side_row(index), SideRow::Line(_)));
    assert!(!folding.rows.contains(&Row::Fold(range)));
    assert!(matches!(folding.side_row(0), SideRow::Fold { .. }));
}
