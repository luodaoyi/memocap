use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::store::{self, Memory};

pub fn remember(
    database: &Path,
    content: &str,
    kind: &str,
    tags: &str,
    force: bool,
    overwrite_id: Option<i64>,
) -> Result<i64> {
    store::remember(
        &store::open(database)?,
        content,
        kind,
        tags,
        "global",
        force,
        overwrite_id,
    )
}

pub fn recall(
    database: &Path,
    query: &str,
    limit: usize,
    kind: Option<&str>,
    max_chars: Option<usize>,
) -> Result<Vec<Memory>> {
    store::recall(&store::open(database)?, query, limit, kind, max_chars)
}

pub fn list(database: &Path, limit: usize) -> Result<Vec<Memory>> {
    store::list(&store::open(database)?, limit)
}

pub fn forget(database: &Path, id: i64) -> Result<bool> {
    store::forget(&store::open(database)?, id)
}

pub fn count(database: &Path) -> Result<i64> {
    store::count(&store::open(database)?)
}

#[must_use]
pub fn format_memories(memories: &[Memory]) -> String {
    if memories.is_empty() {
        return "No local memories found.\n".to_owned();
    }
    let mut out = String::new();
    for memory in memories {
        out.push_str(&format!(
            "#{} [{}] {}\n",
            memory.id, memory.kind, memory.content
        ));
        if !memory.tags.is_empty() {
            out.push_str(&format!("  tags: {}\n", memory.tags));
        }
        out.push_str(&format!("  time: {}\n", memory.created_at));
    }
    out
}

#[must_use]
pub fn format_status(database: &Path, count: i64, files: &[PathBuf], configured: bool) -> String {
    let mut out = format!(
        "database: {}\ncount: {count}\nconfigured: {}\n",
        database.display(),
        if configured { "yes" } else { "no" }
    );
    for path in files {
        out.push_str(&format!("file: {}\n", path.display()));
    }
    out
}

#[must_use]
pub fn format_remote_status(
    address: &str,
    count: i64,
    files: &[PathBuf],
    configured: bool,
) -> String {
    let mut out = format!(
        "remote: {address}\ncount: {count}\nconfigured: {}\n",
        if configured { "yes" } else { "no" }
    );
    for path in files {
        out.push_str(&format!("file: {}\n", path.display()));
    }
    out
}
