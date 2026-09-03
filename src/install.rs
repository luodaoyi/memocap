use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::{
    agents, agents_block,
    hosts::{self, Host},
    paths::Paths,
    skill_markdown,
};

#[derive(Debug)]
pub struct InstallStatus {
    pub binary: PathBuf,
    pub database: PathBuf,
    pub agents_path: PathBuf,
    pub claude_path: PathBuf,
    pub skill_path: PathBuf,
    pub grok_skill_path: PathBuf,
    pub configured: bool,
    pub selected: Vec<Host>,
    pub written: Vec<PathBuf>,
    pub hints: Vec<String>,
}

#[must_use]
pub fn global_agents_path(paths: &Paths) -> PathBuf {
    paths.codex_home.join("AGENTS.md")
}

pub fn project_agents_path() -> Result<PathBuf> {
    Ok(env::current_dir()?.join("AGENTS.md"))
}

#[must_use]
pub fn global_claude_path(paths: &Paths) -> PathBuf {
    paths.claude_home.join("CLAUDE.md")
}

pub fn project_claude_path() -> Result<PathBuf> {
    Ok(env::current_dir()?.join("CLAUDE.md"))
}

#[must_use]
pub fn global_skill_path(paths: &Paths) -> PathBuf {
    paths
        .claude_home
        .join("skills")
        .join("memocap")
        .join("SKILL.md")
}

pub fn project_skill_path() -> Result<PathBuf> {
    Ok(env::current_dir()?
        .join(".claude")
        .join("skills")
        .join("memocap")
        .join("SKILL.md"))
}

fn host_files(global: bool, paths: &Paths) -> Result<(PathBuf, PathBuf, PathBuf)> {
    if global {
        Ok((
            global_agents_path(paths),
            global_claude_path(paths),
            global_skill_path(paths),
        ))
    } else {
        Ok((
            project_agents_path()?,
            project_claude_path()?,
            project_skill_path()?,
        ))
    }
}

pub fn install(global: bool, hosts: &[Host]) -> Result<InstallStatus> {
    install_with(&Paths::discover()?, global, hosts)
}

pub fn install_with(paths: &Paths, global: bool, hosts: &[Host]) -> Result<InstallStatus> {
    let source = env::current_exe().context("无法定位当前 memocap 可执行文件")?;
    if source != paths.installed_binary {
        copy_binary(&source, &paths.installed_binary)?;
    }
    apply_hosts(paths, global, hosts)
}

pub fn uninstall(global: bool, hosts: &[Host]) -> Result<bool> {
    uninstall_with(&Paths::discover()?, global, hosts)
}

pub fn status(global: bool, hosts: &[Host]) -> Result<InstallStatus> {
    status_with(&Paths::discover()?, global, hosts)
}

fn apply_hosts(paths: &Paths, global: bool, hosts: &[Host]) -> Result<InstallStatus> {
    let cwd = env::current_dir()?;
    let (agents_path, claude_path, skill_path) = host_files(global, paths)?;
    let grok_skill = hosts::grok_skill_path(global, paths, &cwd);
    let binary = display_binary(&paths.installed_binary);
    let block = agents_block(&binary);
    let skill = skill_markdown(&binary);
    let mut written = Vec::new();
    let mut hints = Vec::new();
    for host in hosts {
        match host {
            Host::Codex => {
                agents::apply(&agents_path, &block)?;
                written.push(agents_path.clone());
            }
            Host::Claude => {
                agents::apply(&claude_path, &block)?;
                agents::apply(&skill_path, &skill)?;
                written.push(claude_path.clone());
                written.push(skill_path.clone());
            }
            Host::Grok => {
                agents::apply(&grok_skill, &skill)?;
                written.push(grok_skill.clone());
                for candidate in hosts::grok_agent_candidates(global, paths, &cwd) {
                    if candidate.is_file() {
                        agents::apply(&candidate, &block)?;
                        written.push(candidate);
                    }
                }
            }
            Host::Pi | Host::OpenCode => {
                if let Some(text) = host.hint() {
                    hints.push(text.to_owned());
                }
            }
        }
    }
    Ok(InstallStatus {
        configured: true,
        binary: paths.installed_binary.clone(),
        database: paths.database.clone(),
        agents_path,
        claude_path,
        skill_path,
        grok_skill_path: grok_skill,
        selected: hosts.to_vec(),
        written,
        hints,
    })
}

