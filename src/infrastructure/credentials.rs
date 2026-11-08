//! Store credentials in the operating system's credential vault, never in workspace files.

use crate::domain::{
    Failure,
    auth::{Account, Credential},
};
use zeroize::Zeroizing;

pub(super) trait Store: Send + Sync {
    fn read(&self, account: &Account) -> Result<Option<Credential>, Failure>;
    fn write(&self, account: &Account, credential: &Credential) -> Result<(), Failure>;
    fn delete(&self, account: &Account) -> Result<(), Failure>;
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
