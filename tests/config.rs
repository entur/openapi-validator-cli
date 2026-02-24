#[allow(dead_code)]
mod common;

use assert_cmd::prelude::*;
use common::oav_command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

// ── Existing field validation ───────────────────────────────────────────

#[test]
fn set_spec_blank_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "spec", "   "])
        .assert()
        .failure()
        .stderr(predicate::str::contains("spec cannot be blank"));
}

#[test]
fn set_generator_image_blank_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "generator_image", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("generator_image cannot be blank"));
}

#[test]
fn set_redocly_image_blank_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "redocly_image", "  "])
        .assert()
        .failure()
        .stderr(predicate::str::contains("redocly_image cannot be blank"));
}

#[test]
fn set_server_generators_invalid_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "server_generators", "[spring, bogus]"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Unsupported server generator: 'bogus'",
        ));
}

#[test]
fn set_client_generators_invalid_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "client_generators", "[nope]"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Unsupported client generator: 'nope'",
        ));
}

#[test]
fn set_server_generators_valid_accepted() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args([
            "config",
            "set",
            "server_generators",
            "[spring, kotlin-spring]",
        ])
        .assert()
        .success();
}

#[test]
fn set_client_generators_valid_accepted() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "client_generators", "[go, python]"])
        .assert()
        .success();
}

#[test]
fn set_generator_overrides_blank_key_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "generator_overrides. ", "something"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "generator_overrides key cannot be blank",
        ));
}

// ── Linter config validation ────────────────────────────────────────────

#[test]
fn set_linter_spectral_accepted() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "linter", "spectral"])
        .assert()
        .success();
}

#[test]
fn set_linter_redocly_accepted() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "linter", "redocly"])
        .assert()
        .success();
}

#[test]
fn set_linter_none_accepted() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "linter", "none"])
        .assert()
        .success();
}

#[test]
fn set_linter_invalid_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "linter", "eslint"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid linter: eslint"));
}

#[test]
fn set_spectral_fail_severity_valid_values() {
    let temp = TempDir::new().unwrap();
    for severity in &["error", "warn", "info", "hint"] {
        oav_command()
            .current_dir(temp.path())
            .args(["config", "set", "spectral_fail_severity", severity])
            .assert()
            .success();
    }
}

#[test]
fn set_spectral_fail_severity_invalid_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "spectral_fail_severity", "fatal"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid fail severity: fatal"));
}

#[test]
fn set_spectral_image_blank_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "spectral_image", "  "])
        .assert()
        .failure()
        .stderr(predicate::str::contains("spectral_image cannot be blank"));
}

#[test]
fn set_spectral_ruleset_blank_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "spectral_ruleset", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("spectral_ruleset cannot be blank"));
}

#[test]
fn get_linter_default_is_spectral() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "get", "linter"])
        .assert()
        .success()
        .stdout(predicate::str::contains("spectral"));
}

#[test]
fn set_then_get_linter_round_trip() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "linter", "redocly"])
        .assert()
        .success();

    oav_command()
        .current_dir(temp.path())
        .args(["config", "get", "linter"])
        .assert()
        .success()
        .stdout(predicate::str::contains("redocly"));
}

#[test]
fn set_then_get_spectral_fail_severity_round_trip() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "spectral_fail_severity", "warn"])
        .assert()
        .success();

    oav_command()
        .current_dir(temp.path())
        .args(["config", "get", "spectral_fail_severity"])
        .assert()
        .success()
        .stdout(predicate::str::contains("warn"));
}

// ── Kebab-case key aliases ──────────────────────────────────────────────

#[test]
fn set_spectral_image_kebab_case_alias() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "spectral-image", "custom/spectral:7"])
        .assert()
        .success();

    oav_command()
        .current_dir(temp.path())
        .args(["config", "get", "spectral-image"])
        .assert()
        .success()
        .stdout(predicate::str::contains("custom/spectral:7"));
}

// ── docker_timeout validation ───────────────────────────────────────────

#[test]
fn set_docker_timeout_zero_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "docker_timeout", "0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("must be greater than 0"));
}

#[test]
fn set_docker_timeout_negative_rejected() {
    let temp = TempDir::new().unwrap();
    // clap rejects "-1" as an unexpected argument; use "--" to pass it as a value
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "docker_timeout", "--", "-1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid docker_timeout"));
}

#[test]
fn set_docker_timeout_valid_accepted() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "docker_timeout", "120"])
        .assert()
        .success();
}

#[test]
fn set_docker_timeout_round_trip() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "docker_timeout", "120"])
        .assert()
        .success();

    oav_command()
        .current_dir(temp.path())
        .args(["config", "get", "docker_timeout"])
        .assert()
        .success()
        .stdout(predicate::str::contains("120"));
}

// ── search_depth validation ─────────────────────────────────────────────

#[test]
fn set_search_depth_zero_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "search_depth", "0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("must be greater than 0"));
}

#[test]
fn set_search_depth_negative_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "search_depth", "--", "-1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid search_depth"));
}

#[test]
fn set_search_depth_valid_accepted() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "search_depth", "6"])
        .assert()
        .success();
}

#[test]
fn set_search_depth_round_trip() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "search_depth", "6"])
        .assert()
        .success();

    oav_command()
        .current_dir(temp.path())
        .args(["config", "get", "search_depth"])
        .assert()
        .success()
        .stdout(predicate::str::contains("6"));
}

// ── Kebab-case aliases for new fields ───────────────────────────────────

#[test]
fn set_docker_timeout_kebab_alias() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "docker-timeout", "60"])
        .assert()
        .success();

    oav_command()
        .current_dir(temp.path())
        .args(["config", "get", "docker-timeout"])
        .assert()
        .success()
        .stdout(predicate::str::contains("60"));
}

#[test]
fn set_search_depth_kebab_alias() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "search-depth", "3"])
        .assert()
        .success();

    oav_command()
        .current_dir(temp.path())
        .args(["config", "get", "search-depth"])
        .assert()
        .success()
        .stdout(predicate::str::contains("3"));
}

// ── config validate subcommand ──────────────────────────────────────────

#[test]
fn config_validate_default_config_ok() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "validate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
}

#[test]
fn config_validate_detects_bad_timeout() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join(".oavc"), "docker_timeout: 0\n").unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "validate"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "docker_timeout must be greater than 0",
        ));
}

#[test]
fn config_validate_detects_bad_depth() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join(".oavc"), "search_depth: 0\n").unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "validate"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "search_depth must be greater than 0",
        ));
}

// ── config list-generators subcommand ───────────────────────────────────

#[test]
fn config_list_generators_shows_all() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "list-generators"])
        .assert()
        .success()
        .stdout(predicate::str::contains("spring"))
        .stdout(predicate::str::contains("typescript-axios"));
}
