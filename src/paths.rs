use std::{env, path::PathBuf};

use anyhow::{anyhow, Result};

#[derive(Clone, Debug)]
pub struct Paths {
    pub home: PathBuf,
    pub codex_home: PathBuf,
    pub claude_home: PathBuf,
    pub data_dir: PathBuf,
    pub database: PathBuf,
    pub installed_binary: PathBuf,
}

impl Paths {
    pub fn discover() -> Result<Self> {
        let home = env::var_os("MEMOCAP_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(dirs::home_dir)
            .ok_or_else(|| anyhow!("无法确定用户目录；请设置 MEMOCAP_HOME"))?;
        let data_dir = env::var_os("MEMOCAP_DATA_DIR")
            .filter(|value| !value.is_empty())
            .map_or_else(|| home.join(".memocap"), PathBuf::from);
        let codex_home = env::var_os("CODEX_HOME")
            .filter(|value| !value.is_empty())
            .map_or_else(|| home.join(".codex"), PathBuf::from);
        let claude_home = env::var_os("CLAUDE_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                env::var_os("CLAUDE_CONFIG_DIR")
                    .filter(|value| !value.is_empty())
                    .map(PathBuf::from)
            })
            .unwrap_or_else(|| home.join(".claude"));
        Ok(Self {
            installed_binary: codex_home.join("bin").join(binary_name()),
            database: data_dir.join("memocap.db"),
            claude_home,
            codex_home,
            data_dir,
            home,
        })
    }
}

#[must_use]
pub fn binary_name() -> &'static str {
    if cfg!(windows) {
        "memocap.exe"
    } else {
        "memocap"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_name_matches_target() {
        assert!(binary_name().starts_with("memocap"));
    }
}
