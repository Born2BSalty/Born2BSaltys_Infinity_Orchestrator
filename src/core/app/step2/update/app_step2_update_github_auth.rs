// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::sync::{Mutex, OnceLock};
const GITHUB_SECURE_STORE_SERVICE_NAME: &str = "BIO";
const GITHUB_SECURE_STORE_ACCOUNT_NAME: &str = "github-oauth";

pub(super) fn load_github_token() -> Option<String> {
    set_last_load_error(None);
    match load_github_token_from_secure_store() {
        Ok(Some(token)) => return Some(token),
        Ok(None) => {}
        Err(err) => set_last_load_error(Some(format!("github secure auth load failed: {err}"))),
    }
    None
}

pub(crate) fn store_github_oauth_token(token: &str) -> Result<(), String> {
    let token = token.trim();
    if token.is_empty() {
        return Err("github oauth token is empty".to_string());
    }
    secure_store_entry()?
        .set_password(token)
        .map_err(|err| err.to_string())
}

pub(crate) fn clear_github_oauth_token() -> Result<(), String> {
    match secure_store_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

fn load_github_token_from_secure_store() -> Result<Option<String>, String> {
    match secure_store_entry()?.get_password() {
        Ok(token) => {
            let token = token.trim();
            if token.is_empty() {
                Ok(None)
            } else {
                Ok(Some(token.to_string()))
            }
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(err.to_string()),
    }
}

fn secure_store_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(
        GITHUB_SECURE_STORE_SERVICE_NAME,
        GITHUB_SECURE_STORE_ACCOUNT_NAME,
    )
    .map_err(|err| err.to_string())
}

pub(super) fn take_last_load_error() -> Option<String> {
    last_load_error()
        .lock()
        .ok()
        .and_then(|mut guard| guard.take())
}

fn last_load_error() -> &'static Mutex<Option<String>> {
    static LAST_LOAD_ERROR: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    LAST_LOAD_ERROR.get_or_init(|| Mutex::new(None))
}

fn set_last_load_error(error: Option<String>) {
    if let Ok(mut guard) = last_load_error().lock() {
        *guard = error;
    }
}
