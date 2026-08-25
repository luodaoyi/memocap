use std::{fs, path::Path};

use anyhow::{bail, Context, Result};

use crate::{AGENTS_BEGIN, AGENTS_END};

pub fn contains_managed_block(path: &Path) -> Result<bool> {
    Ok(read_optional(path)?.is_some_and(|content| content.contains(AGENTS_BEGIN)))
}

pub fn apply(path: &Path, block: &str) -> Result<()> {
    let current = read_optional(path)?.unwrap_or_default();
    let cleaned = remove_block(&current)?;
    let separator = if cleaned.trim().is_empty() {
        ""
    } else {
        "\n\n"
    };
    atomic_write(
        path,
        format!("{}{}{}", cleaned.trim_end(), separator, block),
    )
}

pub fn remove(path: &Path) -> Result<bool> {
    let Some(current) = read_optional(path)? else {
        return Ok(false);
    };
    if !current.contains(AGENTS_BEGIN) {
        return Ok(false);
    }
    atomic_write(path, remove_block(&current)?.trim_end().to_owned())?;
    Ok(true)
}

fn read_optional(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("读取 {} 失败", path.display())),
    }
}

fn remove_block(content: &str) -> Result<String> {
    let Some(start) = content.find(AGENTS_BEGIN) else {
        return Ok(content.to_owned());
    };
    let end_from_start = content[start..]
        .find(AGENTS_END)
        .ok_or_else(|| anyhow::anyhow!("发现不完整的 memocap 标记块，请手动修复 AGENTS.md"))?;
    let end = start + end_from_start + AGENTS_END.len();
    if content[end..].contains(AGENTS_BEGIN) {
        bail!("发现多个 memocap 标记块，请手动修复 AGENTS.md");
    }
    Ok(format!("{}{}", &content[..start], &content[end..]))
}

fn atomic_write(path: &Path, content: String) -> Result<()> {
    let parent = path.parent().context("AGENTS.md 没有父目录")?;
    fs::create_dir_all(parent)?;
    // `rename` cannot replace an existing destination on Windows. A direct write
    // keeps repeat install/uninstall working on every supported platform.
    fs::write(path, content).with_context(|| format!("写入 {} 失败", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_is_idempotent_and_remove_preserves_other_content() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("AGENTS.md");
        fs::write(&path, "# Existing\n\nKeep this.").unwrap();
        apply(
            &path,
            "<!-- memocap:begin -->\nmemocap\n<!-- memocap:end -->",
        )
        .unwrap();
        apply(
            &path,
            "<!-- memocap:begin -->\nmemocap\n<!-- memocap:end -->",
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(&path)
                .unwrap()
                .matches(AGENTS_BEGIN)
                .count(),
            1
        );
        assert!(remove(&path).unwrap());
        assert_eq!(
            fs::read_to_string(&path).unwrap().trim(),
            "# Existing\n\nKeep this."
        );
    }

    #[test]
    fn apply_replaces_controlled_block() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("AGENTS.md");
        apply(&path, "<!-- memocap:begin -->\nold\n<!-- memocap:end -->").unwrap();
        apply(&path, "<!-- memocap:begin -->\nnew\n<!-- memocap:end -->").unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("new"));
        assert!(!text.contains("old"));
        assert_eq!(text.matches(AGENTS_BEGIN).count(), 1);
    }

    #[test]
    fn incomplete_marker_is_error() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("AGENTS.md");
        fs::write(&path, "<!-- memocap:begin -->\nbroken").unwrap();
        assert!(apply(&path, "<!-- memocap:begin -->\nx\n<!-- memocap:end -->").is_err());
    }
}
