use super::*;
use crate::domain::{auth::Secret, repository::Provider};
use std::sync::{
    Barrier,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[derive(Default)]
struct Vault {
    values: Mutex<BTreeMap<String, Credential>>,
    reads: AtomicUsize,
    read_fails: AtomicBool,
    write_fails: AtomicBool,
    delete_fails: AtomicBool,
}

impl Store for Vault {
    fn read(&self, account: &Account) -> Result<Option<Credential>, Failure> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        if self.read_fails.load(Ordering::SeqCst) {
            return Err(Failure::Credentials);
        }
        Ok(self
            .values
            .lock()
            .unwrap()
            .get(&account.storage_key())
            .cloned())
    }

    fn write(&self, account: &Account, credential: &Credential) -> Result<(), Failure> {
        if self.write_fails.load(Ordering::SeqCst) {
            return Err(Failure::Credentials);
        }
        self.values
            .lock()
            .unwrap()
            .insert(account.storage_key(), credential.clone());
        Ok(())
    }

    fn delete(&self, account: &Account) -> Result<(), Failure> {
        if self.delete_fails.load(Ordering::SeqCst) {
            return Err(Failure::Credentials);
        }
        self.values.lock().unwrap().remove(&account.storage_key());
        Ok(())
    }
}

fn account() -> Account {
    Account {
        provider: Provider::GitLab,
        host: "gitlab.example.test".into(),
    }
}

fn credential(token: &str) -> Credential {
    Credential {
        access_token: Secret::new(token.into()),
        refresh_token: None,
        expires_at: None,
        client_id: None,
        username: "tester".into(),
    }
}

#[test]
fn concurrent_requests_share_one_vault_read_and_explicit_refresh_keeps_it() {
    let vault = Arc::new(Vault::default());
    vault.write(&account(), &credential("original")).unwrap();
    let session = Arc::new(SessionStore::new(vault.clone()));
    let barrier = Arc::new(Barrier::new(8));
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                barrier.wait();
                assert_eq!(
                    session
                        .read(&account())
                        .unwrap()
                        .unwrap()
                        .access_token
                        .expose(),
                    "original"
                );
            });
        }
    });
    session.allow_retry(&account()).unwrap();
    assert!(session.read(&account()).unwrap().is_some());
    assert_eq!(vault.reads.load(Ordering::SeqCst), 1);
}

#[test]
fn missing_credentials_are_cached_and_accounts_remain_isolated() {
    let vault = Arc::new(Vault::default());
    let session = SessionStore::new(vault.clone());
    let other_host = Account {
        host: "another.example.test".into(),
        ..account()
    };
    let other_provider = Account {
        provider: Provider::GitHub,
        ..account()
    };
    vault.write(&other_host, &credential("other-host")).unwrap();
    vault
        .write(&other_provider, &credential("other-provider"))
        .unwrap();
    for _ in 0..3 {
        assert!(session.read(&account()).unwrap().is_none());
        assert_eq!(
            session
                .read(&other_host)
                .unwrap()
                .unwrap()
                .access_token
                .expose(),
            "other-host"
        );
        assert_eq!(
            session
                .read(&other_provider)
                .unwrap()
                .unwrap()
                .access_token
                .expose(),
            "other-provider"
        );
    }
    assert_eq!(vault.reads.load(Ordering::SeqCst), 3);
    session.write(&account(), &credential("signed-in")).unwrap();
    assert_eq!(
        session
            .read(&account())
            .unwrap()
            .unwrap()
            .access_token
            .expose(),
        "signed-in"
    );
    assert_eq!(vault.reads.load(Ordering::SeqCst), 3);
}

#[test]
fn denied_access_waits_for_an_explicit_retry_of_that_account() {
    let vault = Arc::new(Vault::default());
    let session = SessionStore::new(vault.clone());
    let other = Account {
        host: "another.example.test".into(),
        ..account()
    };
    vault.read_fails.store(true, Ordering::SeqCst);
    for _ in 0..3 {
        assert!(matches!(
            session.read(&account()),
            Err(Failure::Credentials)
        ));
        assert!(matches!(session.read(&other), Err(Failure::Credentials)));
    }
    assert_eq!(vault.reads.load(Ordering::SeqCst), 2);
    vault.read_fails.store(false, Ordering::SeqCst);
    session.allow_retry(&account()).unwrap();
    assert!(session.read(&account()).unwrap().is_none());
    assert!(matches!(session.read(&other), Err(Failure::Credentials)));
    assert_eq!(vault.reads.load(Ordering::SeqCst), 3);
}

#[test]
fn rotation_and_sign_out_update_the_cache_after_vault_success() {
    let vault = Arc::new(Vault::default());
    vault.write(&account(), &credential("old")).unwrap();
    let session = SessionStore::new(vault.clone());
    assert!(session.read(&account()).unwrap().is_some());
    session.write(&account(), &credential("rotated")).unwrap();
    assert_eq!(
        session
            .read(&account())
            .unwrap()
            .unwrap()
            .access_token
            .expose(),
        "rotated"
    );
    session.delete(&account()).unwrap();
    assert!(session.read(&account()).unwrap().is_none());
    assert_eq!(vault.reads.load(Ordering::SeqCst), 1);
    let restarted = SessionStore::new(vault.clone());
    assert!(restarted.read(&account()).unwrap().is_none());
    assert_eq!(vault.reads.load(Ordering::SeqCst), 2);
}

#[test]
fn failed_mutations_never_publish_unpersisted_tokens_or_keep_using_cached_tokens() {
    let vault = Arc::new(Vault::default());
    vault.write(&account(), &credential("saved")).unwrap();
    let session = SessionStore::new(vault.clone());
    assert!(session.read(&account()).unwrap().is_some());
    vault.write_fails.store(true, Ordering::SeqCst);
    assert_eq!(
        session.write(&account(), &credential("unsaved")),
        Err(Failure::Credentials)
    );
    assert!(matches!(
        session.read(&account()),
        Err(Failure::Credentials)
    ));
    session.allow_retry(&account()).unwrap();
    assert_eq!(
        session
            .read(&account())
            .unwrap()
            .unwrap()
            .access_token
            .expose(),
        "saved"
    );
    vault.delete_fails.store(true, Ordering::SeqCst);
    assert_eq!(session.delete(&account()), Err(Failure::Credentials));
    assert!(matches!(
        session.read(&account()),
        Err(Failure::Credentials)
    ));
    session.allow_retry(&account()).unwrap();
    assert_eq!(
        session
            .read(&account())
            .unwrap()
            .unwrap()
            .access_token
            .expose(),
        "saved"
    );
}
