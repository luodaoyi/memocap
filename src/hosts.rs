use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};

use crate::{agents_block, paths::Paths};

/// Official host install paths. All four share one CLI and one SQLite store.
pub const CODEX_INSTALL: &str = "memocap install";
pub const CLAUDE_INSTALL: &str = "memocap install";
pub const PI_INSTALL: &str = "pi install npm:memocap";
pub const OPENCODE_INSTALL: &str = "opencode plugin memocap";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Host {
    Codex,
    Claude,
    Grok,
    Pi,
    OpenCode,
}

impl Host {
    pub const ALL: [Host; 5] = [
        Host::Codex,
        Host::Claude,
        Host::Grok,
        Host::Pi,
        Host::OpenCode,
    ];

    pub const FILE_WRITING: [Host; 3] = [Host::Codex, Host::Claude, Host::Grok];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Host::Codex => "codex",
            Host::Claude => "claude",
            Host::Grok => "grok",
            Host::Pi => "pi",
            Host::OpenCode => "opencode",
        }
    }

    pub fn parse_name(name: &str) -> Result<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "codex" => Ok(Host::Codex),
            "claude" => Ok(Host::Claude),
            "grok" => Ok(Host::Grok),
            "pi" => Ok(Host::Pi),
            "opencode" => Ok(Host::OpenCode),
            other => Err(anyhow!(
                "unknown host `{other}` (expected: codex, claude, grok, pi, opencode)"
            )),
        }
    }

    #[must_use]
    pub fn writes_files(&self) -> bool {
        matches!(self, Host::Codex | Host::Claude | Host::Grok)
    }

    #[must_use]
    pub fn hint(self) -> Option<&'static str> {
        match self {
            Host::Pi => Some(PI_INSTALL),
            Host::OpenCode => Some(OPENCODE_INSTALL),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Detection {
    pub host: Host,
    pub detected: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostSelection {
    pub hosts: Vec<Host>,
    pub detected: Vec<Host>,
    pub used_fallback: bool,
}

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

pub fn parse_host_args(args: &[String]) -> Result<Vec<Host>> {
    let mut out = Vec::new();
    for raw in args {
        for part in raw.split(',') {
            if part.trim().is_empty() {
                continue;
            }
            let host = Host::parse_name(part)?;
            if !out.contains(&host) {
                out.push(host);
            }
        }
    }
    if out.is_empty() {
        anyhow::bail!("--host requires at least one of: codex, claude, grok, pi, opencode");
    }
    Ok(out)
}

#[must_use]
pub fn detect(paths: &Paths) -> Vec<Detection> {
    detect_with_path_var(paths, env::var_os("PATH").unwrap_or_default())
}

#[must_use]
pub fn detect_with_path_var(paths: &Paths, path_var: impl AsRef<OsStr>) -> Vec<Detection> {
    Host::ALL
        .into_iter()
        .map(|host| Detection {
            host,
            detected: is_detected(host, paths, path_var.as_ref()),
        })
        .collect()
}

fn is_detected(host: Host, paths: &Paths, path_var: &OsStr) -> bool {
    match host {
        Host::Codex => paths.codex_home.is_dir(),
        Host::Claude => paths.claude_home.is_dir(),
        Host::Grok => paths.grok_home.is_dir(),
        Host::Pi => paths.home.join(".pi").is_dir() || on_path("pi", path_var),
        Host::OpenCode => on_path("opencode", path_var),
    }
}

#[must_use]
pub fn detected_hosts(paths: &Paths) -> Vec<Host> {
    detect(paths)
        .into_iter()
        .filter(|item| item.detected)
        .map(|item| item.host)
        .collect()
}

#[must_use]
pub fn detected_file_writing(paths: &Paths) -> Vec<Host> {
    detected_hosts(paths)
        .into_iter()
        .filter(Host::writes_files)
        .collect()
}

/// Non-interactive default: detected file-writing hosts, else Codex+Claude.
#[must_use]
pub fn default_hosts(paths: &Paths) -> (Vec<Host>, bool) {
    let detected = detected_file_writing(paths);
    if detected.is_empty() {
        (vec![Host::Codex, Host::Claude], true)
    } else {
        (detected, false)
    }
}

pub fn resolve_hosts(all: bool, host_args: &[String], paths: &Paths) -> Result<HostSelection> {
    let detected = detected_hosts(paths);
    let mut hosts = Vec::new();
    if all {
        hosts.extend(Host::FILE_WRITING);
    }
    if !host_args.is_empty() {
        for host in parse_host_args(host_args)? {
            if !hosts.contains(&host) {
                hosts.push(host);
            }
        }
    }
    let used_fallback = if hosts.is_empty() {
        let (defaults, fallback) = default_hosts(paths);
        hosts = defaults;
        fallback
    } else {
        false
    };
    Ok(HostSelection {
        hosts,
        detected,
        used_fallback,
    })
}

#[must_use]
pub fn join_host_names(hosts: &[Host]) -> String {
    hosts
        .iter()
        .map(|host| host.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

fn on_path(cmd: &str, path_var: &OsStr) -> bool {
    let names: Vec<String> = if cfg!(windows) {
        vec![cmd.to_owned(), format!("{cmd}.exe")]
    } else {
        vec![cmd.to_owned()]
    };
    env::split_paths(path_var).any(|dir| names.iter().any(|name| dir.join(name).is_file()))
}

#[must_use]
pub fn grok_skill_path(global: bool, paths: &Paths, cwd: &Path) -> PathBuf {
    if global {
        paths
            .grok_home
            .join("skills")
            .join("memocap")
            .join("SKILL.md")
    } else {
        cwd.join(".grok")
            .join("skills")
            .join("memocap")
            .join("SKILL.md")
    }
}

#[must_use]
pub fn grok_agent_candidates(global: bool, paths: &Paths, cwd: &Path) -> Vec<PathBuf> {
    let root = if global {
        paths.grok_home.clone()
    } else {
        cwd.join(".grok")
    };
    vec![root.join("AGENTS.md"), root.join("GROK.md")]
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
    fn empty_paths(root: &Path) -> Paths {
        Paths {
            home: root.to_path_buf(),
            codex_home: root.join(".codex"),
            claude_home: root.join(".claude"),
            grok_home: root.join(".grok"),
            data_dir: root.join(".memocap"),
            database: root.join(".memocap/memocap.db"),
            installed_binary: root.join(".codex/bin/memocap"),
        }
    }

    #[test]
    fn parse_host_repeatable_and_comma_separated() {
        let hosts = parse_host_args(&["grok".into(), "claude,codex".into()]).unwrap();
        assert_eq!(hosts, vec![Host::Grok, Host::Claude, Host::Codex]);
    }

    #[test]
    fn parse_host_rejects_unknown() {
        assert!(parse_host_args(&["nope".into()]).is_err());
    }

    #[test]
    fn all_selects_file_writing_hosts() {
        let directory = tempfile::tempdir().unwrap();
        let paths = empty_paths(directory.path());
        let selected = resolve_hosts(true, &[], &paths).unwrap();
        assert_eq!(selected.hosts, Vec::from(Host::FILE_WRITING));
        assert!(!selected.used_fallback);
    }

    #[test]
    fn detect_file_writing_homes() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        std::fs::create_dir(root.join(".codex")).unwrap();
        std::fs::create_dir(root.join(".grok")).unwrap();
        let paths = empty_paths(root);
        let found: Vec<_> = detect_with_path_var(&paths, "")
            .into_iter()
            .filter(|item| item.detected)
            .map(|item| item.host)
            .collect();
        assert_eq!(found, vec![Host::Codex, Host::Grok]);
    }

    #[test]
    fn default_falls_back_to_codex_claude() {
        let directory = tempfile::tempdir().unwrap();
        let paths = empty_paths(directory.path());
        let (hosts, fallback) = default_hosts(&paths);
        assert!(fallback);
        assert_eq!(hosts, vec![Host::Codex, Host::Claude]);
    }
}
