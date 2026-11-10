//! Project unchanged source ranges into expandable rows without copying source text.

use super::{
    Document,
    comparison::{Comparison, Hunk, Mark, push_mark},
};
use std::{collections::BTreeSet, ops::Range};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Row {
    Line(usize),
    Fold(Range<usize>),
}

#[derive(Debug, Clone)]
pub(crate) enum SideRow {
    Line(usize),
    Fold {
        range: Range<usize>,
        numbers: [Range<usize>; 2],
    },
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Folding {
    pub collapsed: bool,
    pub revision: u64,
    rows: Vec<Row>,
    side_rows: Vec<SideRow>,
    side_hunks: Vec<Hunk>,
    unified_marks: Vec<Mark>,
    expanded: BTreeSet<usize>,
}

impl Folding {
    pub(crate) fn release_layout(&mut self) {
        self.rows = Vec::new();
        self.side_rows = Vec::new();
        self.side_hunks = Vec::new();
        self.unified_marks = Vec::new();
    }

    pub(crate) fn restore_layout(&mut self, document: &Document) {
        self.rebuild(document);
    }

    pub(crate) fn len(&self, document: &Document) -> usize {
        if self.collapsed {
            self.rows.len()
        } else {
            document.lines.len()
        }
    }

    pub(crate) fn row(&self, index: usize) -> Row {
        if self.collapsed {
            self.rows[index].clone()
        } else {
            Row::Line(index)
        }
    }

    pub(crate) fn side_len(&self, comparison: &Comparison) -> usize {
        if self.collapsed {
            self.side_rows.len()
        } else {
            comparison.rows.len()
        }
    }

    pub(crate) fn side_row(&self, index: usize) -> SideRow {
        if self.collapsed {
            self.side_rows[index].clone()
        } else {
            SideRow::Line(index)
        }
    }

    pub(crate) fn side_hunks<'a>(&'a self, comparison: &'a Comparison) -> &'a [Hunk] {
        if self.collapsed {
            &self.side_hunks
        } else {
            &comparison.hunks
        }
    }

    pub(crate) fn unified_marks<'a>(&'a self, comparison: &'a Comparison) -> &'a [Mark] {
        if self.collapsed {
            &self.unified_marks
        } else {
            &comparison.unified_marks
        }
    }

    pub(crate) fn toggle(&mut self, document: &Document) {
        if document.can_fold() {
            self.collapsed = !self.collapsed;
            self.refresh(document);
        }
    }

    pub(crate) fn refresh(&mut self, document: &Document) {
        self.collapsed &= document.can_fold();
        self.expanded.clear();
        self.rebuild(document);
    }

    pub(crate) fn expand(&mut self, document: &Document, range: Range<usize>) {
        if self.collapsed && self.rows.contains(&Row::Fold(range.clone())) {
            self.expanded.insert(range.start);
            self.rebuild(document);
        }
    }

    fn rebuild(&mut self, document: &Document) {
        self.revision = self.revision.wrapping_add(1);
        self.rows.clear();
        self.side_rows.clear();
        self.side_hunks.clear();
        self.unified_marks.clear();
        if !self.collapsed {
            return;
        }
        let mut index = 0;
        while index < document.line_info.len() {
            if let Some(change) = document.line_info[index].change {
                let change = document
                    .comparison
                    .as_ref()
                    .and_then(|comparison| comparison.unified_line(index))
                    .and_then(|line| line.change)
                    .unwrap_or(change);
                push_mark(
                    &mut self.unified_marks,
                    self.rows.len()..self.rows.len() + 1,
                    change,
                );
                self.rows.push(Row::Line(index));
                index += 1;
                continue;
            }
            let start = index;
            while index < document.line_info.len() && document.line_info[index].change.is_none() {
                index += 1;
            }
            if self.expanded.contains(&start) {
                self.rows.extend((start..index).map(Row::Line));
            } else {
                self.rows.push(Row::Fold(start..index));
            }
        }
        if let Some(comparison) = &document.comparison {
            self.rebuild_sides(comparison);
        }
    }

    fn rebuild_sides(&mut self, comparison: &Comparison) {
        let mut index = 0;
        let mut hunks = comparison.hunks.iter().peekable();
        while index < comparison.rows.len() {
            if let Some(hunk) = hunks.peek()
                && hunk.start == index
            {
                self.side_hunks.push(Hunk {
                    start: self.side_rows.len(),
                    ..(*hunk).clone()
                });
                hunks.next();
            }
            if comparison.rows[index].change.is_some() {
                self.side_rows.push(SideRow::Line(index));
                index += 1;
                continue;
            }
            let start = index;
            while index < comparison.rows.len() && comparison.rows[index].change.is_none() {
                index += 1;
            }
            // Equal rows reference the same unified lines on both sides.
            let first = comparison.rows[start].cells[1].as_ref();
            let last = comparison.rows[index - 1].cells[1].as_ref();
            if let (Some(first), Some(last)) = (first, last) {
                let range = first.index..last.index + 1;
                if self.expanded.contains(&range.start) {
                    self.side_rows.extend((start..index).map(SideRow::Line));
                } else {
                    let numbers = [0, 1].map(|side| {
                        let first = comparison.rows[start].cells[side]
                            .as_ref()
                            .map_or(1, |cell| cell.number);
                        let last = comparison.rows[index - 1].cells[side]
                            .as_ref()
                            .map_or(first, |cell| cell.number);
                        first..last + 1
                    });
                    self.side_rows.push(SideRow::Fold { range, numbers });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
