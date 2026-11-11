//! Prepare immutable, highlighted lines for the source viewer.

pub(crate) mod comparison;
pub(crate) mod folding;
pub(crate) mod frozen_gutter;
pub(crate) mod overview;
mod preparation;
mod rows;
pub(crate) mod selection;

pub(crate) use rows::{view as rows, visible_range as visible_rows};

use super::theme::{Tone, syntax};
use crate::domain::changes::Source;
use iced::Color;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, LazyLock},
};
use two_face::re_exports::syntect::{highlighting::Theme, parsing::SyntaxSet};

pub(crate) static SYNTAXES: LazyLock<SyntaxSet> =
    LazyLock::new(two_face::syntax::extra_no_newlines);
pub(crate) static PALETTE: LazyLock<Theme> = LazyLock::new(syntax::palette);

#[derive(Debug, Clone)]
pub(crate) struct Document {
    pub text: String,
    pub source_lines: Vec<Arc<str>>,
    pub lines: Vec<Arc<Vec<(String, Tone)>>>,
    pub columns: usize,
    pub line_info: Vec<LineInfo>,
    pub comparison_failed: bool,
    pub change_counts: [usize; 3],
    pub fingerprint: [u8; 32],
    pub comparison: Option<comparison::Comparison>,
    pub page: Option<crate::domain::changes::TextPage>,
    memory_bytes: usize,
    syntax: Option<Arc<preparation::Syntax>>,
    expanded_tabs: HashMap<usize, Arc<str>>,
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
        preparation::prepare(text, None, None, None)
    }

    pub(crate) fn can_fold(&self) -> bool {
        let changed = self.change_counts.iter().sum::<usize>();
        !self.comparison_failed && changed > 0 && changed < self.line_info.len()
    }

    pub(crate) fn new(text: String, extension: &str) -> Self {
        preparation::prepare(text, Some(extension), None, None)
    }

    fn new_reusing(
        text: String,
        extension: &str,
        previous: &Self,
        equal: Option<Vec<(std::ops::Range<usize>, std::ops::Range<usize>)>>,
    ) -> Self {
        preparation::prepare(text, Some(extension), Some(previous), equal)
    }

    pub(crate) fn has_syntax(&self) -> bool {
        self.syntax.is_some() && self.comparison.is_none()
    }

    pub(crate) fn display_line(&self, index: usize) -> &str {
        self.expanded_tabs.get(&index).map_or_else(
            || {
                let source = self.source_lines[index].as_ref();
                if source.is_empty() { " " } else { source }
            },
            AsRef::as_ref,
        )
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
        comparison::apply(self, source, extension, false).measure_memory()
    }

    /// Keep both sides visible when an explicit comparison finds identical content.
    pub(crate) fn with_explicit_comparison(self, source: Source, extension: &str) -> Self {
        comparison::apply(self, source, extension, true).measure_memory()
    }

    pub(crate) fn memory_bytes(&self) -> usize {
        self.memory_bytes
    }

    fn measure_memory(mut self) -> Self {
        // Shared allocations count once within a document and conservatively across tabs.
        let mut seen = HashSet::new();
        self.memory_bytes = std::mem::size_of::<Self>()
            + self.text.capacity()
            + self.source_lines.capacity() * std::mem::size_of::<Arc<str>>()
            + self
                .source_lines
                .iter()
                .filter(|line| seen.insert(line.as_ptr() as usize))
                .map(|line| line.len() + 2 * std::mem::size_of::<usize>())
                .sum::<usize>()
            + self.lines.capacity() * std::mem::size_of::<Arc<Vec<(String, Tone)>>>()
            + self
                .lines
                .iter()
                .filter(|line| seen.insert(Arc::as_ptr(line) as usize))
                .map(|line| {
                    std::mem::size_of::<Vec<(String, Tone)>>()
                        + 2 * std::mem::size_of::<usize>()
                        + line.capacity() * std::mem::size_of::<(String, Tone)>()
                        + line.iter().map(|(text, _)| text.capacity()).sum::<usize>()
                })
                .sum::<usize>()
            + self.expanded_tabs.capacity() * std::mem::size_of::<(usize, Arc<str>)>()
            + self
                .expanded_tabs
                .values()
                .filter(|text| seen.insert(text.as_ptr() as usize))
                .map(|text| text.len() + 2 * std::mem::size_of::<usize>())
                .sum::<usize>()
            + self.line_info.capacity() * std::mem::size_of::<LineInfo>()
            + self
                .comparison
                .as_ref()
                .map_or(0, comparison::Comparison::memory_bytes)
            + self
                .syntax
                .as_ref()
                .map_or(0, |syntax| syntax.reserved_bytes());
        self
    }

    pub(crate) fn fingerprint(source: &Source) -> [u8; 32] {
        Self::fingerprint_parts(
            &source.text,
            source.deleted,
            source.comparison_failed,
            source.previous.as_deref(),
        )
    }

    fn fingerprint_parts(
        text: &str,
        deleted: bool,
        comparison_failed: bool,
        previous: Option<&str>,
    ) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(text.len().to_le_bytes());
        hash.update(text.as_bytes());
        hash.update([
            u8::from(deleted),
            u8::from(comparison_failed),
            u8::from(previous.is_some()),
        ]);
        if let Some(previous) = previous {
            hash.update(previous.as_bytes());
        }
        hash.finalize().into()
    }
}

pub(crate) fn syntax_for(path: &std::path::Path) -> String {
    let token = path
        .extension()
        .or_else(|| path.file_name())
        .and_then(|value| value.to_str())
        .unwrap_or("txt");
    if token == "C" {
        return "cpp".to_owned();
    }
    match token.to_ascii_lowercase().as_str() {
        "cpp" | "cc" | "cxx" | "c++" | "h" | "hpp" | "hh" | "hxx" | "h++" | "ipp" | "tpp"
        | "inl" | "ixx" | "cppm" | "cxxm" => "cpp".to_owned(),
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
    let mut emphasis_index = 0;
    for (content, color) in line {
        let end = offset + content.len();
        let mut cursor = offset;
        while emphasis_index < emphasis.len() && emphasis[emphasis_index].end <= offset {
            emphasis_index += 1;
        }
        for range in emphasis[emphasis_index..]
            .iter()
            .take_while(|range| range.start < end)
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
