use std::{fs, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Memory {
    pub id: i64,
    pub content: String,
    pub kind: String,
    pub tags: String,
    pub created_at: String,
    pub updated_at: String,
    pub scope: String,
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
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            scope TEXT NOT NULL DEFAULT 'global'
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

pub fn remember(
    connection: &Connection,
    content: &str,
    kind: &str,
    tags: &str,
    scope: &str,
) -> Result<i64> {
    let content = content.trim();
    if content.is_empty() {
        anyhow::bail!("memory content is empty");
    }
    let now: DateTime<Utc> = Utc::now();
    let stamp = now.to_rfc3339();
    let scope = scope.trim();
    let scope = if scope.is_empty() { "global" } else { scope };
    connection.execute(
        "INSERT INTO memories (content, kind, tags, created_at, updated_at, scope) VALUES (?1, ?2, ?3, ?4, ?4, ?5)",
        params![content, { let k = kind.trim(); if k.is_empty() { "context" } else { k } }, tags.trim(), stamp, scope],
    )?;
    Ok(connection.last_insert_rowid())
}

pub fn recall(connection: &Connection, query: &str, limit: usize) -> Result<Vec<Memory>> {
    let prepared = fts_query(query);
    if prepared.is_empty() {
        return Ok(Vec::new());
    }
    let mut statement = connection.prepare(
        "SELECT m.id, m.content, m.kind, m.tags, m.created_at, m.updated_at, m.scope
         FROM memories_fts f
         JOIN memories m ON m.id = f.rowid
         WHERE memories_fts MATCH ?1
         ORDER BY bm25(memories_fts), m.id DESC
         LIMIT ?2",
    )?;
    let rows = statement.query_map(params![prepared, limit as i64], memory_from_row)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn list(connection: &Connection, limit: usize) -> Result<Vec<Memory>> {
    let mut statement = connection.prepare(
        "SELECT id, content, kind, tags, created_at, updated_at, scope FROM memories ORDER BY id DESC LIMIT ?1",
    )?;
    let rows = statement.query_map(params![limit as i64], memory_from_row)?;
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
        updated_at: row.get(5)?,
        scope: row.get(6)?,
    })
}

fn fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .map(|part| format!("\"{}\"", part.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" AND ")
}
