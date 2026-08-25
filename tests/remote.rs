use memocap::config::{resolve_target_from, token_matches};
use std::path::PathBuf;

#[test]
fn no_address_stays_local() {
    let target = resolve_target_from(None, Some("tok"), PathBuf::from("/tmp/x.db")).unwrap();
    assert!(matches!(target, memocap::config::Target::Local { .. }));
}

#[test]
fn token_reject() {
    assert!(!token_matches("secret", "wrong"));
    assert!(!token_matches("secret", ""));
    assert!(token_matches("secret", "secret"));
}
