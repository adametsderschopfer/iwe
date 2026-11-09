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
    for (text, color) in spans {
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
