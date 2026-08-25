use std::{fs, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Memory {
    pub id: i64,
    pub content: String,
    pub kind: String,
    pub tags: String,
    pub created_at: String,
}

pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("创建数据目录失败：{}", parent.display()))?;
    }
    let connection = Connection::open(path)
        .with_context(|| format!("打开本地记忆库失败：{}", path.display()))?;
    connection.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;
        CREATE TABLE IF NOT EXISTS memories (
            id INTEGER PRIMARY KEY,
            content TEXT NOT NULL,
            kind TEXT NOT NULL DEFAULT 'context',
            tags TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
            content, tags, content='memories', content_rowid='id'
        );
        CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
            INSERT INTO memories_fts(rowid, content, tags) VALUES (new.id, new.content, new.tags);
        END;
        CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
            INSERT INTO memories_fts(memories_fts, rowid, content, tags)
            VALUES ('delete', old.id, old.content, old.tags);
        END;
        CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
            INSERT INTO memories_fts(memories_fts, rowid, content, tags)
            VALUES ('delete', old.id, old.content, old.tags);
            INSERT INTO memories_fts(rowid, content, tags) VALUES (new.id, new.content, new.tags);
        END;
        ",
    )?;
    Ok(connection)
}

pub fn remember(connection: &Connection, content: &str, kind: &str, tags: &str) -> Result<i64> {
    let content = content.trim();
    if content.is_empty() {
        anyhow::bail!("记忆内容不能为空");
    }
    let kind = kind.trim();
    let kind = if kind.is_empty() { "context" } else { kind };
    let now: DateTime<Utc> = Utc::now();
    connection.execute(
        "INSERT INTO memories (content, kind, tags, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![content, kind, tags.trim(), now.to_rfc3339()],
    )?;
    Ok(connection.last_insert_rowid())
}

pub fn recall(connection: &Connection, query: &str, limit: usize) -> Result<Vec<Memory>> {
    let Some(match_query) = fts_query(query) else {
        return Ok(Vec::new());
    };
    let mut statement = connection.prepare(
        "SELECT m.id, m.content, m.kind, m.tags, m.created_at
         FROM memories_fts f
         JOIN memories m ON m.id = f.rowid
         WHERE memories_fts MATCH ?1
         ORDER BY bm25(memories_fts), m.id DESC
         LIMIT ?2",
    )?;
    let rows = statement.query_map(params![match_query, limit], memory_from_row)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn list(connection: &Connection, limit: usize) -> Result<Vec<Memory>> {
    let mut statement = connection.prepare(
        "SELECT id, content, kind, tags, created_at FROM memories ORDER BY id DESC LIMIT ?1",
    )?;
    let rows = statement.query_map(params![limit], memory_from_row)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn forget(connection: &Connection, id: i64) -> Result<bool> {
    Ok(connection.execute("DELETE FROM memories WHERE id = ?1", params![id])? > 0)
}

pub fn count(connection: &Connection) -> Result<i64> {
    Ok(connection.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?)
}

fn memory_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Memory> {
    Ok(Memory {
        id: row.get(0)?,
        content: row.get(1)?,
        kind: row.get(2)?,
        tags: row.get(3)?,
        created_at: row.get(4)?,
    })
}

fn fts_query(query: &str) -> Option<String> {
    let parts: Vec<String> = query
        .split_whitespace()
        .map(|part| part.replace('"', ""))
        .filter(|part| !part.is_empty())
        .map(|part| format!("\"{part}\""))
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" AND "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_temp() -> (tempfile::TempDir, Connection) {
        let directory = tempfile::tempdir().unwrap();
        let connection = open(&directory.path().join("memocap.db")).unwrap();
        (directory, connection)
    }

    #[test]
    fn query_finds_saved_memory() {
        let (_directory, connection) = open_temp();
        let id = remember(
            &connection,
            "项目使用 pnpm，验证交给 GitHub Actions",
            "preference",
            "pnpm,ci",
        )
        .unwrap();
        assert!(id > 0);
        let found = recall(&connection, "pnpm Actions", 5).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, "preference");
    }

    #[test]
    fn save_inserts_memory() {
        let (_directory, connection) = open_temp();
        let id = remember(
            &connection,
            "项目使用 pnpm，验证交给 GitHub Actions",
            "preference",
            "pnpm,ci",
        )
        .unwrap();
        assert_eq!(id, 1);
        assert_eq!(count(&connection).unwrap(), 1);
    }

    #[test]
    fn list_returns_recent_memories() {
        let (_directory, connection) = open_temp();
        remember(
            &connection,
            "项目使用 pnpm，验证交给 GitHub Actions",
            "preference",
            "tag-a",
        )
        .unwrap();
        remember(&connection, "second item", "preference", "tag-a").unwrap();
        let listed = list(&connection, 20).unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].content, "second item");
    }

    #[test]
    fn delete_removes_only_target() {
        let (_directory, connection) = open_temp();
        let keep = remember(
            &connection,
            "项目使用 pnpm，验证交给 GitHub Actions",
            "preference",
            "tag-a",
        )
        .unwrap();
        let drop = remember(&connection, "second item", "preference", "tag-a").unwrap();
        assert!(forget(&connection, drop).unwrap());
        assert!(!forget(&connection, drop).unwrap());
        let listed = list(&connection, 20).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, keep);
    }

    #[test]
    fn empty_store_has_no_memories() {
        let (_directory, connection) = open_temp();
        assert_eq!(count(&connection).unwrap(), 0);
        assert!(list(&connection, 20).unwrap().is_empty());
        assert!(recall(&connection, "anything", 5).unwrap().is_empty());
    }

    #[test]
    fn query_without_match_returns_empty() {
        let (_directory, connection) = open_temp();
        remember(
            &connection,
            "项目使用 pnpm，验证交给 GitHub Actions",
            "preference",
            "tag-a",
        )
        .unwrap();
        assert!(recall(&connection, "zzzz-no-hit", 5).unwrap().is_empty());
        assert!(recall(&connection, "   ", 5).unwrap().is_empty());
    }

    #[test]
    fn remember_rejects_empty_content() {
        let (_directory, connection) = open_temp();
        assert!(remember(&connection, "   ", "note", "").is_err());
        assert_eq!(count(&connection).unwrap(), 0);
    }
}