pub fn uninstall_with(paths: &Paths, global: bool, hosts: &[Host]) -> Result<bool> {
    let cwd = env::current_dir()?;
    let (agents_path, claude_path, skill_path) = host_files(global, paths)?;
    let mut removed = false;
    for host in hosts {
        match host {
            Host::Codex => removed |= agents::remove(&agents_path)?,
            Host::Claude => {
                removed |= agents::remove(&claude_path)?;
                removed |= agents::remove(&skill_path)?;
            }
            Host::Grok => {
                removed |= agents::remove(&hosts::grok_skill_path(global, paths, &cwd))?;
                for candidate in hosts::grok_agent_candidates(global, paths, &cwd) {
                    removed |= agents::remove(&candidate)?;
                }
            }
            Host::Pi | Host::OpenCode => {}
        }
    }
    Ok(removed)
}

pub fn status_with(paths: &Paths, global: bool, hosts: &[Host]) -> Result<InstallStatus> {
    let cwd = env::current_dir()?;
    let (agents_path, claude_path, skill_path) = host_files(global, paths)?;
    let grok_skill = hosts::grok_skill_path(global, paths, &cwd);
    let mut written = Vec::new();
    let mut configured = false;
    for host in hosts {
        match host {
            Host::Codex => {
                written.push(agents_path.clone());
                configured |= agents::contains_managed_block(&agents_path)?;
            }
            Host::Claude => {
                written.push(claude_path.clone());
                written.push(skill_path.clone());
                configured |= agents::contains_managed_block(&claude_path)?;
                configured |= agents::contains_managed_block(&skill_path)?;
            }
            Host::Grok => {
                written.push(grok_skill.clone());
                configured |= agents::contains_managed_block(&grok_skill)?;
                for candidate in hosts::grok_agent_candidates(global, paths, &cwd) {
                    if candidate.is_file() {
                        written.push(candidate.clone());
                        configured |= agents::contains_managed_block(&candidate)?;
                    }
                }
            }
            Host::Pi | Host::OpenCode => {}
        }
    }
    Ok(InstallStatus {
        configured,
        binary: paths.installed_binary.clone(),
        database: paths.database.clone(),
        agents_path,
        claude_path,
        skill_path,
        grok_skill_path: grok_skill,
        selected: hosts.to_vec(),
        written,
        hints: Vec::new(),
    })
}

fn copy_binary(source: &Path, destination: &Path) -> Result<()> {
    let parent = destination.parent().context("安装路径没有父目录")?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(".memocap-install.tmp");
    fs::copy(source, &temporary).with_context(|| format!("复制 {} 失败", source.display()))?;
    // Windows cannot rename over an existing .exe. Removing only our known
    // destination makes repeated install/update work on all supported systems.
    if destination.exists() {
        fs::remove_file(destination)
            .with_context(|| format!("替换旧程序失败：{}", destination.display()))?;
    }
    fs::rename(&temporary, destination)
        .with_context(|| format!("安装到 {} 失败", destination.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(destination)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(destination, permissions)?;
    }
    Ok(())
}

fn display_binary(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_paths() -> Paths {
        Paths {
            home: PathBuf::from("/home/test"),
            codex_home: PathBuf::from("/home/test/.codex"),
            claude_home: PathBuf::from("/home/test/.claude"),
            grok_home: PathBuf::from("/home/test/.grok"),
            data_dir: PathBuf::from("/home/test/.memocap"),
            database: PathBuf::from("/home/test/.memocap/memocap.db"),
            installed_binary: PathBuf::from("/home/test/.codex/bin/memocap"),
        }
    }

    #[test]
    fn global_config_is_under_codex_home() {
        let paths = sample_paths();
        assert_eq!(
            global_agents_path(&paths),
            PathBuf::from("/home/test/.codex/AGENTS.md")
        );
    }

    #[test]
    fn four_hosts_share_one_store() {
        let paths = sample_paths();
        assert_eq!(
            global_claude_path(&paths),
            PathBuf::from("/home/test/.claude/CLAUDE.md")
        );
        assert_eq!(
            global_skill_path(&paths),
            PathBuf::from("/home/test/.claude/skills/memocap/SKILL.md")
        );
        assert_eq!(
            paths.database,
            PathBuf::from("/home/test/.memocap/memocap.db")
        );
        assert_eq!(
            Path::new("/home/test/.claude/CLAUDE.md").parent().unwrap(),
            Path::new("/home/test/.claude")
        );
    }
}
