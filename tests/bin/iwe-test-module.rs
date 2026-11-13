//! External module test double for integration tests.
//!
//! The binary speaks the host protocol over stdin and stdout. It answers
//! initialization with a panel, counts button clicks, mirrors toggle and text
//! input state, and exits on shutdown.

use std::io::{BufRead, Write};

fn main() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut clicks = 0_u32;
    let mut toggle = false;
    let mut text = String::new();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Ok(message) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        let kind = message.get("type").and_then(|value| value.as_str());
        match kind {
            Some("initialize") => send(&mut stdout, &panel(clicks, toggle, &text)),
            Some("event") => {
                let id = message.get("id").and_then(|value| value.as_str());
                let action = message.get("action").unwrap_or(&serde_json::Value::Null);
                let action_kind = action.get("type").and_then(|value| value.as_str());
                match (id, action_kind) {
                    (Some("click"), Some("click")) => {
                        clicks += 1;
                        send(&mut stdout, &panel(clicks, toggle, &text));
                    }
                    (Some("toggle"), Some("toggle")) => {
                        toggle =
                            action.get("value").and_then(|value| value.as_bool()) == Some(true);
                        send(&mut stdout, &panel(clicks, toggle, &text));
                    }
                    (Some("input"), Some("change")) => {
                        text = action
                            .get("value")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default()
                            .to_owned();
                        send(&mut stdout, &panel(clicks, toggle, &text));
                    }
                    (Some("notify"), Some("click")) => {
                        send(
                            &mut stdout,
                            &serde_json::json!({
                                "type": "notify",
                                "level": "info",
                                "message": "notified",
                            }),
                        );
                    }
                    _ => {}
                }
            }
            Some("shutdown") => break,
            _ => {}
        }
    }
}

fn send(stdout: &mut impl Write, message: &serde_json::Value) {
    if serde_json::to_writer(&mut *stdout, message).is_ok() {
        let _ = writeln!(stdout);
        let _ = stdout.flush();
    }
}

fn panel(clicks: u32, toggle: bool, text: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "panel",
        "content": {
            "type": "column",
            "spacing": 8,
            "children": [
                {
                    "type": "text",
                    "style": "heading",
                    "content": format!("Clicked {clicks} times"),
                },
                {
                    "type": "button",
                    "id": "click",
                    "label": "Click",
                    "style": "primary",
                },
                {
                    "type": "toggle",
                    "id": "toggle",
                    "checked": toggle,
                },
                {
                    "type": "text-input",
                    "id": "input",
                    "value": text,
                    "placeholder": "Type",
                },
                {
                    "type": "button",
                    "id": "notify",
                    "label": "Notify",
                },
            ],
        },
    })
}
