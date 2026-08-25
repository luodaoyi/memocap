use std::path::Path;

use anyhow::Result;

use crate::store::{self, Memory};

pub fn remember(database: &Path, content: &str, kind: &str, tags: &str) -> Result<i64> {
    store::remember(&store::open(database)?, content, kind, tags, "global")
}

pub fn recall(database: &Path, query: &str, limit: usize) -> Result<Vec<Memory>> {
    store::recall(&store::open(database)?, query, limit)
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
pub fn format_status(database: &Path, count: i64, agents_path: &Path, configured: bool) -> String {
    format!(
        "database: {}\ncount: {count}\nAGENTS.md: {}\nconfigured: {}\n",
        database.display(),
        agents_path.display(),
        if configured { "yes" } else { "no" }
    )
}

#[must_use]
pub fn format_remote_status(
    address: &str,
    count: i64,
    agents_path: &Path,
    configured: bool,
) -> String {
    format!(
        "remote: {address}\ncount: {count}\nAGENTS.md: {}\nconfigured: {}\n",
        agents_path.display(),
        if configured { "yes" } else { "no" }
    )
}
