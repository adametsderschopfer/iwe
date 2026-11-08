//! Project unchanged source ranges into expandable rows without copying source text.

use super::Document;
use std::{collections::BTreeSet, ops::Range};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Row {
    Line(usize),
    Fold(Range<usize>),
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Folding {
    pub collapsed: bool,
    pub revision: u64,
    rows: Vec<Row>,
    expanded: BTreeSet<usize>,
}

impl Folding {
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
        if !self.collapsed {
            return;
        }
        let mut index = 0;
        while index < document.line_info.len() {
            if document.line_info[index].change.is_some() {
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
    }
}

#[cfg(test)]
mod tests;
