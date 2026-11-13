//! Reference external module for IWE Code.
//!
//! Build it with `cargo build --release --example hello-module`, place the
//! binary into `binaries/<target>/hello-module` next to a `manifest.json`
//! file, compress the folder as a `.iwe-module` archive, and install it from
//! Settings → Modules. The module protocol uses one JSON document per line on
//! stdin and stdout. See the module development guide on the website.

use std::io::{BufRead, Write};

fn main() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut clicks = 0_u32;
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Ok(message) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        match message.get("type").and_then(|value| value.as_str()) {
            Some("initialize") => send(&mut stdout, &panel(clicks)),
            Some("event") => {
                if message.get("id").and_then(|value| value.as_str()) == Some("hello-click")
                    && message
                        .pointer("/action/type")
                        .and_then(|value| value.as_str())
                        == Some("click")
                {
                    clicks += 1;
                    send(&mut stdout, &panel(clicks));
                    if clicks == 3 {
                        send(
                            &mut stdout,
                            &serde_json::json!({
                                "type": "notify",
                                "level": "info",
                                "message": "Hello clicked three times!",
                            }),
                        );
                    }
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

fn panel(clicks: u32) -> serde_json::Value {
    serde_json::json!({
        "type": "panel",
        "content": {
            "type": "column",
            "spacing": 12,
            "children": [
                {
                    "type": "text",
                    "style": "heading",
                    "content": "Hello from an external module!",
                },
                {
                    "type": "text",
                    "style": "muted",
                    "content": format!("The hello button was clicked {clicks} times."),
                },
                {
                    "type": "button",
                    "id": "hello-click",
                    "label": "Say hello",
                    "style": "primary",
                },
                { "type": "separator" },
                {
                    "type": "text",
                    "style": "caption",
                    "content": "The host replaces this panel after every event.",
                },
            ],
        },
    })
}
