//! Verify the external module protocol against a running module process.

use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

/// Exchange one request line and collect the module replies.
fn round_trip(script: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let lines: Vec<String> = script
        .iter()
        .map(|message| serde_json::to_string(message).expect("serialize request"))
        .collect();
    let mut child = Command::new(env!("CARGO_BIN_EXE_iwe-test-module"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn test module");
    let mut stdin = child.stdin.take().expect("module stdin");
    let stdout = child.stdout.take().expect("module stdout");
    let sending = thread::spawn(move || {
        for line in lines {
            writeln!(&mut stdin, "{line}").expect("write request");
        }
        // Closing stdin ends the module after it answers.
        drop(stdin);
    });
    let mut replies = Vec::new();
    for line in BufReader::new(stdout).lines() {
        let Ok(line) = line else { break };
        if let Ok(message) = serde_json::from_str(&line) {
            replies.push(message);
        }
    }
    sending.join().expect("writer thread");
    let _ = child.wait();
    replies
}

fn text_content(message: &serde_json::Value) -> Option<&str> {
    message
        .pointer("/content/children/0/content")
        .and_then(|value| value.as_str())
}

#[test]
fn module_speaks_the_host_protocol() {
    let replies = round_trip(&[
        serde_json::json!({"type":"initialize","api":1,"appVersion":"1.0.8","language":"en","installPath":"/tmp"}),
        serde_json::json!({"type":"event","id":"click","action":{"type":"click"}}),
        serde_json::json!({"type":"event","id":"toggle","action":{"type":"toggle","value":true}}),
        serde_json::json!({"type":"event","id":"input","action":{"type":"change","value":"typed"}}),
        serde_json::json!({"type":"shutdown"}),
    ]);
    assert_eq!(replies.len(), 4, "one panel per request except shutdown");
    assert_eq!(text_content(&replies[0]), Some("Clicked 0 times"));
    assert_eq!(text_content(&replies[1]), Some("Clicked 1 times"));
    let toggle = replies[2]
        .pointer("/content/children/2/checked")
        .and_then(|value| value.as_bool());
    assert_eq!(toggle, Some(true));
    let input = replies[3]
        .pointer("/content/children/3/value")
        .and_then(|value| value.as_str());
    assert_eq!(input, Some("typed"));
}

#[test]
fn module_sends_notifications() {
    let replies = round_trip(&[
        serde_json::json!({"type":"initialize","api":1,"appVersion":"1.0.8","language":"en","installPath":"/tmp"}),
        serde_json::json!({"type":"event","id":"notify","action":{"type":"click"}}),
    ]);
    assert_eq!(replies.len(), 2);
    assert_eq!(replies[1].get("type"), Some(&serde_json::json!("notify")));
    assert_eq!(
        replies[1].get("message"),
        Some(&serde_json::json!("notified"))
    );
}

#[test]
fn module_exits_on_shutdown() {
    let start = std::time::Instant::now();
    let mut child = Command::new(env!("CARGO_BIN_EXE_iwe-test-module"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn test module");
    let mut stdin = child.stdin.take().expect("module stdin");
    stdin
        .write_all(br#"{"type":"shutdown"}"#)
        .and_then(|()| stdin.write_all(b"\n"))
        .expect("write shutdown");
    drop(stdin);
    let status = child.wait().expect("wait module");
    assert!(status.success());
    assert!(start.elapsed() < Duration::from_secs(5));
}
