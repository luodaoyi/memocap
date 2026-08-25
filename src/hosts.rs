use crate::agents_block;

/// Official host install paths. All four share one CLI and one SQLite store.
pub const CODEX_INSTALL: &str = "memocap install";
pub const CLAUDE_INSTALL: &str = "memocap install";
pub const PI_INSTALL: &str = "pi install npm:memocap";
pub const OPENCODE_INSTALL: &str = "opencode plugin memocap";

#[must_use]
pub fn official_hosts() -> [&'static str; 4] {
    [CODEX_INSTALL, CLAUDE_INSTALL, PI_INSTALL, OPENCODE_INSTALL]
}

/// Claude / Pi skill body. Points at the shared CLI, not a second store.
#[must_use]
pub fn skill_markdown(binary: &str) -> String {
    format!(
        "---\nname: memocap\ndescription: Shared SQLite memory via the memocap CLI. Recall first every utterance. Store decisions, preferences, tasks, agreements, and context after a similar-check.\n---\n\nUse `{binary}` only. Do not open another database.\n\n{}\n",
        agents_block(binary)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_official_hosts() {
        let hosts = official_hosts();
        assert_eq!(hosts.len(), 4);
        assert!(hosts.contains(&"memocap install"));
        assert!(hosts.contains(&"pi install npm:memocap"));
        assert!(hosts.contains(&"opencode plugin memocap"));
    }

    #[test]
    fn skill_uses_shared_cli() {
        let skill = skill_markdown("memocap");
        assert!(skill.contains("memocap recall"));
        assert!(skill.contains("Do not open another database"));
        assert!(skill.contains("言必检"));
        assert!(skill.contains("值必存"));
        assert!(!skill.to_lowercase().contains("chroma"));
        assert!(!skill.to_lowercase().contains("embedding"));
    }
}
