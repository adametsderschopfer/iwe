//! Share highlighted lines between unified and aligned, two-column comparisons.

use super::{Document, LineChange, LineInfo};
use crate::domain::changes::Source;
use similar::{ChangeTag, DiffTag, TextDiff};
use std::{
    ops::Range,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Side {
    Before,
    After,
}

impl Side {
    pub(crate) fn index(self) -> usize {
        match self {
            Self::Before => 0,
            Self::After => 1,
        }
    }

    pub(crate) fn scroll_id(self) -> &'static str {
        match self {
            Self::Before => "source-before",
            Self::After => "source-after",
        }
    }

    pub(crate) fn other(self) -> Self {
        match self {
            Self::Before => Self::After,
            Self::After => Self::Before,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Cell {
    pub index: usize,
    pub number: usize,
    pub emphasis: Vec<Range<usize>>,
}

#[derive(Debug, Clone)]
pub(crate) struct Pair {
    pub cells: [Option<Cell>; 2],
    pub change: Option<LineChange>,
}

#[derive(Debug, Clone)]
pub(crate) struct Hunk {
    pub start: usize,
    pub lengths: [usize; 2],
    pub change: LineChange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Mark {
    pub rows: Range<usize>,
    pub change: LineChange,
}

pub(super) fn push_mark(marks: &mut Vec<Mark>, rows: Range<usize>, change: LineChange) {
    if rows.is_empty() {
        return;
    }
    if let Some(last) = marks.last_mut()
        && last.rows.end == rows.start
        && last.change == change
    {
        last.rows.end = rows.end;
    } else {
        marks.push(Mark { rows, change });
    }
}

impl Hunk {
    pub(crate) fn end(&self) -> usize {
        self.start + self.lengths[0].max(self.lengths[1])
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Comparison {
    pub rows: Vec<Pair>,
    pub hunks: Vec<Hunk>,
    pub columns: [usize; 2],
    pub unified_marks: Vec<Mark>,
    unified_rows: Vec<(usize, Side)>,
}

pub(crate) struct UnifiedLine<'a> {
    pub numbers: [Option<usize>; 2],
    pub change: Option<LineChange>,
    pub emphasis: &'a [Range<usize>],
}

impl Comparison {
    pub(crate) fn unified_line(&self, index: usize) -> Option<UnifiedLine<'_>> {
        let &(row, side) = self.unified_rows.get(index)?;
        let pair = self.rows.get(row)?;
        let cell = pair.cells[side.index()].as_ref()?;
        Some(UnifiedLine {
            numbers: [0, 1].map(|side| {
                pair.cells[side]
                    .as_ref()
                    .filter(|cell| cell.index == index)
                    .map(|cell| cell.number)
            }),
            change: pair.change.map(|change| {
                if side == Side::After {
                    LineChange::Added
                } else {
                    change
                }
            }),
            emphasis: &cell.emphasis,
        })
    }

    fn indexed(mut self, line_count: usize) -> Self {
        self.unified_rows = vec![(0, Side::Before); line_count];
        for (row, pair) in self.rows.iter().enumerate() {
            for side in [Side::Before, Side::After] {
                if let Some(cell) = &pair.cells[side.index()] {
                    self.unified_rows[cell.index] = (row, side);
                }
            }
        }
        for index in 0..line_count {
            if let Some(change) = self.unified_line(index).and_then(|line| line.change) {
                push_mark(&mut self.unified_marks, index..index + 1, change);
            }
        }
        self
    }
}

pub(super) fn apply(mut document: Document, source: Source, extension: &str) -> Document {
    document.fingerprint = Document::fingerprint(&source);
    document.comparison_failed = source.comparison_failed;
    if source.comparison_failed {
        return document;
    }
    if source.deleted {
        for info in &mut document.line_info {
            info.change = Some(LineChange::Removed);
        }
        document.change_counts[2] = document.lines.len();
        document.comparison = Some(
            Comparison {
                rows: (0..document.lines.len())
                    .map(|index| Pair {
                        cells: [Some(cell(index, index + 1)), None],
                        change: Some(LineChange::Removed),
                    })
                    .collect(),
                hunks: vec![Hunk {
                    start: 0,
                    lengths: [document.lines.len(), 0],
                    change: LineChange::Removed,
                }],
                columns: [document.columns, 0],
                ..Default::default()
            }
            .indexed(document.lines.len()),
        );
        return document;
    }
    let Some(previous) = source
        .previous
        .filter(|previous| previous != &document.text)
    else {
        return document;
    };
    let before = Document::new(previous, extension);
    let diff = TextDiff::configure()
        .timeout(Duration::from_millis(300))
        .diff_lines(before.text.as_str(), document.text.as_str());
    let mut comparison = Comparison {
        columns: [before.columns, document.columns],
        ..Default::default()
    };
    let mut lines = Vec::new();
    let mut line_info = Vec::new();
    // One shared deadline bounds word highlighting across the whole document.
    let word_deadline = Instant::now() + Duration::from_millis(150);
    for op in diff.ops() {
        let old = op.old_range();
        let new = op.new_range();
        let old_start = lines.len();
        if matches!(op.tag(), DiffTag::Delete | DiffTag::Replace) {
            for index in old.clone() {
                lines.push(before.lines[index].clone());
                line_info.push(LineInfo {
                    number: index + 1,
                    change: Some(LineChange::Removed),
                });
            }
        }
        let new_start = lines.len();
        for index in new.clone() {
            let change = match op.tag() {
                DiffTag::Insert => Some(LineChange::Added),
                DiffTag::Replace if index - new.start < old.len() => Some(LineChange::Modified),
                DiffTag::Replace => Some(LineChange::Added),
                _ => None,
            };
            lines.push(document.lines[index].clone());
            line_info.push(LineInfo {
                number: index + 1,
                change,
            });
        }
        let change = match op.tag() {
            DiffTag::Equal => None,
            DiffTag::Insert => Some(LineChange::Added),
            DiffTag::Delete => Some(LineChange::Removed),
            DiffTag::Replace => Some(LineChange::Modified),
        };
        if let Some(change) = change {
            comparison.hunks.push(Hunk {
                start: comparison.rows.len(),
                lengths: [old.len(), new.len()],
                change,
            });
        }
        for offset in 0..old.len().max(new.len()) {
            let mut left = (offset < old.len()).then(|| {
                cell(
                    if change.is_none() {
                        new_start + offset
                    } else {
                        old_start + offset
                    },
                    old.start + offset + 1,
                )
            });
            let mut right =
                (offset < new.len()).then(|| cell(new_start + offset, new.start + offset + 1));
            if change == Some(LineChange::Modified)
                && let (Some(left), Some(right)) = (&mut left, &mut right)
                && Instant::now() < word_deadline
            {
                let old: String = lines[left.index]
                    .iter()
                    .map(|(text, _)| text.as_str())
                    .collect();
                let new: String = lines[right.index]
                    .iter()
                    .map(|(text, _)| text.as_str())
                    .collect();
                let [removed, added] = emphasis(&old, &new, word_deadline);
                left.emphasis = removed;
                right.emphasis = added;
            }
            comparison.rows.push(Pair {
                cells: [left, right],
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
    document.comparison = Some(comparison.indexed(document.lines.len()));
    document
}

fn cell(index: usize, number: usize) -> Cell {
    Cell {
        index,
        number,
        emphasis: Vec::new(),
    }
}

fn emphasis(before: &str, after: &str, deadline: Instant) -> [Vec<Range<usize>>; 2] {
    let mut result = [Vec::new(), Vec::new()];
    // Very long lines retain the complete line highlight without tokenizing them.
    if before.len().max(after.len()) > 8192 {
        return result;
    }
    let diff = TextDiff::configure()
        .deadline(deadline)
        .diff_words(before, after);
    let mut offsets = [0, 0];
    for change in diff.iter_all_changes() {
        let length = change.value().len();
        match change.tag() {
            ChangeTag::Equal => {
                offsets[0] += length;
                offsets[1] += length;
            }
            ChangeTag::Delete => {
                result[0].push(offsets[0]..offsets[0] + length);
                offsets[0] += length;
            }
            ChangeTag::Insert => {
                result[1].push(offsets[1]..offsets[1] + length);
                offsets[1] += length;
            }
        }
    }
    result
}

#[cfg(test)]
mod tests;
