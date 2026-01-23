use assert_cmd::prelude::*;
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
