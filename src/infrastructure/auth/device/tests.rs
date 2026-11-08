use super::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[tokio::test]
async fn device_flow_exchanges_a_code_without_a_client_secret() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let origin = base.clone();
    let server = tokio::spawn(async move {
        for index in 0..2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = [0; 4096];
            let size = socket.read(&mut bytes).await.unwrap();
            let request = String::from_utf8_lossy(&bytes[..size]);
            assert!(!request.contains("client_secret"));
            let body = if index == 0 {
                assert!(request.starts_with("POST /login/device/code "));
                assert!(request.contains("scope=repo"));
                format!(
                    r#"{{"device_code":"private-device","user_code":"ABCD-EFGH","verification_uri":"{origin}/login/device","expires_in":900,"interval":1}}"#
                )
            } else {
                assert!(request.starts_with("POST /login/oauth/access_token "));
                assert!(request.contains("device_code=private-device"));
                r#"{"access_token":"test-access","token_type":"bearer"}"#.to_owned()
            };
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        }
    });
    let http = Http::local();
    let session = start(&http, &base, "public-id".to_owned()).await.unwrap();
    assert_eq!(session.prompt().code.as_deref(), Some("ABCD-EFGH"));
    let credential = session.complete(&http).await.unwrap();
    assert_eq!(credential.access_token.expose(), "test-access");
    server.await.unwrap();
}

#[tokio::test]
async fn expired_device_sessions_do_not_poll() {
    let session = Session {
        base: "http://127.0.0.1:1".to_owned(),
        client_id: "id".to_owned(),
        device_code: Secret::new("code".to_owned()),
        user_code: "test".to_owned(),
        verification_uri: "https://github.com/login/device".to_owned(),
        deadline: Instant::now(),
        interval: 5,
    };
    assert!(matches!(
        session.complete(&Http::local()).await,
        Err(Failure::AuthExpired)
    ));
}
