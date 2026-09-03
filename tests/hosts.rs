use memocap::hosts;

#[test]
fn four_hosts_exist() {
    let hosts = hosts::official_hosts();
    assert_eq!(hosts.len(), 4);
}

#[test]
fn plugin_uses_official_default_export() {
    let plugin = include_str!("../plugin/index.js");
    assert!(plugin.contains("server: memocap"));
    assert!(plugin.contains("id: \"memocap\""));
    assert!(!plugin.contains("export default async function"));
    assert!(!plugin.contains("export { run, RULES }"));
    assert!(!plugin.contains("better-sqlite"));
    assert!(!plugin.to_lowercase().contains("chroma"));
}

#[test]
fn plugin_sidecar_uses_cli_not_a_second_store() {
    let sidecar = include_str!("../plugin/cli.js");
    assert!(sidecar.contains("spawnSync"));
    assert!(sidecar.contains("memocap"));
    assert!(!sidecar.contains("better-sqlite"));
}

#[test]
fn skill_uses_cli_not_a_second_store() {
    let skill = include_str!("../skills/memocap/SKILL.md");
    assert!(skill.contains("memocap remember"));
    assert!(skill.contains("Do not open another store") || skill.contains("do not open another"));
    assert!(skill.contains("言必检"));
    assert!(skill.contains("值必存"));
}

#[test]
fn pi_skills_point_at_skill_md_dir() {
    let pkg = include_str!("../package.json");
    assert!(pkg.contains("./skills/memocap"));
}

#[test]
fn package_json_has_bin_launcher() {
    let pkg = include_str!("../package.json");
    assert!(pkg.contains("\"memocap\": \"bin/cli.cjs\""));
}

#[test]
fn bin_launcher_downloads_release_assets() {
    let launcher = include_str!("../bin/cli.cjs");
    assert!(launcher.contains("memocap-x86_64-unknown-linux-gnu"));
    assert!(launcher.contains("memocap-aarch64-apple-darwin"));
    assert!(launcher.contains("memocap-x86_64-pc-windows-msvc.exe"));
    assert!(launcher.contains("spawnSync"));
    assert!(!launcher.contains("better-sqlite"));
    assert!(!launcher.to_lowercase().contains("chroma"));
}

#[test]
fn dockerfile_rust_compiles_edition2024() {
    let docker = include_str!("../Dockerfile");
    assert!(docker.contains("FROM rust:1.88-bookworm AS build"));
    assert!(!docker.contains("rust:1.85"));
}

use memocap::install;
use memocap::paths::Paths;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

fn env_map(pairs: &[(&str, &Path)]) -> HashMap<String, OsString> {
    pairs
        .iter()
        .map(|(key, path)| ((*key).to_owned(), path.as_os_str().to_os_string()))
        .collect()
}

fn paths_from_env(map: &HashMap<String, OsString>) -> Paths {
    Paths::from_vars(|key| map.get(key).cloned()).unwrap()
}

fn detected(paths: &Paths) -> Vec<hosts::Host> {
    hosts::detect_with_path_var(paths, "")
        .into_iter()
        .filter(|item| item.detected)
        .map(|item| item.host)
        .collect()
}

fn temp_paths(root: &Path) -> Paths {
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
fn detect_codex_via_env_home() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join(".codex")).unwrap();
    let map = env_map(&[("MEMOCAP_HOME", directory.path())]);
    let paths = paths_from_env(&map);
    assert_eq!(detected(&paths), vec![hosts::Host::Codex]);
}

#[test]
fn detect_claude_via_env_home() {
    let directory = tempfile::tempdir().unwrap();
    let claude = directory.path().join("claude-home");
    fs::create_dir(&claude).unwrap();
    let map = env_map(&[("MEMOCAP_HOME", directory.path()), ("CLAUDE_HOME", &claude)]);
    let paths = paths_from_env(&map);
    assert_eq!(paths.claude_home, claude);
    assert_eq!(detected(&paths), vec![hosts::Host::Claude]);
}

#[test]
fn detect_grok_via_env_home() {
    let directory = tempfile::tempdir().unwrap();
    let grok = directory.path().join("grok-home");
    fs::create_dir(&grok).unwrap();
    let map = env_map(&[("MEMOCAP_HOME", directory.path()), ("GROK_HOME", &grok)]);
    let paths = paths_from_env(&map);
    assert_eq!(paths.grok_home, grok);
    assert_eq!(detected(&paths), vec![hosts::Host::Grok]);
}

#[test]
fn detect_claude_via_config_dir_env() {
    let directory = tempfile::tempdir().unwrap();
    let claude = directory.path().join("claude-config");
    fs::create_dir(&claude).unwrap();
    let map = env_map(&[
        ("MEMOCAP_HOME", directory.path()),
        ("CLAUDE_CONFIG_DIR", &claude),
    ]);
    let paths = paths_from_env(&map);
    assert_eq!(paths.claude_home, claude);
}

