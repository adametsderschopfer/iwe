use super::*;

fn compare(before: &str, after: &str) -> Document {
    Document::from_source(
        Source {
            text: after.into(),
            previous: Some(before.into()),
            ..Source::default()
        },
        "py",
    )
}

#[test]
fn aligned_rows_preserve_both_sources_and_original_line_numbers() {
    let before = "keep\nold_a = 1\nold_b = 2\nsame\nremove\nend\n";
    let after = "insert\nkeep\nnew_a = 3\nsame\nend\nextra\n";
    let document = compare(before, after);
    let comparison = document.comparison.as_ref().unwrap();
    for (side, source) in [(Side::Before, before), (Side::After, after)] {
        let cells: Vec<_> = comparison
            .rows
            .iter()
            .filter_map(|row| row.cells[side.index()].as_ref())
            .collect();
        assert_eq!(
            cells.iter().map(|cell| cell.number).collect::<Vec<_>>(),
            (1..=source.lines().count()).collect::<Vec<_>>()
        );
        let actual: Vec<String> = cells
            .iter()
            .map(|cell| {
                document.lines[cell.index]
                    .iter()
                    .map(|(text, _)| text.as_str())
                    .collect()
            })
            .collect();
        assert_eq!(actual, source.lines().collect::<Vec<_>>());
    }
    assert!(comparison.rows.iter().any(|row| row.cells[0].is_none()));
    assert!(comparison.rows.iter().any(|row| row.cells[1].is_none()));
    assert!(
        comparison
            .hunks
            .iter()
            .all(|hunk| hunk.start < hunk.end() && hunk.end() <= comparison.rows.len())
    );
}

#[test]
fn replacement_highlights_both_sides_and_unicode_words_without_splitting_characters() {
    let document = compare(
        "value = 'привет мир' + old_count\n",
        "value = 'пока мир' + new_count\n",
    );
    let comparison = document.comparison.as_ref().unwrap();
    let row = &comparison.rows[0];
    assert_eq!(row.change, Some(LineChange::Modified));
    for (side, expected) in [(Side::Before, "привет"), (Side::After, "пока")] {
        let cell = row.cells[side.index()].as_ref().unwrap();
        let text: String = document.lines[cell.index]
            .iter()
            .map(|(text, _)| text.as_str())
            .collect();
        let emphasized: String = cell
            .emphasis
            .iter()
            .map(|range| &text[range.clone()])
            .collect();
        assert!(emphasized.contains(expected));
        assert!(!emphasized.contains("value"));
    }
}

#[test]
fn added_deleted_and_empty_files_keep_the_missing_side_empty() {
    for (before, after, empty) in [("", "new\n", Side::Before), ("old\n", "", Side::After)] {
        let document = compare(before, after);
        let comparison = document.comparison.unwrap();
        assert!(
            comparison
                .rows
                .iter()
                .all(|row| row.cells[empty.index()].is_none())
        );
        assert_eq!(comparison.hunks[0].lengths[empty.index()], 0);
    }
    let document = Document::from_source(
        Source {
            text: "removed\n".into(),
            deleted: true,
            ..Source::default()
        },
        "txt",
    );
    let comparison = document.comparison.unwrap();
    assert!(comparison.rows[0].cells[0].is_some());
    assert!(comparison.rows[0].cells[1].is_none());
}

#[test]
fn unavailable_or_identical_baselines_do_not_offer_comparison() {
    for source in [
        Source {
            text: "text".into(),
            ..Source::default()
        },
        Source {
            text: "text".into(),
            previous: Some("text".into()),
            ..Source::default()
        },
        Source {
            text: "text".into(),
            previous: Some("old".into()),
            comparison_failed: true,
            ..Source::default()
        },
    ] {
        assert!(Document::from_source(source, "txt").comparison.is_none());
    }
}

#[test]
fn very_long_changed_lines_use_line_highlighting_without_word_work() {
    let before = "x".repeat(9000);
    let after = "y".repeat(9000);
    assert!(
        emphasis(&before, &after, Instant::now() + Duration::from_secs(1))
            .iter()
            .all(Vec::is_empty)
    );
}

#[test]
fn unified_rows_keep_both_line_numbers_and_share_word_highlights() {
    let document = compare(
        "start\nold value\nshared\nremoved\nend\n",
        "insert\nstart\nnew value\nshared\nend\n",
    );
    let comparison = document.comparison.as_ref().unwrap();
    let rows: Vec<_> = (0..document.lines.len())
        .map(|index| comparison.unified_line(index).unwrap())
        .collect();
    assert_eq!(
        rows.iter().map(|row| row.numbers).collect::<Vec<_>>(),
        vec![
            [None, Some(1)],
            [Some(1), Some(2)],
            [Some(2), None],
            [None, Some(3)],
            [Some(3), Some(4)],
            [Some(4), None],
            [Some(5), Some(5)],
        ]
    );
    assert_eq!(
        rows.iter().map(|row| row.change).collect::<Vec<_>>(),
        vec![
            Some(LineChange::Added),
            None,
            Some(LineChange::Modified),
            Some(LineChange::Added),
            None,
            Some(LineChange::Removed),
            None,
        ]
    );
    for (index, expected) in [(2, "old"), (3, "new")] {
        let text: String = document.lines[index]
            .iter()
            .map(|(text, _)| text.as_str())
            .collect();
        let highlighted: String = rows[index]
            .emphasis
            .iter()
            .map(|range| &text[range.clone()])
            .collect();
        assert_eq!(highlighted, expected);
    }
    assert!(comparison.unified_line(document.lines.len()).is_none());
}
