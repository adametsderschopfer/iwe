//! Prepare immutable, highlighted lines for the source viewer.

pub(crate) mod folding;
mod rows;

pub(crate) use rows::view as rows;

use super::theme::syntax;
use crate::domain::changes::Source;
use iced::Color;
use sha2::{Digest, Sha256};
use similar::{DiffTag, TextDiff};
use std::{sync::LazyLock, time::Duration};
use two_face::re_exports::syntect::{
    easy::HighlightLines, highlighting::Theme, parsing::SyntaxSet,
};

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_no_newlines);
static PALETTE: LazyLock<Theme> = LazyLock::new(syntax::palette);

#[derive(Debug, Clone)]
pub(crate) struct Document {
    pub text: String,
    pub lines: Vec<Vec<(String, Color)>>,
    pub columns: usize,
    pub line_info: Vec<LineInfo>,
    pub comparison_failed: bool,
    pub change_counts: [usize; 3],
    pub fingerprint: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineChange {
    Added,
    Modified,
    Removed,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LineInfo {
    pub number: usize,
    pub change: Option<LineChange>,
}

impl Document {
    pub(crate) fn plain(text: String) -> Self {
        let mut columns = 0;
        let lines: Vec<_> = text
            .lines()
            .map(|line| {
                let line = line.replace('\t', "    ");
                columns = columns.max(line.chars().count());
                vec![(
                    if line.is_empty() {
                        " ".to_owned()
                    } else {
                        line
                    },
                    syntax::TEXT,
                )]
            })
            .collect();
        Self {
            fingerprint: Self::fingerprint(&Source {
                text: text.clone(),
                ..Source::default()
            }),
            line_info: (1..=lines.len())
                .map(|number| LineInfo {
                    number,
                    change: None,
                })
                .collect(),
            text,
            lines,
            columns,
            comparison_failed: false,
            change_counts: [0; 3],
        }
    }

    pub(crate) fn can_fold(&self) -> bool {
        let changed = self.change_counts.iter().sum::<usize>();
        !self.comparison_failed && changed > 0 && changed < self.line_info.len()
    }

    pub(crate) fn new(text: String, extension: &str) -> Self {
        let syntax = SYNTAXES
            .find_syntax_by_token(extension)
            .unwrap_or_else(|| SYNTAXES.find_syntax_plain_text());
        let mut highlighter = HighlightLines::new(syntax, &PALETTE);
        let mut columns = 0;
        let lines: Vec<_> = text
            .lines()
            .map(|line| {
                let line = line.replace('\t', "    ");
                columns = columns.max(line.chars().count());
                let highlights = highlighter
                    .highlight_line(&line, &SYNTAXES)
                    .map(|spans| {
                        spans
                            .into_iter()
                            .map(|(style, content)| {
                                let color = style.foreground;
                                (
                                    content.to_owned(),
                                    Color::from_rgb8(color.r, color.g, color.b),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_else(|_| vec![(line.clone(), syntax::TEXT)]);
                if line.is_empty() {
                    vec![(" ".to_owned(), syntax::TEXT)]
                } else {
                    highlights
                }
            })
            .collect();
        Self {
            fingerprint: Self::fingerprint(&Source {
                text: text.clone(),
                ..Source::default()
            }),
            line_info: (1..=lines.len())
                .map(|number| LineInfo {
                    number,
                    change: None,
                })
                .collect(),
            text,
            lines,
            columns,
            comparison_failed: false,
            change_counts: [0; 3],
        }
    }

    pub(crate) fn from_source(source: Source, extension: &str) -> Self {
        let document = Self::new(source.text.clone(), extension);
        document.with_comparison(source, extension)
    }

    pub(crate) fn with_comparison(self, source: Source, extension: &str) -> Self {
        let fingerprint = Self::fingerprint(&source);
        let mut document = self;
        document.fingerprint = fingerprint;
        document.comparison_failed = source.comparison_failed;
        if source.deleted {
            for info in &mut document.line_info {
                info.change = Some(LineChange::Removed);
            }
            document.change_counts[2] = document.lines.len();
            return document;
        }
        let Some(previous) = source
            .previous
            .filter(|previous| previous != &document.text)
        else {
            return document;
        };
        let before = Self::new(previous, extension);
        let diff = TextDiff::configure()
            .timeout(Duration::from_millis(300))
            .diff_lines(before.text.as_str(), document.text.as_str());
        let mut lines = Vec::new();
        let mut line_info = Vec::new();
        for op in diff.ops() {
            if matches!(op.tag(), DiffTag::Delete | DiffTag::Replace) {
                for index in op.old_range() {
                    lines.push(before.lines[index].clone());
                    line_info.push(LineInfo {
                        number: index + 1,
                        change: Some(LineChange::Removed),
                    });
                }
            }
            for index in op.new_range() {
                let change = match op.tag() {
                    DiffTag::Insert => Some(LineChange::Added),
                    DiffTag::Replace if index - op.new_range().start < op.old_range().len() => {
                        Some(LineChange::Modified)
                    }
                    DiffTag::Replace => Some(LineChange::Added),
                    _ => None,
                };
                lines.push(document.lines[index].clone());
                line_info.push(LineInfo {
                    number: index + 1,
                    change,
                });
            }
        }
        document.columns = document.columns.max(before.columns);
        document.lines = lines;
        document.line_info = line_info;
        for info in &document.line_info {
            if let Some(change) = info.change {
                document.change_counts[match change {
                    LineChange::Added => 0,
                    LineChange::Modified => 1,
                    LineChange::Removed => 2,
                }] += 1;
            }
        }
        document
    }

    pub(crate) fn fingerprint(source: &Source) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(source.text.len().to_le_bytes());
        hash.update(source.text.as_bytes());
        hash.update([
            u8::from(source.deleted),
            u8::from(source.comparison_failed),
            u8::from(source.previous.is_some()),
        ]);
        if let Some(previous) = &source.previous {
            hash.update(previous.as_bytes());
        }
        hash.finalize().into()
    }
}

pub(crate) fn syntax_for(path: &std::path::Path) -> String {
    path.extension()
        .or_else(|| path.file_name())
        .and_then(|value| value.to_str())
        .unwrap_or("txt")
        .to_owned()
}

#[cfg(test)]
mod tests;
