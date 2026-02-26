#[allow(dead_code)]
mod common;

use assert_cmd::prelude::*;
use common::oav_command;
use predicates::prelude::*;
use serde_json::Value;
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

// ── JSON output for config subcommands ──────────────────────────────────

fn parse_json_stdout(cmd: &mut std::process::Command) -> Value {
    let output = cmd.output().expect("failed to execute oav");
    assert!(output.status.success(), "command failed: {:?}", output);
    let stdout = String::from_utf8(output.stdout).expect("non-UTF-8 stdout");
    serde_json::from_str(&stdout).expect("stdout is not valid JSON")
}

#[test]
fn config_get_json_returns_key_and_value() {
    let temp = TempDir::new().unwrap();
    let json = parse_json_stdout(
        oav_command()
            .current_dir(temp.path())
            .args(["config", "get", "linter", "--output", "json"]),
    );
    assert_eq!(json["key"], "linter");
    assert_eq!(json["value"], "spectral");
}

#[test]
fn config_get_json_numeric_field() {
    let temp = TempDir::new().unwrap();
    let json = parse_json_stdout(oav_command().current_dir(temp.path()).args([
        "config",
        "get",
        "docker_timeout",
        "--output",
        "json",
    ]));
    assert_eq!(json["key"], "docker_timeout");
    assert_eq!(json["value"], 300);
}

#[test]
fn config_get_json_boolean_field() {
    let temp = TempDir::new().unwrap();
    let json = parse_json_stdout(
        oav_command()
            .current_dir(temp.path())
            .args(["config", "get", "lint", "--output", "json"]),
    );
    assert_eq!(json["key"], "lint");
    assert_eq!(json["value"], true);
}

#[test]
fn config_validate_json_valid() {
    let temp = TempDir::new().unwrap();
    let json = parse_json_stdout(
        oav_command()
            .current_dir(temp.path())
            .args(["config", "validate", "--output", "json"]),
    );
    assert_eq!(json["valid"], true);
}

#[test]
fn config_validate_json_invalid_exits_nonzero() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join(".oavc"), "docker_timeout: 0\n").unwrap();
    // Invalid config should fail even with --output json
    oav_command()
        .current_dir(temp.path())
        .args(["config", "validate", "--output", "json"])
        .assert()
        .failure();
}

#[test]
fn config_list_generators_json_has_server_and_client() {
    let temp = TempDir::new().unwrap();
    let json = parse_json_stdout(oav_command().current_dir(temp.path()).args([
        "config",
        "list-generators",
        "--output",
        "json",
    ]));
    let server = json["server"]
        .as_array()
        .expect("server should be an array");
    let client = json["client"]
        .as_array()
        .expect("client should be an array");
    assert!(!server.is_empty(), "server generators should not be empty");
    assert!(!client.is_empty(), "client generators should not be empty");
    assert!(server.iter().any(|v| v == "spring"));
    assert!(client.iter().any(|v| v == "typescript-axios"));
}

#[test]
fn config_print_json_has_all_fields() {
    let temp = TempDir::new().unwrap();
    let json = parse_json_stdout(
        oav_command()
            .current_dir(temp.path())
            .args(["config", "--output", "json"]),
    );
    let obj = json.as_object().expect("config print should be an object");
    let expected_fields = [
        "spec",
        "mode",
        "lint",
        "generate",
        "compile",
        "server_generators",
        "client_generators",
        "generator_overrides",
        "generator_image",
        "redocly_image",
        "linter",
        "spectral_image",
        "spectral_ruleset",
        "spectral_fail_severity",
        "manage_gitignore",
        "docker_timeout",
        "search_depth",
        "jobs",
    ];
    for field in &expected_fields {
        assert!(obj.contains_key(*field), "missing field: {field}");
    }
}

#[test]
fn config_print_json_reflects_custom_values() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join(".oavc"),
        "linter: redocly\ndocker_timeout: 120\n",
    )
    .unwrap();
    let json = parse_json_stdout(
        oav_command()
            .current_dir(temp.path())
            .args(["config", "--output", "json"]),
    );
    assert_eq!(json["linter"], "redocly");
    assert_eq!(json["docker_timeout"], 120);
}

// ── jobs config field ───────────────────────────────────────────────────

#[test]
fn set_jobs_auto_accepted() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "jobs", "auto"])
        .assert()
        .success();

    oav_command()
        .current_dir(temp.path())
        .args(["config", "get", "jobs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("auto"));
}

#[test]
fn set_jobs_zero_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "jobs", "0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("positive integer"));
}

#[test]
fn set_jobs_valid_accepted() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "jobs", "4"])
        .assert()
        .success();

    oav_command()
        .current_dir(temp.path())
        .args(["config", "get", "jobs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("4"));
}

#[test]
fn set_jobs_fixed_json() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "jobs", "2"])
        .assert()
        .success();

    let json = parse_json_stdout(
        oav_command()
            .current_dir(temp.path())
            .args(["config", "get", "jobs", "--output", "json"]),
    );
    assert_eq!(json["key"], "jobs");
    assert_eq!(json["value"], 2);
}

#[test]
fn set_jobs_auto_json() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "jobs", "auto"])
        .assert()
        .success();

    let json = parse_json_stdout(
        oav_command()
            .current_dir(temp.path())
            .args(["config", "get", "jobs", "--output", "json"]),
    );
    assert_eq!(json["key"], "jobs");
    assert_eq!(json["value"], "auto");
}

#[test]
fn set_jobs_negative_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "jobs", "--", "-1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid jobs"));
}

#[test]
fn load_jobs_auto_from_yaml() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join(".oavc"), "jobs: auto\n").unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "get", "jobs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("auto"));
}

#[test]
fn load_jobs_numeric_from_yaml() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join(".oavc"), "jobs: 3\n").unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "get", "jobs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("3"));
}
