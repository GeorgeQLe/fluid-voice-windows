use keyring::Entry;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const SERVICE_NAME: &str = "fluidvoice-windows";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SecretStatus {
    pub key: String,
    pub exists: bool,
}

#[derive(Debug, Error)]
pub enum SecretError {
    #[error("failed to open credential entry: {0}")]
    Entry(#[source] keyring::Error),
    #[error("failed to read credential: {0}")]
    Read(#[source] keyring::Error),
    #[error("failed to write credential: {0}")]
    Write(#[source] keyring::Error),
    #[error("failed to delete credential: {0}")]
    Delete(#[source] keyring::Error),
}

#[derive(Debug, Clone, Default)]
pub struct SecretStore;

impl SecretStore {
    pub fn set(&self, key: &str, secret: &str) -> Result<(), SecretError> {
        let entry = entry_for(key)?;
        entry.set_password(secret).map_err(SecretError::Write)
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, SecretError> {
        let entry = entry_for(key)?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(SecretError::Read(error)),
        }
    }

    pub fn exists(&self, key: &str) -> Result<SecretStatus, SecretError> {
        Ok(SecretStatus {
            key: key.to_string(),
            exists: self.get(key)?.is_some(),
        })
    }

    pub fn delete(&self, key: &str) -> Result<(), SecretError> {
        let entry = entry_for(key)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(SecretError::Delete(error)),
        }
    }
}

fn entry_for(key: &str) -> Result<Entry, SecretError> {
    Entry::new(SERVICE_NAME, key).map_err(SecretError::Entry)
}
