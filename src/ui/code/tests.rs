use super::*;

#[test]
fn syntax_selection_covers_extensions_and_named_build_files() {
    for (path, syntax) in [
        ("src/app.tsx", "tsx"),
        ("Dockerfile", "Dockerfile"),
        ("Makefile", "Makefile"),
        (".gitignore", ".gitignore"),
    ] {
        assert_eq!(syntax_for(std::path::Path::new(path)), syntax);
    }
}

#[test]
fn source_is_preserved_and_unknown_syntax_still_displays_text() {
    let source = "fn main() {\n\tprintln!(\"Hello\");\n}\n";
    let document = Document::new(source.to_owned(), "rs");
    assert_eq!(document.text, source);
    assert_eq!(document.lines.len(), 3);
    assert!(document.lines[0].len() > 1);
    let plain = Document::new("some text".to_owned(), "unknown-extension");
    assert_eq!(
        plain.lines[0]
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<String>(),
        "some text"
    );
}

fn color_at(document: &Document, line: usize, token: &str) -> Tone {
    let spans = &document.lines[line];
    let text: String = spans.iter().map(|(text, _)| text.as_str()).collect();
    let target = text.find(token).expect("Token occurs in the fixture");
    let mut offset = 0;
    for (text, color) in spans.iter() {
        offset += text.len();
        if offset > target {
            return *color;
        }
    }
    panic!("Token has a highlighted span");
}

#[test]
fn python_and_web_tokens_use_the_reference_palette() {
    let python = Document::new(
        "from services import app\n# Initialize the service\n@signals.connect\ndef connect_client(name=None):\n    config = apps.get_app_config(\"client\")\n    return hasattr(config, \"ready\")\n".to_owned(), "py",
    );
    for (line, token, expected) in [
        (0, "from", syntax::KEYWORD),
        (1, "#", syntax::COMMENT),
        (2, "signals", syntax::DECORATOR),
        (3, "connect_client", syntax::FUNCTION),
        (3, "None", syntax::KEYWORD),
        (4, "get_app_config", syntax::FUNCTION),
        (4, "client", syntax::STRING),
        (5, "hasattr", syntax::FUNCTION),
    ] {
        assert_eq!(
            color_at(&python, line, token),
            expected,
            "Python token: {token}"
        );
    }
    let typescript = Document::new(
        "export function greet(name: string) {\n  const count = 42;\n  return \"Hello\" + name;\n}\n".to_owned(), "ts",
    );
    for (line, token, expected) in [
        (0, "export", syntax::KEYWORD),
        (0, "greet", syntax::FUNCTION),
        (1, "42", syntax::NUMBER),
        (2, "Hello", syntax::STRING),
    ] {
        assert_eq!(
            color_at(&typescript, line, token),
            expected,
            "TypeScript token: {token}"
        );
    }
    let html = Document::new("<button class=\"primary\">Save</button>".to_owned(), "html");
    assert_eq!(color_at(&html, 0, "button"), syntax::FIELD);
    assert_eq!(color_at(&html, 0, "class"), syntax::FUNCTION);
    assert_eq!(color_at(&html, 0, "primary"), syntax::STRING);
}

#[test]
fn multiline_strings_and_unknown_syntax_preserve_readable_text() {
    let document = Document::new(
        "message = \"\"\"first\nsecond\"\"\"\nvalue = 42".to_owned(),
        "py",
    );
    assert_eq!(color_at(&document, 0, "first"), syntax::STRING);
    assert_eq!(color_at(&document, 1, "second"), syntax::STRING);
    assert_eq!(color_at(&document, 2, "42"), syntax::NUMBER);
    let plain = Document::new("hello Мир".to_owned(), "unknown-syntax");
    assert_eq!(color_at(&plain, 0, "Мир"), syntax::TEXT);
}

