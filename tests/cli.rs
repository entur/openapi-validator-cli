use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn docker_available() -> bool {
    Command::new("docker")
        .arg("version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn oav_command() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("oav"))
}

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn write_config(root: &Path, spec: &str) -> Result<(), Box<dyn Error>> {
    let content =
        format!("spec: {spec}\nmode: server\nlint: true\ngenerate: false\ncompile: false\n");
    fs::write(root.join(".oavc"), content)?;
    Ok(())
}

#[test]
#[ignore]
fn valid_spec_lints() -> Result<(), Box<dyn Error>> {
    if !docker_available() {
        eprintln!("Docker not available, skipping.");
        return Ok(());
    }

    let temp = TempDir::new()?;
    let root = temp.path();
    fs::copy(fixture_path("valid.yml"), root.join("valid.yml"))?;
    write_config(root, "valid.yml")?;

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
fn invalid_spec_lints_fail() -> Result<(), Box<dyn Error>> {
    if !docker_available() {
        eprintln!("Docker not available, skipping.");
        return Ok(());
    }

    let temp = TempDir::new()?;
    let root = temp.path();
    fs::copy(fixture_path("invalid.yml"), root.join("invalid.yml"))?;
    write_config(root, "invalid.yml")?;

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
fn inheritance_spec_lints() -> Result<(), Box<dyn Error>> {
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
    write_config(root, "inheritance.yml")?;

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

// ── Input validation tests (no Docker required) ─────────────────────────

#[test]
fn config_set_spec_blank_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "spec", "   "])
        .assert()
        .failure()
        .stderr(predicate::str::contains("spec cannot be blank"));
}

#[test]
fn config_set_generator_image_blank_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "generator_image", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("generator_image cannot be blank"));
}

#[test]
fn config_set_redocly_image_blank_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "redocly_image", "  "])
        .assert()
        .failure()
        .stderr(predicate::str::contains("redocly_image cannot be blank"));
}

#[test]
fn config_set_server_generators_invalid_rejected() {
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
fn config_set_client_generators_invalid_rejected() {
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
fn config_set_server_generators_valid_accepted() {
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
fn config_set_client_generators_valid_accepted() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["config", "set", "client_generators", "[go, python]"])
        .assert()
        .success();
}

#[test]
fn config_set_generator_overrides_blank_key_rejected() {
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

#[test]
fn init_blank_spec_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["init", "--spec", "  "])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--spec cannot be blank"));
}

#[test]
fn validate_blank_spec_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["validate", "--spec", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--spec cannot be blank"));
}

#[test]
fn validate_invalid_server_generators_rejected() {
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
fn validate_invalid_client_generators_rejected() {
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
fn init_invalid_server_generators_rejected() {
    let temp = TempDir::new().unwrap();
    fs::copy(fixture_path("valid.yml"), temp.path().join("openapi.yaml")).unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["init", "--server-generators", "fake"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Unsupported server generator: 'fake'",
        ));
}

#[test]
fn init_invalid_client_generators_rejected() {
    let temp = TempDir::new().unwrap();
    fs::copy(fixture_path("valid.yml"), temp.path().join("openapi.yaml")).unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["init", "--client-generators", "fake"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Unsupported client generator: 'fake'",
        ));
}
