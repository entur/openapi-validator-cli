#[allow(dead_code)]
mod common;

use assert_cmd::prelude::*;
use common::oav_command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn skill_dir(root: &std::path::Path) -> std::path::PathBuf {
    root.join(".claude").join("skills").join("oav")
}

// ── Install ────────────────────────────────────────────────────────────

#[test]
fn install_creates_skill_files() {
    let temp = TempDir::new().unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["agent", "install"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed"));

    let dir = skill_dir(temp.path());
    assert!(dir.join("SKILL.md").exists());
    assert!(dir.join("reference.md").exists());
}

#[test]
fn install_skill_md_has_valid_frontmatter() {
    let temp = TempDir::new().unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["agent", "install"])
        .assert()
        .success();

    let content = fs::read_to_string(skill_dir(temp.path()).join("SKILL.md")).unwrap();
    assert!(content.starts_with("---\n"));
    assert!(content.contains("name: oav"));
    assert!(content.contains("description:"));
}

#[test]
fn install_default_subcommand() {
    let temp = TempDir::new().unwrap();

    // `oav agent` with no subcommand should default to install
    oav_command()
        .current_dir(temp.path())
        .args(["agent"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed"));

    assert!(skill_dir(temp.path()).join("SKILL.md").exists());
}

#[test]
fn install_already_installed_warns() {
    let temp = TempDir::new().unwrap();

    // First install
    oav_command()
        .current_dir(temp.path())
        .args(["agent", "install"])
        .assert()
        .success();

    // Second install without --force
    oav_command()
        .current_dir(temp.path())
        .args(["agent", "install"])
        .assert()
        .success()
        .stderr(predicate::str::contains("already installed"));
}

#[test]
fn install_force_overwrites() {
    let temp = TempDir::new().unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["agent", "install"])
        .assert()
        .success();

    // Tamper with an installed file
    let skill_md = skill_dir(temp.path()).join("SKILL.md");
    fs::write(&skill_md, "tampered").unwrap();

    // Force reinstall
    oav_command()
        .current_dir(temp.path())
        .args(["agent", "install", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed"));

    let content = fs::read_to_string(&skill_md).unwrap();
    assert!(content.contains("name: oav"), "file should be restored");
}

// ── Uninstall ──────────────────────────────────────────────────────────

#[test]
fn uninstall_removes_skill_files() {
    let temp = TempDir::new().unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["agent", "install"])
        .assert()
        .success();

    oav_command()
        .current_dir(temp.path())
        .args(["agent", "uninstall"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed"));

    assert!(!skill_dir(temp.path()).exists());
}

#[test]
fn uninstall_cleans_up_empty_parent_dirs() {
    let temp = TempDir::new().unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["agent", "install"])
        .assert()
        .success();

    oav_command()
        .current_dir(temp.path())
        .args(["agent", "uninstall"])
        .assert()
        .success();

    // .claude/skills/ and .claude/ should be removed if empty
    assert!(!temp.path().join(".claude").join("skills").exists());
    assert!(!temp.path().join(".claude").exists());
}

#[test]
fn uninstall_preserves_sibling_dirs() {
    let temp = TempDir::new().unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["agent", "install"])
        .assert()
        .success();

    // Create a sibling skill dir
    let sibling = temp.path().join(".claude").join("skills").join("other");
    fs::create_dir_all(&sibling).unwrap();
    fs::write(sibling.join("SKILL.md"), "other skill").unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["agent", "uninstall"])
        .assert()
        .success();

    // oav skill gone, sibling and parents preserved
    assert!(!skill_dir(temp.path()).exists());
    assert!(sibling.exists());
    assert!(temp.path().join(".claude").join("skills").exists());
}

#[test]
fn uninstall_when_not_installed_is_noop() {
    let temp = TempDir::new().unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["agent", "uninstall"])
        .assert()
        .success()
        .stdout(predicate::str::contains("not installed"));
}

// ── Round-trip ─────────────────────────────────────────────────────────

#[test]
fn install_uninstall_roundtrip() {
    let temp = TempDir::new().unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["agent", "install"])
        .assert()
        .success();
    assert!(skill_dir(temp.path()).join("SKILL.md").exists());

    oav_command()
        .current_dir(temp.path())
        .args(["agent", "uninstall"])
        .assert()
        .success();
    assert!(!skill_dir(temp.path()).exists());

    // Reinstall should work cleanly
    oav_command()
        .current_dir(temp.path())
        .args(["agent", "install"])
        .assert()
        .success();
    assert!(skill_dir(temp.path()).join("SKILL.md").exists());
}

// ── Output modes ───────────────────────────────────────────────────────

#[test]
fn install_quiet_suppresses_output() {
    let temp = TempDir::new().unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["--quiet", "agent", "install"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    assert!(skill_dir(temp.path()).join("SKILL.md").exists());
}

#[test]
fn uninstall_quiet_suppresses_output() {
    let temp = TempDir::new().unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["agent", "install"])
        .assert()
        .success();

    oav_command()
        .current_dir(temp.path())
        .args(["--quiet", "agent", "uninstall"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    assert!(!skill_dir(temp.path()).exists());
}
