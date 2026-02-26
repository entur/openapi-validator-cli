#[allow(dead_code)]
mod common;

use assert_cmd::prelude::*;
use common::oav_command;
use predicates::prelude::*;

// ── Generate ────────────────────────────────────────────────────────────

#[test]
fn generate_bash_outputs_completion_script() {
    oav_command()
        .args(["completions", "generate", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn generate_zsh_outputs_completion_script() {
    oav_command()
        .args(["completions", "generate", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("compdef"));
}

#[test]
fn generate_fish_outputs_completion_script() {
    oav_command()
        .args(["completions", "generate", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn generate_invalid_shell_rejected() {
    oav_command()
        .args(["completions", "generate", "nushell"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

// ── Install ─────────────────────────────────────────────────────────────

#[test]
fn install_unsupported_shell_prints_warning() {
    oav_command()
        .args(["completions", "install", "--shell", "elvish"])
        .assert()
        .success()
        .stderr(predicate::str::contains("not supported"));
}

#[test]
fn install_quiet_suppresses_informational_output() {
    let temp = tempfile::TempDir::new().unwrap();
    oav_command()
        .env("HOME", temp.path())
        .args([
            "--quiet",
            "completions",
            "install",
            "--shell",
            "bash",
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    // File should still be installed even in quiet mode
    let completion_file = temp
        .path()
        .join(".local/share/bash-completion/completions/oav");
    assert!(completion_file.exists());
}

// ── Uninstall ───────────────────────────────────────────────────────────

#[test]
fn uninstall_unsupported_shell_prints_info() {
    oav_command()
        .args(["completions", "uninstall", "--shell", "elvish", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing to remove"));
}

#[test]
fn uninstall_nonexistent_file_is_noop() {
    let temp = tempfile::TempDir::new().unwrap();
    oav_command()
        .env("HOME", temp.path())
        .args(["completions", "uninstall", "--shell", "bash", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing to do"));
}

#[test]
fn uninstall_respects_quiet_flag() {
    oav_command()
        .args([
            "--quiet",
            "completions",
            "uninstall",
            "--shell",
            "elvish",
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

// ── Install + Uninstall round-trip ──────────────────────────────────────

#[test]
fn install_then_uninstall_bash() {
    let temp = tempfile::TempDir::new().unwrap();

    oav_command()
        .env("HOME", temp.path())
        .args(["completions", "install", "--shell", "bash", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed"));

    let completion_file = temp
        .path()
        .join(".local/share/bash-completion/completions/oav");
    assert!(completion_file.exists());

    oav_command()
        .env("HOME", temp.path())
        .args(["completions", "uninstall", "--shell", "bash", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed"));

    assert!(!completion_file.exists());
}

#[test]
fn install_then_uninstall_fish() {
    let temp = tempfile::TempDir::new().unwrap();

    oav_command()
        .env("HOME", temp.path())
        .args(["completions", "install", "--shell", "fish", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed"));

    let completion_file = temp.path().join(".config/fish/completions/oav.fish");
    assert!(completion_file.exists());

    oav_command()
        .env("HOME", temp.path())
        .args(["completions", "uninstall", "--shell", "fish", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed"));

    assert!(!completion_file.exists());
}

#[test]
fn install_zsh_patches_zshrc() {
    let temp = tempfile::TempDir::new().unwrap();

    oav_command()
        .env("HOME", temp.path())
        .args(["completions", "install", "--shell", "zsh", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed"));

    let zshrc = std::fs::read_to_string(temp.path().join(".zshrc")).unwrap();
    assert!(zshrc.contains("oav shell completions"));
    assert!(zshrc.contains("compinit"));
}

#[test]
fn install_zsh_skips_zshrc_patch_when_fpath_exists() {
    let temp = tempfile::TempDir::new().unwrap();
    std::fs::write(
        temp.path().join(".zshrc"),
        "fpath=(~/.zsh/completions $fpath)\n",
    )
    .unwrap();

    oav_command()
        .env("HOME", temp.path())
        .args(["completions", "install", "--shell", "zsh", "--yes"])
        .assert()
        .success();

    let zshrc = std::fs::read_to_string(temp.path().join(".zshrc")).unwrap();
    assert!(!zshrc.contains("oav shell completions"));
}
