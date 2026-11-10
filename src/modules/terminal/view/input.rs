//! Encode keyboard input using the terminal's active cursor mode.

use iced::keyboard::{
    Key, Modifiers,
    key::{Code, Named, Physical},
};

pub(super) fn physical_control(key: Physical) -> Option<Vec<u8>> {
    const LETTERS: [Code; 26] = [
        Code::KeyA,
        Code::KeyB,
        Code::KeyC,
        Code::KeyD,
        Code::KeyE,
        Code::KeyF,
        Code::KeyG,
        Code::KeyH,
        Code::KeyI,
        Code::KeyJ,
        Code::KeyK,
        Code::KeyL,
        Code::KeyM,
        Code::KeyN,
        Code::KeyO,
        Code::KeyP,
        Code::KeyQ,
        Code::KeyR,
        Code::KeyS,
        Code::KeyT,
        Code::KeyU,
        Code::KeyV,
        Code::KeyW,
        Code::KeyX,
        Code::KeyY,
        Code::KeyZ,
    ];
    let Physical::Code(code) = key else {
        return None;
    };
    LETTERS
        .iter()
        .position(|letter| *letter == code)
        .map(|index| vec![index as u8 + 1])
}

pub(super) fn encode(
    key: &Key,
    text: Option<&str>,
    modifiers: Modifiers,
    application_cursor: bool,
) -> Option<Vec<u8>> {
    if modifiers.logo() {
        return None;
    }
    if modifiers.control()
        && let Key::Character(value) = key
    {
        let character = value.chars().next()?;
        let code = match character.to_ascii_uppercase() {
            '@' | ' ' => 0,
            'A'..='Z' | '['..='_' => character.to_ascii_uppercase() as u8 & 0x1f,
            '?' => 127,
            _ => return None,
        };
        return Some(vec![code]);
    }
    let sequence = match key {
        Key::Named(Named::Enter) => "\r",
        Key::Named(Named::Backspace) => "\x7f",
        Key::Named(Named::Tab) if modifiers.shift() => "\x1b[Z",
        Key::Named(Named::Tab) => "\t",
        Key::Named(Named::Escape) => "\x1b",
        Key::Named(Named::ArrowUp) if application_cursor => "\x1bOA",
        Key::Named(Named::ArrowDown) if application_cursor => "\x1bOB",
        Key::Named(Named::ArrowRight) if application_cursor => "\x1bOC",
        Key::Named(Named::ArrowLeft) if application_cursor => "\x1bOD",
        Key::Named(Named::ArrowUp) => "\x1b[A",
        Key::Named(Named::ArrowDown) => "\x1b[B",
        Key::Named(Named::ArrowRight) => "\x1b[C",
        Key::Named(Named::ArrowLeft) => "\x1b[D",
        Key::Named(Named::Home) => "\x1b[H",
        Key::Named(Named::End) => "\x1b[F",
        Key::Named(Named::Delete) => "\x1b[3~",
        Key::Named(Named::Insert) => "\x1b[2~",
        Key::Named(Named::PageUp) => "\x1b[5~",
        Key::Named(Named::PageDown) => "\x1b[6~",
        Key::Named(Named::F1) => "\x1bOP",
        Key::Named(Named::F2) => "\x1bOQ",
        Key::Named(Named::F3) => "\x1bOR",
        Key::Named(Named::F4) => "\x1bOS",
        Key::Named(Named::F5) => "\x1b[15~",
        Key::Named(Named::F6) => "\x1b[17~",
        Key::Named(Named::F7) => "\x1b[18~",
        Key::Named(Named::F8) => "\x1b[19~",
        Key::Named(Named::F9) => "\x1b[20~",
        Key::Named(Named::F10) => "\x1b[21~",
        Key::Named(Named::F11) => "\x1b[23~",
        Key::Named(Named::F12) => "\x1b[24~",
        Key::Character(value) => text.filter(|s| !s.is_empty()).unwrap_or(value.as_str()),
        _ => return None,
    };
    let mut bytes = Vec::new();
    if modifiers.alt() {
        bytes.push(0x1b);
    }
    bytes.extend_from_slice(sequence.as_bytes());
    Some(bytes)
}
