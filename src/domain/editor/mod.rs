//! Track editable text and bounded undo history independently of the interface.

use std::collections::VecDeque;

pub(crate) const LIMIT: usize = 2 * 1024 * 1024;
const HISTORY_LIMIT: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct Document {
    pub text: String,
    pub saved: String,
    undo: VecDeque<String>,
    redo: Vec<String>,
}

impl Document {
    pub(crate) fn new(text: String) -> Self {
        Self {
            saved: text.clone(),
            text,
            undo: VecDeque::new(),
            redo: Vec::new(),
        }
    }

    pub(crate) fn dirty(&self) -> bool {
        self.text != self.saved
    }

    pub(crate) fn edit(&mut self, text: String) -> bool {
        if text.len() > LIMIT || text == self.text {
            return false;
        }
        self.undo.push_back(std::mem::replace(&mut self.text, text));
        self.redo.clear();
        let mut bytes: usize = self.undo.iter().map(String::len).sum();
        while bytes > HISTORY_LIMIT || self.undo.len() > 200 {
            bytes -= self.undo.pop_front().map_or(0, |text| text.len());
        }
        true
    }

    pub(crate) fn undo(&mut self, redo: bool) {
        if redo {
            if let Some(text) = self.redo.pop() {
                self.undo.push_back(std::mem::replace(&mut self.text, text));
            }
        } else if let Some(text) = self.undo.pop_back() {
            self.redo.push(std::mem::replace(&mut self.text, text));
        }
    }
}

#[cfg(test)]
mod tests;
