#[allow(dead_code)]
mod common;

use assert_cmd::prelude::*;
use common::oav_command;
use predicates::prelude::*;
use std::fs;
use std::process::Stdio;
use tempfile::TempDir;

// --- oav clean (no flags) ---

#[test]
fn clean_removes_oav_dir() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".oav")).unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["clean"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed"));

    assert!(!temp.path().join(".oav").exists());
}

#[test]
fn clean_no_oav_dir() {
    let temp = TempDir::new().unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["clean"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No .oav directory found"));
}

#[test]
fn clean_preserves_config_and_gitignore() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".oav")).unwrap();
    fs::write(temp.path().join(".oavc"), "spec: foo.yaml\n").unwrap();
    fs::write(temp.path().join(".gitignore"), ".oav/\n.oavc\n").unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["clean"])
        .assert()
        .success();

    assert!(!temp.path().join(".oav").exists());
    assert!(temp.path().join(".oavc").exists());
    let gi = fs::read_to_string(temp.path().join(".gitignore")).unwrap();
    assert!(gi.contains(".oav/"));
    assert!(gi.contains(".oavc"));
}

// --- oav clean --nuke --yes ---

#[test]
fn nuke_removes_everything() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".oav")).unwrap();
    fs::write(temp.path().join(".oavc"), "spec: foo.yaml\n").unwrap();
    fs::write(temp.path().join(".gitignore"), ".oav/\n.oavc\n").unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["clean", "--nuke", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed"));

    assert!(!temp.path().join(".oav").exists());
    assert!(!temp.path().join(".oavc").exists());
    // .gitignore only had oav entries, so the file itself should be removed
    assert!(!temp.path().join(".gitignore").exists());
}

#[test]
fn nuke_nothing_to_clean() {
    let temp = TempDir::new().unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["clean", "--nuke", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing to clean"));
}

#[test]
fn nuke_only_oav_dir() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".oav")).unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["clean", "--nuke", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed").and(predicate::str::contains(".oav/")));

    assert!(!temp.path().join(".oav").exists());
}

#[test]
fn nuke_only_config() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join(".oavc"), "spec: foo.yaml\n").unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["clean", "--nuke", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed").and(predicate::str::contains(".oavc")));

    assert!(!temp.path().join(".oavc").exists());
}

#[test]
fn nuke_only_gitignore_entries() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join(".gitignore"), "target/\n.oav/\n.oavc\n").unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["clean", "--nuke", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed"));

    let gi = fs::read_to_string(temp.path().join(".gitignore")).unwrap();
    assert!(!gi.contains(".oav/"));
    assert!(!gi.contains(".oavc"));
    assert!(gi.contains("target/"));
}

#[test]
fn nuke_preserves_other_gitignore_entries() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".oav")).unwrap();
    fs::write(temp.path().join(".oavc"), "spec: foo.yaml\n").unwrap();
    fs::write(
        temp.path().join(".gitignore"),
        "target/\nnode_modules/\n.oav/\n.oavc\n*.log\n",
    )
    .unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["clean", "--nuke", "--yes"])
        .assert()
        .success();

    let gi = fs::read_to_string(temp.path().join(".gitignore")).unwrap();
    assert!(gi.contains("target/"));
    assert!(gi.contains("node_modules/"));
    assert!(gi.contains("*.log"));
    assert!(!gi.contains(".oav/"));
    assert!(!gi.contains(".oavc"));
}

#[test]
fn nuke_removes_gitignore_file_when_empty_after_cleanup() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join(".gitignore"), ".oav/\n.oavc\n").unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["clean", "--nuke", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains(".gitignore"));

    assert!(!temp.path().join(".gitignore").exists());
}

// --- oav clean --nuke (no --yes) without TTY should fail ---

#[test]
fn nuke_without_yes_fails_without_tty() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(".oav")).unwrap();

    // Without a TTY, should give a clear error message
    oav_command()
        .current_dir(temp.path())
        .stdin(Stdio::null())
        .args(["clean", "--nuke"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("rerun with --yes"));

    // .oav/ should still exist since confirm couldn't proceed
    assert!(temp.path().join(".oav").exists());
}

#[test]
fn yes_without_nuke_rejected() {
    let temp = TempDir::new().unwrap();

    oav_command()
        .current_dir(temp.path())
        .args(["clean", "--yes"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--nuke"));
}