#[test]
fn inline_changes_preserve_source_numbers_and_copy_content() {
    let before = "same\nold\nkeep\nremoved\n";
    let after = "same\nnew\nkeep\nextra\n";
    let document = Document::from_source(
        Source {
            text: after.to_owned(),
            previous: Some(before.to_owned()),
            ..Source::default()
        },
        "txt",
    );
    assert_eq!(document.text, after);
    assert_eq!(
        document
            .line_info
            .iter()
            .map(|info| (info.number, info.change))
            .collect::<Vec<_>>(),
        vec![
            (1, None),
            (2, Some(LineChange::Removed)),
            (2, Some(LineChange::Modified)),
            (3, None),
            (4, Some(LineChange::Removed)),
            (4, Some(LineChange::Modified)),
        ]
    );
    assert_eq!(document.change_counts, [0, 2, 2]);
    let added = Document::from_source(
        Source {
            text: "one\ntwo\n".to_owned(),
            previous: Some(String::new()),
            ..Source::default()
        },
        "txt",
    );
    assert_eq!(added.change_counts, [2, 0, 0]);
    let mixed = Document::from_source(
        Source {
            text: "changed\nadded\n".to_owned(),
            previous: Some("old\n".to_owned()),
            ..Source::default()
        },
        "txt",
    );
    assert_eq!(mixed.change_counts, [1, 1, 1]);
    let removed = Document::from_source(
        Source {
            text: "old\n".to_owned(),
            deleted: true,
            ..Source::default()
        },
        "txt",
    );
    assert_eq!(removed.text, "old\n");
    assert_eq!(removed.change_counts, [0, 0, 1]);
}

#[test]
fn end_of_file_deletions_and_empty_results_keep_removed_rows() {
    for (before, after) in [
        ("old\n", ""),
        ("same\nold\n", "same\n"),
        ("hello\r\n", "hello\n"),
        ("line\n", "line"),
    ] {
        let document = Document::from_source(
            Source {
                text: after.to_owned(),
                previous: Some(before.to_owned()),
                ..Source::default()
            },
            "txt",
        );
        assert_eq!(document.text, after);
        assert_eq!(document.lines.len(), document.line_info.len());
        assert!(document.change_counts[2] > 0);
    }
}

#[test]
fn reused_syntax_matches_fresh_highlighting_across_multiline_state_changes() {
    let original: String = (0..320)
        .map(|i| format!("let value_{i} = \"строка {i}\";\n"))
        .collect();
    let prepared = Document::new(original.clone(), "rs");
    let identical = Document::new_reusing(original.clone(), "rs", &prepared, None);
    assert!(
        prepared
            .lines
            .iter()
            .zip(&identical.lines)
            .all(|(a, b)| Arc::ptr_eq(a, b))
    );
    for changed in [
        original.replacen("value_42", "changed_42", 1),
        format!("// inserted\n{original}"),
        original.lines().skip(3).collect::<Vec<_>>().join("\n"),
        original
            .replacen("let value_42", "/* let value_42", 1)
            .replacen("let value_190", "*/ let value_190", 1),
        original.replace("let value_64", "/* let value_64"),
    ] {
        let reused = Document::new_reusing(changed.clone(), "rs", &prepared, None);
        let fresh = Document::new(changed, "rs");
        assert_eq!(reused.lines, fresh.lines);
        assert_eq!(reused.source_lines, fresh.source_lines);
        assert_eq!(reused.columns, fresh.columns);
    }
    let changed = original.replacen("value_42", "changed_42", 1);
    let reused = Document::new_reusing(changed, "rs", &prepared, None);
    assert!(Arc::ptr_eq(&reused.lines[256], &prepared.lines[256]));
    let plain = Document::new_reusing(original.clone(), "txt", &prepared, None);
    assert_eq!(plain.lines, Document::new(original, "txt").lines);
}

#[test]
fn dense_comparisons_share_line_storage_and_stay_within_the_closed_document_budget() {
    let before = "x\n".repeat(100_000);
    let text = before.replacen('x', "y", 1);
    let document = Document::from_source(
        Source {
            text,
            previous: Some(before),
            ..Source::default()
        },
        "txt",
    );
    assert_eq!(document.comparison.as_ref().unwrap().rows.len(), 100_000);
    assert!(document.memory_bytes() < 16 * 1024 * 1024);
    let cloned = document.clone();
    assert!(Arc::ptr_eq(&document.lines[50_000], &cloned.lines[50_000]));
    assert!(Arc::ptr_eq(
        &document.source_lines[50_000],
        &cloned.source_lines[50_000]
    ));
}

#[test]
fn shared_source_preserves_tabs_unicode_empty_lines_and_line_endings() {
    let source = "\tпривет\r\n\r\nplain\n";
    let prepared = Document::new(source.to_owned(), "rs");
    let reused = Document::new_reusing(source.to_owned(), "rs", &prepared, None);
    assert_eq!(prepared.lines, reused.lines);
    assert_eq!(reused.source_lines[0].as_ref(), "\tпривет");
    assert_eq!(reused.source_lines[1].as_ref(), "");
    assert_eq!(reused.text, source);
}

