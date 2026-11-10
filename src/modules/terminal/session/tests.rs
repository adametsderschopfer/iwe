use super::*;

#[cfg(windows)]
#[tokio::test]
async fn shell_starts_without_cursor_reply_and_drains_output_before_closing() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().to_path_buf();
    let mut command = CommandBuilder::new("cmd.exe");
    command.args(["/D", "/Q", "/C", "echo FINAL-OUTPUT"]);
    let session = tokio::task::spawn_blocking(move || Session::spawn(&path, 24, 80, command))
        .await
        .unwrap()
        .unwrap();
    let mut output = Vec::new();
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            for event in session.next().await {
                match event {
                    Output::Bytes(bytes) => output.extend(bytes),
                    Output::Closed => return,
                    Output::Failed => panic!("terminal worker failed"),
                }
            }
        }
    })
    .await
    .unwrap();
    assert!(String::from_utf8_lossy(&output).contains("FINAL-OUTPUT"));
}

#[tokio::test]
async fn project_shell_accepts_input_and_reports_output_and_exit() {
    let directory = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    let command = {
        let mut command = CommandBuilder::new("/bin/sh");
        command.args([
            "-c",
            "printf 'READY\\n'; read value; printf 'RESULT:%s\\n' \"$value\"; pwd",
        ]);
        command
    };
    #[cfg(windows)]
    let command = {
        let mut command = CommandBuilder::new("cmd.exe");
        command.args(["/D", "/Q"]);
        command
    };
    let path = directory.path().to_path_buf();
    let session = tokio::task::spawn_blocking(move || Session::spawn(&path, 24, 80, command))
        .await
        .unwrap()
        .unwrap();
    #[cfg(unix)]
    session.input(b"hello-terminal\r".to_vec()).unwrap();
    #[cfg(windows)]
    session
        .input(b"echo RESULT:hello-terminal\r\ncd > terminal-cwd.txt\r\nexit\r\n".to_vec())
        .unwrap();
    let mut output = Vec::new();
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            for event in session.next().await {
                match event {
                    Output::Bytes(bytes) => output.extend(bytes),
                    Output::Closed => return,
                    Output::Failed => panic!("terminal worker failed"),
                }
            }
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!(
            "terminal did not close: {}",
            String::from_utf8_lossy(&output)
        )
    });
    let output = String::from_utf8_lossy(&output);
    assert!(output.contains("RESULT:hello-terminal"), "{output}");
    let expected = std::fs::canonicalize(directory.path()).unwrap();
    #[cfg(unix)]
    assert!(
        output.contains(expected.to_string_lossy().as_ref()),
        "{output}"
    );
    #[cfg(windows)]
    {
        // cmd may print an 8.3 path. Compare filesystem identity, not its spelling.
        let actual = std::fs::read_to_string(directory.path().join("terminal-cwd.txt")).unwrap();
        assert_eq!(std::fs::canonicalize(actual.trim()).unwrap(), expected);
    }
}

#[cfg(unix)]
#[tokio::test]
async fn resize_reaches_the_pseudo_terminal() {
    let directory = tempfile::tempdir().unwrap();
    let mut command = CommandBuilder::new("/bin/sh");
    command.args(["-c", "read value; stty size"]);
    let path = directory.path().to_path_buf();
    let session = tokio::task::spawn_blocking(move || Session::spawn(&path, 24, 80, command))
        .await
        .unwrap()
        .unwrap();
    session.resize(35, 110).unwrap();
    session.input(b"go\r".to_vec()).unwrap();
    let mut output = Vec::new();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            for event in session.next().await {
                match event {
                    Output::Bytes(bytes) => output.extend(bytes),
                    Output::Closed => return,
                    Output::Failed => panic!("terminal worker failed"),
                }
            }
        }
    })
    .await
    .unwrap();
    assert!(String::from_utf8_lossy(&output).contains("35 110"));
}

#[test]
fn missing_project_directory_cannot_start_a_shell() {
    let directory = tempfile::tempdir().unwrap();
    assert!(Session::start(&directory.path().join("missing"), 24, 80).is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn dropping_the_session_stops_its_shell_without_blocking_the_caller() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join("stopped");
    let mut command = CommandBuilder::new("/bin/sh");
    command.env("IWE_TEST_STOP", &marker);
    command.args(["-c", "trap 'printf stopped > \"$IWE_TEST_STOP\"; exit 0' HUP; printf ready; while :; do sleep 0.05; done"]);
    let path = directory.path().to_path_buf();
    let session = tokio::task::spawn_blocking(move || Session::spawn(&path, 24, 80, command))
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), session.next())
        .await
        .unwrap();
    drop(session);
    tokio::time::timeout(Duration::from_secs(5), async {
        while !marker.exists() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(std::fs::read_to_string(marker).unwrap(), "stopped");
}
