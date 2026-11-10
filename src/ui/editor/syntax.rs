//! Keep multiline syntax state while the native editor invalidates changed lines.

use crate::ui::{
    code::{PALETTE, SYNTAXES},
    theme::{Tone, syntax},
};
use two_face::re_exports::syntect::{
    easy::HighlightLines, highlighting::HighlightState, parsing::ParseState,
};

pub(super) struct Highlighter {
    states: Vec<(HighlightState, ParseState)>,
    line: usize,
}

impl iced::advanced::text::Highlighter for Highlighter {
    type Settings = String;
    type Highlight = Tone;
    type Iterator<'a> = std::vec::IntoIter<(std::ops::Range<usize>, Tone)>;

    fn new(token: &String) -> Self {
        let language = SYNTAXES
            .find_syntax_by_token(token)
            .unwrap_or_else(|| SYNTAXES.find_syntax_plain_text());
        Self {
            states: vec![HighlightLines::new(language, &PALETTE).state()],
            line: 0,
        }
    }
    fn update(&mut self, token: &String) {
        *self = Self::new(token);
    }
    fn change_line(&mut self, line: usize) {
        self.line = line.min(self.states.len() - 1);
        self.states.truncate(self.line + 1);
    }
    fn current_line(&self) -> usize {
        self.line
    }
    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let (highlight, parse) = self.states[self.line].clone();
        let mut highlighter = HighlightLines::from_state(&PALETTE, highlight, parse);
        let mut offset = 0;
        let spans = if line.len() <= 16 * 1024 {
            highlighter
                .highlight_line(line, &SYNTAXES)
                .unwrap_or_default()
                .into_iter()
                .map(|(style, text)| {
                    let start = offset;
                    offset += text.len();
                    (start..offset, syntax::tone(style.foreground))
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        self.states.push(highlighter.state());
        self.line += 1;
        spans.into_iter()
    }
}
