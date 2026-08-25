use memocap::cli;
use memocap::config;
use memocap::server;
use std::path::PathBuf;

fn db() -> (tempfile::TempDir, PathBuf) {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("db");
    (d, p)
}

#[test]
fn save() {
    let (_d, db) = db();
    let id = cli::remember(&db, "alpha", "note", "t1").unwrap();
    assert!(id > 0);
    assert_eq!(cli::count(&db).unwrap(), 1);
}

#[test]
fn query() {
    let (_d, db) = db();
    cli::remember(&db, "alpha beta", "note", "").unwrap();
    let found = cli::recall(&db, "alpha", 5).unwrap();
    assert_eq!(found.len(), 1);
}

#[test]
fn list_newest() {
    let (_d, db) = db();
    let a = cli::remember(&db, "first", "note", "").unwrap();
    let b = cli::remember(&db, "second", "note", "").unwrap();
    let listed = cli::list(&db, 20).unwrap();
    assert_eq!(listed[0].id, b);
    assert_eq!(listed[1].id, a);
}

#[test]
fn delete_target() {
    let (_d, db) = db();
    let keep = cli::remember(&db, "keep", "note", "").unwrap();
    let drop = cli::remember(&db, "drop", "note", "").unwrap();
    assert!(cli::forget(&db, drop).unwrap());
    assert_eq!(cli::list(&db, 20).unwrap()[0].id, keep);
}

#[test]
fn empty_store() {
    let (_d, db) = db();
    let listed = cli::list(&db, 20).unwrap();
    assert!(listed.is_empty());
    assert_eq!(cli::format_memories(&listed), "No local memories found.\n");
    assert_eq!(cli::count(&db).unwrap(), 0);
}

#[test]
fn no_result() {
    let (_d, db) = db();
    cli::remember(&db, "only rust sqlite", "note", "").unwrap();
    let found = cli::recall(&db, "python chroma", 5).unwrap();
    assert!(found.is_empty());
    assert_eq!(cli::format_memories(&found), "No local memories found.\n");
}

#[test]
fn no_address_stays_local() {
    let target =
        config::resolve_target_from(None, Some("secret"), PathBuf::from("/tmp/x.db")).unwrap();
    assert!(matches!(target, config::Target::Local { .. }));
    let (_d, db) = db();
    let id = cli::remember(&db, "stays local", "note", "").unwrap();
    assert!(id > 0);
    assert_eq!(cli::count(&db).unwrap(), 1);
}

#[test]
fn token_reject() {
    let (_d, db) = db();
    let connection = memocap::store::open(&db).unwrap();
    let req = server::Incoming {
        method: "GET".to_owned(),
        path: "/count".to_owned(),
        query: String::new(),
        token: None,
        body: String::new(),
    };
    let response = server::handle(&connection, "secret", &req);
    assert_eq!(response.status, 401);
    assert!(response.body.contains("unauthorized"));
    let denied = server::Incoming {
        method: "POST".to_owned(),
        path: "/remember".to_owned(),
        query: String::new(),
        token: Some("wrong".to_owned()),
        body: r#"{"content":"nope"}"#.to_owned(),
    };
    let response = server::handle(&connection, "secret", &denied);
    assert_eq!(response.status, 401);
    assert_eq!(cli::count(&db).unwrap(), 0);
}
