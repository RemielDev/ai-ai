//! API-key storage using the OS credential store (Windows Credential Manager).
//!
//! Key is NEVER written to settings.json — only the OS keychain.

use anyhow::{Context, Result};
use keyring::Entry;

const SERVICE: &str = "app.aiai.desktop";
const USERNAME: &str = "anthropic-api-key";

fn entry() -> Result<Entry> {
    Entry::new(SERVICE, USERNAME).context("create keyring entry")
}

pub fn set_api_key(value: &str) -> Result<()> {
    entry()?
        .set_password(value)
        .context("write key to credential manager")
}

pub fn get_api_key() -> Result<Option<String>> {
    match entry()?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}

pub fn clear_api_key() -> Result<()> {
    match entry()?.delete_credential() {
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}

pub fn has_api_key() -> bool {
    matches!(get_api_key(), Ok(Some(_)))
}
