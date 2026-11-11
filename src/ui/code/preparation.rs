//! Reuse highlighted blocks only when their text and complete parser states match.

use super::{Document, LineInfo, PALETTE, SYNTAXES, Tone, syntax};
use similar::{DiffTag, TextDiff};
use std::{collections::HashMap, ops::Range, sync::Arc, time::Duration};
use two_face::re_exports::syntect::{
    easy::HighlightLines, highlighting::HighlightState, parsing::ParseState,
};

const BLOCK: usize = 64;
const INTERNED_LINES: usize = 4096;
type State = (HighlightState, ParseState);
type Line = Arc<Vec<(String, Tone)>>;

#[derive(Debug)]
pub(super) struct Syntax {
    extension: String,
    checkpoints: Vec<(usize, State)>,
}

impl Syntax {
    pub(super) fn reserved_bytes(&self) -> usize {
        // Syntect owns opaque stacks. Reserve additional space for each bounded checkpoint.
        self.extension.capacity()
            + self.checkpoints.capacity() * (std::mem::size_of::<State>() + 4096)
    }
}

pub(super) fn prepare(
    text: String,
    extension: Option<&str>,
    previous: Option<&Document>,
    equal_ranges: Option<Vec<(Range<usize>, Range<usize>)>>,
) -> Document {
    if let Some(previous) = previous.filter(|document| {
        document.has_syntax()
            && document.page.is_none()
            && document.text == text
            && document
                .syntax
                .as_ref()
                .is_some_and(|syntax| Some(syntax.extension.as_str()) == extension)
    }) {
        let mut document = previous.clone();
        document.text = text;
        document.fingerprint = Document::fingerprint_parts(&document.text, false, false, None);
        document.comparison_failed = false;
        return document;
    }
    let mut originals = HashMap::<&str, Arc<str>>::new();
    let source_lines: Vec<_> = text
        .lines()
        .map(|line| {
            if let Some(shared) = originals.get(line) {
                return Arc::clone(shared);
            }
            let shared: Arc<str> = Arc::from(line);
            if line.len() <= 256 && originals.len() < INTERNED_LINES {
                originals.insert(line, Arc::clone(&shared));
            }
            shared
        })
        .collect();
    drop(originals);
    let expanded_tabs: HashMap<usize, Arc<str>> = source_lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains('\t'))
        .map(|(index, line)| (index, Arc::from(line.replace('\t', "    "))))
        .collect();
    let columns = source_lines
        .iter()
        .map(|line| line.chars().map(|c| if c == '\t' { 4 } else { 1 }).sum())
        .max()
        .unwrap_or(0);
    let reference = previous.filter(|document| {
        document.comparison.is_none()
            && document.syntax.as_ref().is_some_and(|syntax| {
                !syntax.checkpoints.is_empty() && Some(syntax.extension.as_str()) == extension
            })
    });
    let equal: Vec<(Range<usize>, Range<usize>)> = equal_ranges.unwrap_or_else(|| {
        reference
            .map(|reference| {
                TextDiff::configure()
                    .timeout(Duration::from_millis(300))
                    .diff_lines(reference.text.as_str(), text.as_str())
                    .ops()
                    .iter()
                    .filter(|op| op.tag() == DiffTag::Equal)
                    .map(|op| (op.old_range(), op.new_range()))
                    .collect()
            })
            .unwrap_or_default()
    });
    let selected = extension.map(|extension| {
        SYNTAXES
            .find_syntax_by_token(extension)
            .unwrap_or_else(|| SYNTAXES.find_syntax_plain_text())
    });
    let plain =
        selected.is_none_or(|selected| std::ptr::eq(selected, SYNTAXES.find_syntax_plain_text()));
    let mut highlighter = selected
        .filter(|_| !plain)
        .map(|syntax| HighlightLines::new(syntax, &PALETTE));
    let mut checkpoints = Vec::new();
    let mut plain_lines = HashMap::<Arc<str>, Line>::new();
    let mut lines = Vec::with_capacity(source_lines.len());
    let mut index = 0;
    let mut equal_index = 0;
    while index < source_lines.len() {
        while equal_index < equal.len() && equal[equal_index].1.end <= index {
            equal_index += 1;
        }
        let at_reference_checkpoint = reference.is_some_and(|reference| {
            equal.get(equal_index).is_some_and(|(old, new)| {
                new.contains(&index)
                    && reference.syntax.as_ref().is_some_and(|syntax| {
                        syntax
                            .checkpoints
                            .binary_search_by_key(&(old.start + index - new.start), |(line, _)| {
                                *line
                            })
                            .is_ok()
                    })
            })
        });
        let state = if index % BLOCK == 0 || at_reference_checkpoint {
            highlighter.take().map(HighlightLines::state)
        } else {
            None
        };
        if let Some(state) = &state {
            // Keep checkpoints sparse, including boundaries after reused blocks.
            if index % BLOCK == 0 {
                checkpoints.push((index, state.clone()));
            }
        }
        let reusable = reference
            .zip(state.as_ref())
            .and_then(|(reference, state)| {
                let (old, new) = equal.get(equal_index)?;
                if !new.contains(&index) {
                    return None;
                }
                let old_index = old.start + index - new.start;
                let saved = &reference.syntax.as_ref()?.checkpoints;
                let checkpoint = saved
                    .binary_search_by_key(&old_index, |(line, _)| *line)
                    .ok()?;
                let (start, before) = &saved[checkpoint];
                let (end, after) = saved.get(checkpoint + 1)?;
                if state != before || end - start > new.end - index {
                    return None;
                }
                Some((*start..*end, after.clone()))
            });
        if let Some((range, after)) = reusable {
            let reference = reference.unwrap();
            lines.extend(reference.lines[range.clone()].iter().cloned());
            index += range.len();
            highlighter = Some(HighlightLines::from_state(&PALETTE, after.0, after.1));
            continue;
        }
        if let Some((highlight, parse)) = state {
            highlighter = Some(HighlightLines::from_state(&PALETTE, highlight, parse));
        }
        let original = &source_lines[index];
        let line = if plain {
            if let Some(shared) = plain_lines.get(original) {
                Arc::clone(shared)
            } else {
                let line = Arc::new(vec![(display(original), syntax::TEXT)]);
                if original.len() <= 256 && plain_lines.len() < INTERNED_LINES {
                    plain_lines.insert(Arc::clone(original), Arc::clone(&line));
                }
                line
            }
        } else {
            let display = expanded_tabs
                .get(&index)
                .map_or(original.as_ref(), AsRef::as_ref);
            let mut spans = (display.len() <= 16 * 1024)
                .then(|| {
                    highlighter
                        .as_mut()?
                        .highlight_line(display, &SYNTAXES)
                        .ok()
                })
                .flatten()
                .map(|spans| {
                    spans
                        .into_iter()
                        .map(|(style, text)| (text.to_owned(), syntax::tone(style.foreground)))
                        .collect()
                })
                .unwrap_or_else(|| vec![(display.to_owned(), syntax::TEXT)]);
            if display.is_empty() {
                spans = vec![(" ".to_owned(), syntax::TEXT)];
            }
            Arc::new(spans)
        };
        lines.push(line);
        index += 1;
    }
    if let Some(highlighter) = highlighter {
        checkpoints.push((source_lines.len(), highlighter.state()));
    }
    Document {
        fingerprint: Document::fingerprint_parts(&text, false, false, None),
        line_info: (1..=lines.len())
            .map(|number| LineInfo {
                number,
                change: None,
            })
            .collect(),
        text,
        source_lines,
        lines,
        columns,
        comparison_failed: false,
        change_counts: [0; 3],
        comparison: None,
        page: None,
        memory_bytes: 0,
        expanded_tabs,
        syntax: extension.map(|extension| {
            Arc::new(Syntax {
                extension: extension.to_owned(),
                checkpoints,
            })
        }),
    }
    .measure_memory()
}

fn display(line: &str) -> String {
    if line.is_empty() {
        " ".to_owned()
    } else {
        line.replace('\t', "    ")
    }
}
