use super::*;

fn account(provider: Provider, host: &str) -> Result<Account, Failure> {
    Account::from_remote(&Remote {
        provider: Some(provider),
        host: host.to_owned(),
        project: "team/app".to_owned(),
    })
}

#[test]
fn credentials_are_redacted_in_debug_and_round_trip_for_vault_storage() {
    let credential = Credential {
        access_token: Secret::new("private-access".to_owned()),
        refresh_token: Some(Secret::new("private-refresh".to_owned())),
        expires_at: Some(42),
        client_id: Some("public-client".to_owned()),
        username: "developer".to_owned(),
    };
    let debug = format!("{credential:?}");
    assert!(!debug.contains("private-access") && !debug.contains("private-refresh"));
    let decoded: Credential =
        serde_json::from_str(&serde_json::to_string(&credential).unwrap()).unwrap();
    assert_eq!(decoded.access_token.expose(), "private-access");
    assert_eq!(decoded.refresh_token.unwrap().expose(), "private-refresh");
}

#[test]
fn accounts_isolate_hosts_ports_and_providers() {
    let cloud = account(Provider::GitLab, "GITLAB.com:443").unwrap();
    assert!(cloud.browser_login());
    let custom = account(Provider::GitLab, "gitlab.example.com:8443").unwrap();
    assert!(!custom.browser_login());
    assert_ne!(cloud.storage_key(), custom.storage_key());
    assert_ne!(
        custom.storage_key(),
        account(Provider::GitHub, "gitlab.example.com:8443")
            .unwrap()
            .storage_key()
    );
    assert_eq!(custom.api_base(), "https://gitlab.example.com:8443/api/v4");
    assert_eq!(
        account(Provider::GitHub, "github.com").unwrap().api_base(),
        "https://api.github.com"
    );
    for host in ["gitlab.com.evil.example", "gitlab.com:444"] {
        assert!(!account(Provider::GitLab, host).unwrap().browser_login());
    }
    for host in [
        "user@gitlab.com",
        "gitlab.com/path",
        "gitlab.com?x=1",
        "gitlab.com#x",
        "gitlab.com\n",
    ] {
        assert_eq!(account(Provider::GitLab, host), Err(Failure::Read));
    }
}
