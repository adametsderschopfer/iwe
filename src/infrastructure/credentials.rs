//! Store credentials in the operating system's credential vault, never in workspace files.

use crate::domain::{
    Failure,
    auth::{Account, Credential},
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use zeroize::Zeroizing;

pub(super) trait Store: Send + Sync {
    fn read(&self, account: &Account) -> Result<Option<Credential>, Failure>;
    fn write(&self, account: &Account, credential: &Credential) -> Result<(), Failure>;
    fn delete(&self, account: &Account) -> Result<(), Failure>;
}

/// Share vault reads across requests without persisting credentials outside the vault.
pub(super) struct SessionStore {
    backend: Arc<dyn Store>,
    cache: Mutex<BTreeMap<String, Result<Option<Credential>, Failure>>>,
}

impl SessionStore {
    pub(super) fn new(backend: Arc<dyn Store>) -> Self {
        Self {
            backend,
            cache: Mutex::new(BTreeMap::new()),
        }
    }

    /// Only an explicit retry may repeat a denied vault read.
    pub(super) fn allow_retry(&self, account: &Account) -> Result<(), Failure> {
        let mut cache = self.cache.lock().map_err(|_| Failure::Credentials)?;
        let key = account.storage_key();
        if cache.get(&key).is_some_and(Result::is_err) {
            cache.remove(&key);
        }
        Ok(())
    }
}

impl Store for SessionStore {
    fn read(&self, account: &Account) -> Result<Option<Credential>, Failure> {
        let mut cache = self.cache.lock().map_err(|_| Failure::Credentials)?;
        cache
            .entry(account.storage_key())
            .or_insert_with(|| self.backend.read(account))
            .clone()
    }

    fn write(&self, account: &Account, credential: &Credential) -> Result<(), Failure> {
        let mut cache = self.cache.lock().map_err(|_| Failure::Credentials)?;
        let result = self.backend.write(account, credential);
        cache.insert(
            account.storage_key(),
            result.map(|()| Some(credential.clone())),
        );
        result
    }

    fn delete(&self, account: &Account) -> Result<(), Failure> {
        let mut cache = self.cache.lock().map_err(|_| Failure::Credentials)?;
        let result = self.backend.delete(account);
        cache.insert(account.storage_key(), result.map(|()| None));
        result
    }
}

pub(super) struct SystemStore;

fn entry(account: &Account) -> Result<keyring::Entry, Failure> {
    keyring::Entry::new("app.iwe.credentials", &account.storage_key())
        .map_err(|_| Failure::Credentials)
}

impl Store for SystemStore {
    fn read(&self, account: &Account) -> Result<Option<Credential>, Failure> {
        match entry(account)?.get_password() {
            Ok(value) => serde_json::from_str(&Zeroizing::new(value))
                .map(Some)
                .map_err(|_| Failure::Credentials),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(Failure::Credentials),
        }
    }

    fn write(&self, account: &Account, credential: &Credential) -> Result<(), Failure> {
        let value =
            Zeroizing::new(serde_json::to_string(credential).map_err(|_| Failure::Credentials)?);
        entry(account)?
            .set_password(&value)
            .map_err(|_| Failure::Credentials)
    }

    fn delete(&self, account: &Account) -> Result<(), Failure> {
        match entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(Failure::Credentials),
        }
    }
}

#[cfg(test)]
mod tests;
