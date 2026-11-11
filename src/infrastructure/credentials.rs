//! Store provider credentials in private files and share reads within a session.

use super::secret_files;
use crate::domain::{
    Failure,
    auth::{Account, Credential},
};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use zeroize::Zeroizing;

pub(super) trait Store: Send + Sync {
    fn read(&self, account: &Account) -> Result<Option<Credential>, Failure>;
    fn write(&self, account: &Account, credential: &Credential) -> Result<(), Failure>;
    fn delete(&self, account: &Account) -> Result<(), Failure>;
}

/// Share credential reads across requests.
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

    /// Only an explicit retry may repeat a failed credential read.
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

pub(super) struct FileStore;

fn path(account: &Account) -> Result<PathBuf, Failure> {
    secret_files::path("provider", &account.storage_key()).map_err(|_| Failure::Credentials)
}

impl Store for FileStore {
    fn read(&self, account: &Account) -> Result<Option<Credential>, Failure> {
        secret_files::read(&path(account)?)
            .map_err(|_| Failure::Credentials)?
            .map(|value| serde_json::from_slice(&value).map_err(|_| Failure::Credentials))
            .transpose()
    }

    fn write(&self, account: &Account, credential: &Credential) -> Result<(), Failure> {
        let value =
            Zeroizing::new(serde_json::to_vec(credential).map_err(|_| Failure::Credentials)?);
        secret_files::write(&path(account)?, &value).map_err(|_| Failure::Credentials)
    }

    fn delete(&self, account: &Account) -> Result<(), Failure> {
        secret_files::delete(&path(account)?).map_err(|_| Failure::Credentials)
    }
}

#[cfg(test)]
mod tests;