// Set IWE_PROFILE_COMPARISON to report preparation costs without wall-clock assertions.
#[test]
fn comparison_audit_probe() {
    if std::env::var_os("IWE_PROFILE_COMPARISON").is_none() {
        return;
    }
    use std::{hint::black_box, time::Instant};
    let _ = Document::new("fn main() {}".into(), "rs");
    for (name, count, short, equal) in [
        ("code-8000", 8000, false, false),
        ("code-30000", 30000, false, false),
        ("equal-30000", 30000, false, true),
        ("short-100000", 100000, true, false),
    ] {
        let before: String = (0..count)
            .map(|i| {
                if short {
                    "x\n".to_owned()
                } else {
                    format!("let value_{i} = Some(\"source value {i}\");\n")
                }
            })
            .collect();
        let after = if equal {
            before.clone()
        } else if short {
            before.replacen('x', "y", 1)
        } else {
            before.replace("source value 42\"", "new value 42\"")
        };
        for run in 0..3 {
            let start = Instant::now();
            let doc = Document::new(after.clone(), if short { "txt" } else { "rs" });
            let syntax = start.elapsed().as_secs_f64() * 1000.;
            let start = Instant::now();
            let cloned = black_box(doc.clone());
            let clone_ms = start.elapsed().as_secs_f64() * 1000.;
            drop(cloned);
            let source = Source {
                text: after.clone(),
                previous: Some(before.clone()),
                ..Default::default()
            };
            let start = Instant::now();
            let doc = doc.with_explicit_comparison(source, if short { "txt" } else { "rs" });
            let comparison_ms = start.elapsed().as_secs_f64() * 1000.;
            let mut folding = folding::Folding::default();
            let start = Instant::now();
            folding.toggle(&doc);
            let fold_ms = start.elapsed().as_secs_f64() * 1000.;
            eprintln!(
                "AUDIT {name} run={run} bytes={} syntax_ms={syntax:.3} clone_ms={clone_ms:.3} comparison_ms={comparison_ms:.3} fold_ms={fold_ms:.3} document_bytes={}",
                after.len(),
                doc.memory_bytes()
            );
            black_box(doc);
        }
    }
    for n in [128, 512, 2048, 4096] {
        let line: Vec<_> = (0..n).map(|_| ("x ".to_owned(), syntax::TEXT)).collect();
        let emphasis: Vec<_> = (0..n).map(|i| i * 2..i * 2 + 1).collect();
        let start = Instant::now();
        for _ in 0..20 {
            black_box(emphasized_spans(
                &line,
                &emphasis,
                iced::Color::BLACK,
                &iced::Theme::Dark,
            ));
        }
        eprintln!(
            "AUDIT spans n={n} mean_ms={:.3}",
            start.elapsed().as_secs_f64() * 1000. / 20.
        );
    }
}

#[test]
fn word_highlights_cross_syntax_boundaries_without_losing_unicode_or_colors() {
    let line = vec![
        ("aé".into(), syntax::TEXT),
        ("中🙂".into(), syntax::KEYWORD),
        ("z".into(), syntax::TEXT),
    ];
    let ranges = vec![1..6, 6..10];
    let result = emphasized_spans(&line, &ranges, Color::BLACK, &iced::Theme::Dark);
    assert_eq!(
        result
            .iter()
            .map(|span| span.text.as_ref())
            .collect::<String>(),
        "aé中🙂z"
    );
    assert_eq!(
        result
            .iter()
            .filter(|span| span.highlight.is_some())
            .map(|span| span.text.as_ref())
            .collect::<String>(),
        "é中🙂"
    );
    assert_eq!(
        result.iter().map(|span| span.color).collect::<Vec<_>>(),
        vec![
            Some(syntax::TEXT.resolve(&iced::Theme::Dark)),
            Some(syntax::TEXT.resolve(&iced::Theme::Dark)),
            Some(syntax::KEYWORD.resolve(&iced::Theme::Dark)),
            Some(syntax::KEYWORD.resolve(&iced::Theme::Dark)),
            Some(syntax::TEXT.resolve(&iced::Theme::Dark)),
        ]
    );
}
