use super::*;
use crate::domain::shortcuts::Command;

#[test]
fn physical_keys_preserve_shortcuts_across_layouts_and_modifier_changes() {
    let binding = Command::Copy.default_binding();
    assert!(matches(
        &binding,
        &Key::Character("с".into()),
        key::Physical::Code(key::Code::KeyC),
        Modifiers::COMMAND
    ));
    assert!(!matches(
        &binding,
        &Key::Character("c".into()),
        key::Physical::Code(key::Code::KeyC),
        Modifiers::COMMAND | Modifiers::SHIFT
    ));
    // Logical characters must not create aliases for another physical shortcut.
    assert!(!matches(
        &binding,
        &Key::Character("c".into()),
        key::Physical::Code(key::Code::KeyK),
        Modifiers::COMMAND
    ));
}
