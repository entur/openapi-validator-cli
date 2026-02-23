use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn docker_available() -> bool {
    Command::new("docker")
        .arg("version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn oav_command() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("oav"))
}

pub fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

pub fn write_config(root: &Path, spec: &str) -> Result<(), Box<dyn std::error::Error>> {
    let content =
        format!("spec: {spec}\nmode: server\nlint: true\ngenerate: false\ncompile: false\n");
    fs::write(root.join(".oavc"), content)?;
    Ok(())
}

pub fn write_config_with_linter(
    root: &Path,
    spec: &str,
    linter: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = format!(
        "spec: {spec}\nmode: server\nlint: true\ngenerate: false\ncompile: false\nlinter: {linter}\n"
    );
    fs::write(root.join(".oavc"), content)?;
    Ok(())
}
