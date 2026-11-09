//! Prepare immutable, highlighted lines for the source viewer.

pub(crate) mod comparison;
pub(crate) mod folding;
pub(crate) mod overview;
mod rows;
pub(crate) mod selection;

pub(crate) use rows::view as rows;

use super::theme::{Tone, syntax};
use crate::domain::changes::Source;
use iced::Color;
use sha2::{Digest, Sha256};
use std::sync::LazyLock;
use two_face::re_exports::syntect::{
    easy::HighlightLines, highlighting::Theme, parsing::SyntaxSet,
};

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_no_newlines);
static PALETTE: LazyLock<Theme> = LazyLock::new(syntax::palette);

#[derive(Debug, Clone)]
pub(crate) struct Document {
    pub text: String,
    pub source_lines: Vec<String>,
    pub lines: Vec<Vec<(String, Tone)>>,
    pub columns: usize,
    pub line_info: Vec<LineInfo>,
    pub comparison_failed: bool,
    pub change_counts: [usize; 3],
    pub fingerprint: [u8; 32],
    pub comparison: Option<comparison::Comparison>,
    pub page: Option<crate::domain::changes::TextPage>,
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
            source_lines: text.lines().map(str::to_owned).collect(),
            text,
            lines,
            columns,
            comparison_failed: false,
            change_counts: [0; 3],
            comparison: None,
            page: None,
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
                if line.len() > 16 * 1024 {
                    return vec![(line, syntax::TEXT)];
                }
                let highlights = highlighter
                    .highlight_line(&line, &SYNTAXES)
                    .map(|spans| {
                        spans
                            .into_iter()
                            .map(|(style, content)| {
                                (content.to_owned(), syntax::tone(style.foreground))
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
            source_lines: text.lines().map(str::to_owned).collect(),
            text,
            lines,
            columns,
            comparison_failed: false,
            change_counts: [0; 3],
            comparison: None,
            page: None,
        }
    }

    pub(crate) fn with_page(mut self, page: Option<crate::domain::changes::TextPage>) -> Self {
        if let Some(page) = page {
            for line in &mut self.line_info {
                line.number += page.cursor.line.saturating_sub(1);
            }
        }
        self.page = page;
        self
    }

    pub(crate) fn from_source(source: Source, extension: &str) -> Self {
        let document = Self::new(source.text.clone(), extension);
        document.with_comparison(source, extension)
    }

    pub(crate) fn with_comparison(self, source: Source, extension: &str) -> Self {
        comparison::apply(self, source, extension)
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
    let token = path.extension()
        .or_else(|| path.file_name())
        .and_then(|value| value.to_str())
        .unwrap_or("txt");
    if token == "C" {
        return "cpp".to_owned();
    }
    match token.to_ascii_lowercase().as_str() {
        "cpp" | "cc" | "cxx" | "c++" | "h" | "hpp" | "hh" | "hxx" | "h++" | "ipp"
        | "tpp" | "inl" | "ixx" | "cppm" | "cxxm" => "cpp".to_owned(),
        "cs" | "csx" => "cs".to_owned(),
        _ => token.to_owned(),
    }
}

pub(crate) fn emphasized_spans<'a>(
    line: &'a [(String, Tone)],
    emphasis: &[std::ops::Range<usize>],
    background: Color,
    theme: &iced::Theme,
) -> Vec<iced::widget::text::Span<'a>> {
    let mut spans = Vec::new();
    let mut offset = 0;
    for (content, color) in line {
        let end = offset + content.len();
        let mut cursor = offset;
        for range in emphasis
            .iter()
            .filter(|range| range.start < end && range.end > offset)
        {
            let start = range.start.max(offset);
            let stop = range.end.min(end);
            if cursor < start {
                spans.push(
                    iced::widget::span(&content[cursor - offset..start - offset])
                        .color(color.resolve(theme)),
                );
            }
            spans.push(
                iced::widget::span(&content[start - offset..stop - offset])
                    .color(color.resolve(theme))
                    .background(background),
            );
            cursor = stop;
        }
        if cursor < end {
            spans.push(iced::widget::span(&content[cursor - offset..]).color(color.resolve(theme)));
        }
        offset = end;
    }
    spans
}

#[cfg(test)]
mod tests;
