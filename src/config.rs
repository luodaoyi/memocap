use std::path::PathBuf;

use anyhow::{anyhow, Result};

use crate::paths::Paths;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Local { database: PathBuf },
    Remote { address: String, token: String },
}

#[must_use]
pub fn normalize_opt(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[must_use]
pub fn configured_address() -> Option<String> {
    normalize_opt(std::env::var("MEMOCAP_ADDR").ok().as_deref())
        .or_else(|| normalize_opt(std::env::var("MEMOCAP_URL").ok().as_deref()))
        .or_else(|| normalize_opt(std::env::var("MEMOCAP_ADDRESS").ok().as_deref()))
}

#[must_use]
pub fn configured_token() -> Option<String> {
    normalize_opt(std::env::var("MEMOCAP_TOKEN").ok().as_deref())
}

pub fn require_token() -> Result<String> {
    configured_token().ok_or_else(|| anyhow!("MEMOCAP_TOKEN is required"))
}

pub fn resolve_target_from(
    address: Option<&str>,
    token: Option<&str>,
    database: PathBuf,
) -> Result<Target> {
    match normalize_opt(address) {
        None => Ok(Target::Local { database }),
        Some(address) => {
            let token = normalize_opt(token)
                .ok_or_else(|| anyhow!("MEMOCAP_TOKEN is required when a remote address is set"))?;
            Ok(Target::Remote { address, token })
        }
    }
}

pub fn resolve_target() -> Result<Target> {
    resolve_target_from(
        configured_address().as_deref(),
        configured_token().as_deref(),
        Paths::discover()?.database,
    )
}

#[must_use]
pub fn token_matches(expected: &str, provided: &str) -> bool {
    let expected = expected.as_bytes();
    let provided = provided.as_bytes();
    if expected.is_empty() || expected.len() != provided.len() {
        return false;
    }
    expected
        .iter()
        .zip(provided.iter())
        .fold(0u8, |acc, (left, right)| acc | (left ^ right))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_address_stays_local() {
        let target =
            resolve_target_from(None, Some("secret"), PathBuf::from("/tmp/memocap.db")).unwrap();
        assert_eq!(
            target,
            Target::Local {
                database: PathBuf::from("/tmp/memocap.db")
            }
        );
    }

    #[test]
    fn empty_address_stays_local() {
        let target =
            resolve_target_from(Some("  "), Some("secret"), PathBuf::from("/tmp/memocap.db"))
                .unwrap();
        assert!(matches!(target, Target::Local { .. }));
    }

    #[test]
    fn address_and_token_select_remote() {
        let target = resolve_target_from(
            Some("http://127.0.0.1:8787"),
            Some("secret"),
            PathBuf::from("/tmp/memocap.db"),
        )
        .unwrap();
        assert_eq!(
            target,
            Target::Remote {
                address: "http://127.0.0.1:8787".into(),
                token: "secret".into(),
            }
        );
    }

    #[test]
    fn address_without_token_is_error() {
        assert!(resolve_target_from(
            Some("http://127.0.0.1:8787"),
            None,
            PathBuf::from("/tmp/memocap.db")
        )
        .is_err());
    }

    #[test]
    fn token_reject_wrong_or_empty() {
        assert!(!token_matches("secret", "wrong"));
        assert!(!token_matches("secret", ""));
        assert!(!token_matches("", "secret"));
        assert!(token_matches("secret", "secret"));
    }
}
