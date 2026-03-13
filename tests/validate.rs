#[allow(dead_code)]
mod common;

use assert_cmd::prelude::*;
use common::{docker_available, fixture_path, oav_command, write_config, write_config_with_linter};
use predicates::prelude::*;
use serde_json::Value;
use std::error::Error;
use std::fs;
use tempfile::TempDir;

// ── Input validation (no Docker required) ───────────────────────────────

#[test]
fn blank_spec_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["validate", "--spec", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--spec cannot be blank"));
}

#[test]
fn invalid_server_generators_rejected() {
    let temp = TempDir::new().unwrap();
    fs::copy(fixture_path("valid.yml"), temp.path().join("openapi.yaml")).unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["validate", "--server-generators", "bogus"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Unsupported server generator: 'bogus'",
        ));
}

#[test]
fn invalid_client_generators_rejected() {
    let temp = TempDir::new().unwrap();
    fs::copy(fixture_path("valid.yml"), temp.path().join("openapi.yaml")).unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["validate", "--client-generators", "nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Unsupported client generator: 'nope'",
        ));
}

#[test]
fn invalid_linter_flag_rejected() {
    let temp = TempDir::new().unwrap();
    fs::copy(fixture_path("valid.yml"), temp.path().join("openapi.yaml")).unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["validate", "--linter", "eslint"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'eslint'"));
}

#[test]
fn docker_timeout_zero_rejected() {
    let temp = TempDir::new().unwrap();
    fs::copy(fixture_path("valid.yml"), temp.path().join("openapi.yaml")).unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["validate", "--docker-timeout", "0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("must be greater than 0"));
}

#[test]
fn search_depth_zero_rejected() {
    let temp = TempDir::new().unwrap();
    fs::copy(fixture_path("valid.yml"), temp.path().join("openapi.yaml")).unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["validate", "--search-depth", "0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("must be greater than 0"));
}

#[test]
fn infra_error_exits_with_code_2() {
    let temp = TempDir::new().unwrap();
    // No spec and no .oavc → infra error
    let output = oav_command()
        .current_dir(temp.path())
        .args(["validate"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}

// ── Docker integration tests ────────────────────────────────────────────

#[test]
#[ignore]
fn valid_spec_lints_with_redocly() -> Result<(), Box<dyn Error>> {
    if !docker_available() {
        eprintln!("Docker not available, skipping.");
        return Ok(());
    }

    let temp = TempDir::new()?;
    let root = temp.path();
    fs::copy(fixture_path("valid.yml"), root.join("valid.yml"))?;
    write_config_with_linter(root, "valid.yml", "redocly")?;

    let mut cmd = oav_command();
    cmd.current_dir(root)
        .arg("validate")
        .arg("--skip-generate")
        .arg("--skip-compile");
    cmd.assert().success();

    let status = fs::read_to_string(root.join(".oav").join("status.tsv"))?;
    assert!(status.contains("lint\tspec\tredocly\tok"));
    Ok(())
}

#[test]
#[ignore]
fn invalid_spec_lints_fail_with_redocly() -> Result<(), Box<dyn Error>> {
    if !docker_available() {
        eprintln!("Docker not available, skipping.");
        return Ok(());
    }

    let temp = TempDir::new()?;
    let root = temp.path();
    fs::copy(fixture_path("invalid.yml"), root.join("invalid.yml"))?;
    write_config_with_linter(root, "invalid.yml", "redocly")?;

    let mut cmd = oav_command();
    cmd.current_dir(root)
        .arg("validate")
        .arg("--skip-generate")
        .arg("--skip-compile");
    cmd.assert().failure();

    let status = fs::read_to_string(root.join(".oav").join("status.tsv"))?;
    assert!(status.contains("lint\tspec\tredocly\tfail"));
    Ok(())
}

#[test]
#[ignore]
fn inheritance_spec_lints_with_redocly() -> Result<(), Box<dyn Error>> {
    if !docker_available() {
        eprintln!("Docker not available, skipping.");
        return Ok(());
    }

    let temp = TempDir::new()?;
    let root = temp.path();
    fs::copy(
        fixture_path("inheritance.yml"),
        root.join("inheritance.yml"),
    )?;
    write_config_with_linter(root, "inheritance.yml", "redocly")?;

    let mut cmd = oav_command();
    cmd.current_dir(root)
        .arg("validate")
        .arg("--skip-generate")
        .arg("--skip-compile");
    cmd.assert().success();

    let status = fs::read_to_string(root.join(".oav").join("status.tsv"))?;
    assert!(status.contains("lint\tspec\tredocly\tok"));
    Ok(())
}

#[test]
#[ignore]
fn linter_override_redocly_via_flag() -> Result<(), Box<dyn Error>> {
    if !docker_available() {
        eprintln!("Docker not available, skipping.");
        return Ok(());
    }

    let temp = TempDir::new()?;
    let root = temp.path();
    fs::copy(fixture_path("valid.yml"), root.join("valid.yml"))?;
    // Config defaults to spectral, but we override via CLI flag
    write_config(root, "valid.yml")?;

    oav_command()
        .current_dir(root)
        .args([
            "validate",
            "--skip-generate",
            "--skip-compile",
            "--linter",
            "redocly",
        ])
        .assert()
        .success();

    let status = fs::read_to_string(root.join(".oav").join("status.tsv"))?;
    assert!(status.contains("lint\tspec\tredocly\tok"));
    Ok(())
}

#[test]
#[ignore]
fn linter_none_skips_lint() -> Result<(), Box<dyn Error>> {
    if !docker_available() {
        eprintln!("Docker not available, skipping.");
        return Ok(());
    }

    let temp = TempDir::new()?;
    let root = temp.path();
    fs::copy(fixture_path("valid.yml"), root.join("valid.yml"))?;
    write_config(root, "valid.yml")?;

    oav_command()
        .current_dir(root)
        .args([
            "validate",
            "--skip-generate",
            "--skip-compile",
            "--linter",
            "none",
        ])
        .assert()
        .success();

    let status_path = root.join(".oav").join("status.tsv");
    if status_path.exists() {
        let status = fs::read_to_string(&status_path)?;
        assert!(
            !status.contains("lint\t"),
            "lint row should not exist when linter=none"
        );
    }
    Ok(())
}

// Uses the spectral:oas base ruleset (via --ruleset override) to avoid
// Entur-specific rule failures on the generic test fixture.
const SPECTRAL_OAS_RULESET: &str =
    "https://raw.githubusercontent.com/entur/api-guidelines/main/.spectral-required.yml";

#[test]
#[ignore]
fn valid_spec_passes_spectral() -> Result<(), Box<dyn Error>> {
    if !docker_available() {
        eprintln!("Docker not available, skipping.");
        return Ok(());
    }

    let temp = TempDir::new()?;
    let root = temp.path();
    fs::copy(fixture_path("valid.yml"), root.join("valid.yml"))?;
    write_config(root, "valid.yml")?;

    oav_command()
        .current_dir(root)
        .args([
            "validate",
            "--skip-generate",
            "--skip-compile",
            "--ruleset",
            SPECTRAL_OAS_RULESET,
        ])
        .assert()
        .success();

    let status = fs::read_to_string(root.join(".oav").join("status.tsv"))?;
    assert!(status.contains("lint\tspec\tspectral\tok"));
    Ok(())
}

#[test]
#[ignore]
fn invalid_spec_fails_spectral() -> Result<(), Box<dyn Error>> {
    if !docker_available() {
        eprintln!("Docker not available, skipping.");
        return Ok(());
    }

    let temp = TempDir::new()?;
    let root = temp.path();
    fs::copy(fixture_path("invalid.yml"), root.join("invalid.yml"))?;
    write_config(root, "invalid.yml")?;

    oav_command()
        .current_dir(root)
        .args([
            "validate",
            "--skip-generate",
            "--skip-compile",
            "--ruleset",
            SPECTRAL_OAS_RULESET,
        ])
        .assert()
        .failure();

    let status = fs::read_to_string(root.join(".oav").join("status.tsv"))?;
    assert!(status.contains("lint\tspec\tspectral\tfail"));
    Ok(())
}

#[test]
#[ignore]
fn default_linter_is_spectral() -> Result<(), Box<dyn Error>> {
    if !docker_available() {
        eprintln!("Docker not available, skipping.");
        return Ok(());
    }

    let temp = TempDir::new()?;
    let root = temp.path();
    fs::copy(fixture_path("valid.yml"), root.join("valid.yml"))?;
    write_config(root, "valid.yml")?;

    // Don't assert success/failure — the Entur ruleset may flag warnings.
    // We only care that Spectral was invoked (status.tsv says "spectral").
    let _ = oav_command()
        .current_dir(root)
        .args(["validate", "--skip-generate", "--skip-compile"])
        .assert();

    let status = fs::read_to_string(root.join(".oav").join("status.tsv"))?;
    assert!(
        status.contains("lint\tspec\tspectral\t"),
        "expected spectral in status.tsv, got: {status}"
    );
    Ok(())
}

// ── JSON output for validate ────────────────────────────────────────────

#[test]
fn infra_error_json_has_error_and_exit_code() {
    let temp = TempDir::new().unwrap();
    let output = oav_command()
        .current_dir(temp.path())
        .args(["validate", "--output", "json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: Value = serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    assert!(json["error"].is_string(), "should have an error string");
    assert_eq!(json["exit_code"], 2);
}

#[test]
#[ignore]
fn valid_spec_json_report_schema() -> Result<(), Box<dyn Error>> {
    if !docker_available() {
        eprintln!("Docker not available, skipping.");
        return Ok(());
    }

    let temp = TempDir::new()?;
    let root = temp.path();
    fs::copy(fixture_path("valid.yml"), root.join("valid.yml"))?;
    write_config(root, "valid.yml")?;

    let output = oav_command()
        .current_dir(root)
        .args([
            "validate",
            "--skip-generate",
            "--skip-compile",
            "--ruleset",
            SPECTRAL_OAS_RULESET,
            "--output",
            "json",
        ])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let json: Value = serde_json::from_str(&stdout).expect("stdout should be valid JSON");

    // Top-level fields
    assert!(json["spec"].is_string());
    assert!(json["mode"].is_string());

    // Phases
    assert!(json["phases"].is_object());
    let lint = &json["phases"]["lint"];
    assert!(lint.is_object(), "lint phase should be present");
    assert!(lint["linter"].is_string());
    assert!(lint["status"].is_string());
    assert!(lint.get("log").is_some());

    // Summary
    let summary = &json["summary"];
    assert!(summary["total"].is_number());
    assert!(summary["passed"].is_number());
    assert!(summary["failed"].is_number());

    Ok(())
}

#[test]
#[ignore]
fn invalid_spec_json_report_shows_failure() -> Result<(), Box<dyn Error>> {
    if !docker_available() {
        eprintln!("Docker not available, skipping.");
        return Ok(());
    }

    let temp = TempDir::new()?;
    let root = temp.path();
    fs::copy(fixture_path("invalid.yml"), root.join("invalid.yml"))?;
    write_config(root, "invalid.yml")?;

    let output = oav_command()
        .current_dir(root)
        .args([
            "validate",
            "--skip-generate",
            "--skip-compile",
            "--ruleset",
            SPECTRAL_OAS_RULESET,
            "--output",
            "json",
        ])
        .output()?;

    assert_eq!(
        output.status.code(),
        Some(1),
        "failing validation should exit with code 1"
    );

    let stdout = String::from_utf8(output.stdout)?;
    let json: Value = serde_json::from_str(&stdout).expect("stdout should be valid JSON");

    assert_eq!(json["phases"]["lint"]["status"], "fail");
    assert!(json["summary"]["failed"].as_u64().unwrap() > 0);

    Ok(())
}

// ── JSON spec file support (Docker) ─────────────────────────────────────

#[test]
#[ignore]
fn valid_json_spec_passes_spectral() -> Result<(), Box<dyn Error>> {
    if !docker_available() {
        eprintln!("Docker not available, skipping.");
        return Ok(());
    }

    let temp = TempDir::new()?;
    let root = temp.path();
    fs::copy(fixture_path("valid.json"), root.join("valid.json"))?;
    write_config(root, "valid.json")?;

    oav_command()
        .current_dir(root)
        .args([
            "validate",
            "--skip-generate",
            "--skip-compile",
            "--ruleset",
            SPECTRAL_OAS_RULESET,
        ])
        .assert()
        .success();

    let status = fs::read_to_string(root.join(".oav").join("status.tsv"))?;
    assert!(status.contains("lint\tspec\tspectral\tok"));
    Ok(())
}

#[test]
#[ignore]
fn invalid_json_spec_fails_spectral() -> Result<(), Box<dyn Error>> {
    if !docker_available() {
        eprintln!("Docker not available, skipping.");
        return Ok(());
    }

    let temp = TempDir::new()?;
    let root = temp.path();
    fs::copy(fixture_path("invalid.json"), root.join("invalid.json"))?;
    write_config(root, "invalid.json")?;

    oav_command()
        .current_dir(root)
        .args([
            "validate",
            "--skip-generate",
            "--skip-compile",
            "--ruleset",
            SPECTRAL_OAS_RULESET,
        ])
        .assert()
        .failure();

    let status = fs::read_to_string(root.join(".oav").join("status.tsv"))?;
    assert!(status.contains("lint\tspec\tspectral\tfail"));
    Ok(())
}

#[test]
#[ignore]
fn valid_json_spec_lints_with_redocly() -> Result<(), Box<dyn Error>> {
    if !docker_available() {
        eprintln!("Docker not available, skipping.");
        return Ok(());
    }

    let temp = TempDir::new()?;
    let root = temp.path();
    fs::copy(fixture_path("valid.json"), root.join("valid.json"))?;
    write_config_with_linter(root, "valid.json", "redocly")?;

    oav_command()
        .current_dir(root)
        .args(["validate", "--skip-generate", "--skip-compile"])
        .assert()
        .success();

    let status = fs::read_to_string(root.join(".oav").join("status.tsv"))?;
    assert!(status.contains("lint\tspec\tredocly\tok"));
    Ok(())
}

#[test]
#[ignore]
fn invalid_json_spec_fails_redocly() -> Result<(), Box<dyn Error>> {
    if !docker_available() {
        eprintln!("Docker not available, skipping.");
        return Ok(());
    }

    let temp = TempDir::new()?;
    let root = temp.path();
    fs::copy(fixture_path("invalid.json"), root.join("invalid.json"))?;
    write_config_with_linter(root, "invalid.json", "redocly")?;

    oav_command()
        .current_dir(root)
        .args(["validate", "--skip-generate", "--skip-compile"])
        .assert()
        .failure();

    let status = fs::read_to_string(root.join(".oav").join("status.tsv"))?;
    assert!(status.contains("lint\tspec\tredocly\tfail"));
    Ok(())
}
