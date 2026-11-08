use super::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[tokio::test]
async fn authenticated_requests_never_follow_redirects() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let target = url.clone();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0; 2048];
        let count = socket.read(&mut request).await.unwrap();
        let request = String::from_utf8_lossy(&request[..count]).to_ascii_lowercase();
        assert!(request.contains("authorization: bearer test-only-token"));
        socket
            .write_all(
                format!(
                    "HTTP/1.1 302 Found\r\nLocation: {target}/other\r\nContent-Length: 0\r\n\r\n"
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(100), listener.accept())
                .await
                .is_err()
        );
    });
    let http = Http::local();
    let token = Secret::new("test-only-token".to_owned());
    let request = http.get(&url, Some(&token)).unwrap().build().unwrap();
    assert!(request.headers()[AUTHORIZATION].is_sensitive());
    let response = http
        .send(http.get(&url, Some(&token)).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status, StatusCode::FOUND);
    assert_eq!(response.check(true), Err(Failure::Unavailable));
    server.await.unwrap();
}

#[tokio::test]
async fn production_client_rejects_plain_http() {
    let http = Http::new().unwrap();
    assert!(matches!(
        http.send(http.get("http://127.0.0.1:1", None).unwrap())
            .await,
        Err(Failure::Unavailable)
    ));
}

#[test]
fn access_permission_and_rate_limit_errors_remain_distinct() {
    for (status, limited, authenticated, error) in [
        (401, false, true, Failure::Access),
        (404, false, false, Failure::Access),
        (404, false, true, Failure::PermissionDenied),
        (403, false, true, Failure::PermissionDenied),
        (403, true, true, Failure::RateLimited),
        (429, true, false, Failure::RateLimited),
        (503, false, true, Failure::Unavailable),
    ] {
        let response = Response {
            status: StatusCode::from_u16(status).unwrap(),
            limited,
            bytes: Zeroizing::new(Vec::new()),
        };
        assert_eq!(response.check(authenticated), Err(error));
    }
}

#[tokio::test]
async fn oversized_responses_are_rejected_before_reading_the_body() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer = [0; 1024];
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let received = socket.read(&mut buffer).await.unwrap();
            assert!(received > 0, "The request ended before its headers");
            request.extend_from_slice(&buffer[..received]);
            assert!(
                request.len() <= 16 * 1024,
                "Request headers exceeded the test limit"
            );
        }
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 9999999\r\n\r\n")
            .await
            .unwrap();
    });
    let http = Http::local();
    assert!(matches!(
        http.send(http.get(&url, None).unwrap()).await,
        Err(Failure::TooLarge)
    ));
    server.await.unwrap();
}
