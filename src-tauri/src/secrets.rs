//! API-key storage via OS credential store (Windows Credential Manager).
//! One entry per provider so switching providers preserves all keys.

use anyhow::{Context, Result};
use keyring::Entry;

use crate::settings::Provider;

const SERVICE: &str = "app.aiai.desktop";

fn entry_for(provider: Provider) -> Result<Entry> {
    let username = format!("api-key-{}", provider.slug());
    Entry::new(SERVICE, &username).context("create keyring entry")
}

pub fn set_api_key(provider: Provider, value: &str) -> Result<()> {
    entry_for(provider)?
        .set_password(value)
        .context("write key to credential manager")
}

pub fn get_api_key(provider: Provider) -> Result<Option<String>> {
    match entry_for(provider)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}

pub fn clear_api_key(provider: Provider) -> Result<()> {
    match entry_for(provider)?.delete_credential() {
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}

pub fn has_api_key(provider: Provider) -> bool {
    matches!(get_api_key(provider), Ok(Some(_)))
}

pub fn clear_all() {
    for p in [
        Provider::Anthropic,
        Provider::OpenAI,
        Provider::OpenRouter,
        Provider::Gemini,
    ] {
        let _ = clear_api_key(p);
    }
}
