use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::{agents, agents_block, paths::Paths, skill_markdown};

#[derive(Debug)]
pub struct InstallStatus {
    pub binary: PathBuf,
    pub database: PathBuf,
    pub agents_path: PathBuf,
    pub claude_path: PathBuf,
    pub skill_path: PathBuf,
    pub configured: bool,
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

pub fn install(global: bool) -> Result<InstallStatus> {
    let paths = Paths::discover()?;
    let source = env::current_exe().context("无法定位当前 memocap 可执行文件")?;
    if source != paths.installed_binary {
        copy_binary(&source, &paths.installed_binary)?;
    }
    let (agents_path, claude_path, skill_path) = host_files(global, &paths)?;
    let binary = display_binary(&paths.installed_binary);
    let block = agents_block(&binary);
    agents::apply(&agents_path, &block)?;
    agents::apply(&claude_path, &block)?;
    agents::apply(&skill_path, &skill_markdown(&binary))?;
    Ok(InstallStatus {
        configured: true,
        binary: paths.installed_binary,
        database: paths.database,
        agents_path,
        claude_path,
        skill_path,
    })
}

pub fn uninstall(global: bool) -> Result<bool> {
    let paths = Paths::discover()?;
    let (agents_path, claude_path, skill_path) = host_files(global, &paths)?;
    let removed_agents = agents::remove(&agents_path)?;
    let removed_claude = agents::remove(&claude_path)?;
    let removed_skill = agents::remove(&skill_path)?;
    Ok(removed_agents || removed_claude || removed_skill)
}

pub fn status(global: bool) -> Result<InstallStatus> {
    let paths = Paths::discover()?;
    let (agents_path, claude_path, skill_path) = host_files(global, &paths)?;
    Ok(InstallStatus {
        configured: agents::contains_managed_block(&agents_path)?
            || agents::contains_managed_block(&claude_path)?
            || agents::contains_managed_block(&skill_path)?,
        binary: paths.installed_binary,
        database: paths.database,
        agents_path,
        claude_path,
        skill_path,
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
