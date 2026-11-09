use super::*;

fn request(query: &str) -> String {
    format!("GET /oauth/gitlab/callback?{query} HTTP/1.1\r\nHost: 127.0.0.1:46291\r\n\r\n")
}

#[test]
fn pkce_matches_the_rfc7636_challenge_vector() {
    let verifier = Secret::new("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk".to_owned());
    assert_eq!(
        challenge(&verifier),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
    let first = random_secret().unwrap();
    let second = random_secret().unwrap();
    assert_eq!(first.expose().len(), 43);
    assert_ne!(first.expose(), second.expose());
}

#[test]
fn callback_requires_matching_state_host_method_and_unique_code() {
    let state = Secret::new("expected".to_owned());
    let valid = request("state=expected&code=secret-code");
    assert_eq!(
        parse_callback(valid.as_bytes(), &state)
            .unwrap()
            .unwrap()
            .expose(),
        "secret-code"
    );
    for invalid in [
        request("state=wrong&code=secret"),
        request("code=secret"),
        request("state=expected&code=a&code=b"),
        request("state=expected&state=expected&code=a"),
        valid.replace("GET", "POST"),
        valid.replace("127.0.0.1:46291", "attacker.example"),
        valid.replace("/oauth/gitlab/callback", "/other"),
        valid.replace("HTTP/1.1", "HTTP/1.0"),
        request("state=expected&code="),
    ] {
        assert!(
            parse_callback(invalid.as_bytes(), &state)
                .unwrap()
                .is_none()
        );
    }
    assert!(matches!(
        parse_callback(
            request("state=expected&error=access_denied").as_bytes(),
            &state
        ),
        Err(Failure::AuthDenied)
    ));
    assert!(
        parse_callback(
            request("state=wrong&error=access_denied").as_bytes(),
            &state
        )
        .unwrap()
        .is_none()
    );
}

#[tokio::test]
async fn loopback_callback_does_not_echo_authorization_codes() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let session = Session {
        base: "https://gitlab.com".to_owned(),
        client_id: "public-id".to_owned(),
        state: Secret::new("expected".to_owned()),
        verifier: random_secret().unwrap(),
        authorization_url: String::new(),
        listener: Arc::new(listener),
    };
    let server = tokio::spawn(async move { session.receive_code("Return to IWE Code.").await });
    let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
    socket
        .write_all(request("state=expected&code=private-code").as_bytes())
        .await
        .unwrap();
    let mut response = String::new();
    socket.read_to_string(&mut response).await.unwrap();
    assert!(response.contains("Cache-Control: no-store"));
    assert!(response.contains("Return to IWE Code."));
    assert!(!response.contains("private-code"));
    assert_eq!(server.await.unwrap().unwrap().expose(), "private-code");
}