#[test]
fn detect_pi_via_dot_pi_home() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join(".pi")).unwrap();
    let map = env_map(&[("MEMOCAP_HOME", directory.path())]);
    let paths = paths_from_env(&map);
    assert_eq!(detected(&paths), vec![hosts::Host::Pi]);
}

#[test]
fn host_grok_writes_grok_skill_not_claude() {
    let directory = tempfile::tempdir().unwrap();
    let paths = temp_paths(directory.path());
    fs::create_dir_all(&paths.grok_home).unwrap();
    fs::create_dir_all(&paths.claude_home).unwrap();
    install::install_with(&paths, true, &[hosts::Host::Grok]).unwrap();
    let grok_skill = paths
        .grok_home
        .join("skills")
        .join("memocap")
        .join("SKILL.md");
    assert!(grok_skill.is_file());
    let text = fs::read_to_string(&grok_skill).unwrap();
    assert!(text.contains("name: memocap"));
    assert!(!paths.claude_home.join("CLAUDE.md").exists());
    assert!(
        !paths
            .claude_home
            .join("skills")
            .join("memocap")
            .join("SKILL.md")
            .exists()
    );
    assert!(!paths.codex_home.join("AGENTS.md").exists());
}

#[test]
fn host_claude_does_not_write_grok() {
    let directory = tempfile::tempdir().unwrap();
    let paths = temp_paths(directory.path());
    fs::create_dir_all(&paths.grok_home).unwrap();
    fs::create_dir_all(&paths.claude_home).unwrap();
    install::install_with(&paths, true, &[hosts::Host::Claude]).unwrap();
    assert!(paths.claude_home.join("CLAUDE.md").is_file());
    assert!(
        paths
            .claude_home
            .join("skills")
            .join("memocap")
            .join("SKILL.md")
            .is_file()
    );
    assert!(
        !paths
            .grok_home
            .join("skills")
            .join("memocap")
            .join("SKILL.md")
            .exists()
    );
    assert!(!paths.codex_home.join("AGENTS.md").exists());
}

#[test]
fn grok_upserts_existing_agents_md() {
    let directory = tempfile::tempdir().unwrap();
    let paths = temp_paths(directory.path());
    fs::create_dir_all(&paths.grok_home).unwrap();
    let agents = paths.grok_home.join("AGENTS.md");
    fs::write(&agents, "# keep\n").unwrap();
    install::install_with(&paths, true, &[hosts::Host::Grok]).unwrap();
    let text = fs::read_to_string(&agents).unwrap();
    assert!(text.contains("# keep"));
    assert!(text.contains("<!-- memocap:begin -->"));
}

#[test]
fn grok_does_not_create_agents_md() {
    let directory = tempfile::tempdir().unwrap();
    let paths = temp_paths(directory.path());
    install::install_with(&paths, true, &[hosts::Host::Grok]).unwrap();
    assert!(!paths.grok_home.join("AGENTS.md").exists());
    assert!(
        paths
            .grok_home
            .join("skills")
            .join("memocap")
            .join("SKILL.md")
            .is_file()
    );
}

#[test]
fn host_pi_prints_hint_without_files() {
    let directory = tempfile::tempdir().unwrap();
    let paths = temp_paths(directory.path());
    let result = install::install_with(&paths, true, &[hosts::Host::Pi]).unwrap();
    assert_eq!(result.hints, vec![hosts::PI_INSTALL.to_owned()]);
    assert!(result.written.is_empty());
    assert!(!paths.claude_home.join("CLAUDE.md").exists());
    assert!(
        !paths
            .grok_home
            .join("skills")
            .join("memocap")
            .join("SKILL.md")
            .exists()
    );
}

#[test]
fn uninstall_host_grok_leaves_claude() {
    let directory = tempfile::tempdir().unwrap();
    let paths = temp_paths(directory.path());
    install::install_with(&paths, true, &[hosts::Host::Claude, hosts::Host::Grok]).unwrap();
    assert!(install::uninstall_with(&paths, true, &[hosts::Host::Grok]).unwrap());
    assert!(
        !paths
            .grok_home
            .join("skills")
            .join("memocap")
            .join("SKILL.md")
            .exists()
            || {
                let text = fs::read_to_string(
                    paths
                        .grok_home
                        .join("skills")
                        .join("memocap")
                        .join("SKILL.md"),
                )
                .unwrap_or_default();
                !text.contains("<!-- memocap:begin -->")
            }
    );
    assert!(paths.claude_home.join("CLAUDE.md").is_file());
}
