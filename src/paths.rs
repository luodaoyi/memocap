use std::{env, path::PathBuf};

use anyhow::{anyhow, Result};

#[derive(Clone, Debug)]
pub struct Paths {
    pub home: PathBuf,
    pub codex_home: PathBuf,
    pub claude_home: PathBuf,
    pub grok_home: PathBuf,
    pub data_dir: PathBuf,
    pub database: PathBuf,
    pub installed_binary: PathBuf,
}

impl Paths {
    pub fn discover() -> Result<Self> {
        Self::from_vars(|key| env::var_os(key))
    }

    pub fn from_vars<F>(mut var: F) -> Result<Self>
    where
        F: FnMut(&str) -> Option<std::ffi::OsString>,
    {
        let nonempty = |key: &str, var: &mut F| {
            var(key)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        };
        let home = nonempty("MEMOCAP_HOME", &mut var)
            .or_else(dirs::home_dir)
            .ok_or_else(|| anyhow!("无法确定用户目录；请设置 MEMOCAP_HOME"))?;
        let data_dir =
            nonempty("MEMOCAP_DATA_DIR", &mut var).unwrap_or_else(|| home.join(".memocap"));
        let codex_home = nonempty("CODEX_HOME", &mut var).unwrap_or_else(|| home.join(".codex"));
        let claude_home = nonempty("CLAUDE_HOME", &mut var)
            .or_else(|| nonempty("CLAUDE_CONFIG_DIR", &mut var))
            .unwrap_or_else(|| home.join(".claude"));
        let grok_home = nonempty("GROK_HOME", &mut var).unwrap_or_else(|| home.join(".grok"));
        Ok(Self {
            installed_binary: codex_home.join("bin").join(binary_name()),
            database: data_dir.join("memocap.db"),
            grok_home,
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
