use memocap::hosts;

#[test]
fn four_hosts_exist() {
    let hosts = hosts::official_hosts();
    assert_eq!(hosts.len(), 4);
}

#[test]
fn plugin_uses_cli_not_a_second_store() {
    let plugin = include_str!("../plugin/index.js");
    assert!(plugin.contains("memocap"));
    assert!(!plugin.contains("better-sqlite"));
    assert!(!plugin.to_lowercase().contains("chroma"));
}

#[test]
fn skill_uses_cli_not_a_second_store() {
    let skill = include_str!("../skills/memocap/SKILL.md");
    assert!(skill.contains("memocap remember"));
    assert!(skill.contains("Do not open another store") || skill.contains("do not open another"));
    assert!(skill.contains("言必检"));
    assert!(skill.contains("值必存"));
}
