//! Verify protocol encoding and rejected input.

use super::{HostMessage, ModuleMessage, NotifyLevel, encode, parse};

#[test]
fn host_messages_encode_as_lines() {
    assert_eq!(encode(&HostMessage::Shutdown), r#"{"type":"shutdown"}"#);
    let initialize = HostMessage::initialize("en", "/modules");
    let line = encode(&initialize);
    assert!(line.contains(r#""type":"initialize""#));
    assert!(line.contains(r#""api":1"#));
}

#[test]
fn module_messages_parse_from_lines() {
    let panel = parse(r#"{"type":"panel","content":{"type":"text","content":"Hi"}}"#);
    assert!(matches!(panel, Some(ModuleMessage::Panel(_))));
    let notice = parse(r#"{"type":"notify","level":"warning","message":"Check the connection"}"#);
    assert_eq!(
        notice,
        Some(ModuleMessage::Notify {
            level: NotifyLevel::Warning,
            message: "Check the connection".to_owned(),
        })
    );
}

#[test]
fn invalid_lines_are_ignored() {
    assert!(parse("not json").is_none());
    assert!(parse(r#"{"type":"unknown"}"#).is_none());
    assert!(
        parse(&format!(
            r#"{{"type":"notify","level":"info","message":"{}"}}"#,
            "a".repeat(9000)
        ))
        .is_none()
    );
    assert!(parse(r#"{"type":"panel","content":{"type":"progress","value":9.0}}"#).is_none());
}
