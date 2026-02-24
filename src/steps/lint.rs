use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::cli::Linter;
use crate::config::Config;
use crate::docker;
use crate::output::Output;
use crate::util::{OAV_DIR, append_status, to_posix_path, write_log_header};

pub fn run(root: &Path, spec_path: &Path, config: &Config, output: &Output) -> Result<bool> {
    let timeout = Duration::from_secs(config.docker_timeout);
    match config.linter {
        Linter::Spectral => run_spectral(root, spec_path, config, output, timeout),
        Linter::Redocly => run_redocly(root, spec_path, config, output, timeout),
        Linter::None => Ok(true),
    }
}

fn run_spectral(
    root: &Path,
    spec_path: &Path,
    config: &Config,
    output: &Output,
    timeout: Duration,
) -> Result<bool> {
    let reports_dir = root.join(OAV_DIR).join("reports").join("lint");
    fs::create_dir_all(&reports_dir).context("Failed to create lint reports directory")?;
    let log_path = reports_dir.join("spectral.log");

    let workspace = root.to_string_lossy().to_string();
    let spec = format!("/work/{}", to_posix_path(spec_path));
    let image = &config.spectral_image;
    let ruleset = &config.spectral_ruleset;
    let fail_severity = &config.spectral_fail_severity;

    let command_line = format!(
        "$ docker run --rm -v {workspace}:/work {image} lint {spec} --ruleset {ruleset} --fail-severity {fail_severity}"
    );
    write_log_header(&log_path, &command_line)?;

    let mut command = Command::new("docker");
    command
        .arg("run")
        .arg("--rm")
        .arg("-v")
        .arg(format!("{workspace}:/work"))
        .arg(image)
        .arg("lint")
        .arg(&spec)
        .arg("--ruleset")
        .arg(ruleset)
        .arg("--fail-severity")
        .arg(fail_severity);

    let success = docker::run_with_logging(&mut command, &log_path, output, timeout)?;
    append_status(
        root,
        "lint",
        "spec",
        "spectral",
        if success { "ok" } else { "fail" },
        &log_path,
    )?;
    Ok(success)
}

fn run_redocly(
    root: &Path,
    spec_path: &Path,
    config: &Config,
    output: &Output,
    timeout: Duration,
) -> Result<bool> {
    let reports_dir = root.join(OAV_DIR).join("reports").join("lint");
    fs::create_dir_all(&reports_dir).context("Failed to create lint reports directory")?;
    let log_path = reports_dir.join("redocly.log");

    let workspace = root.to_string_lossy().to_string();
    let container_root = format!("/work/{OAV_DIR}");
    let spec = format!("/work/{}", to_posix_path(spec_path));
    let redocly_image = &config.redocly_image;

    let command_line = format!(
        "$ docker run --rm -v {workspace}:/work -w {container_root} {redocly_image} lint {spec}"
    );
    write_log_header(&log_path, &command_line)?;

    let mut command = Command::new("docker");
    command
        .arg("run")
        .arg("--rm")
        .arg("-v")
        .arg(format!("{workspace}:/work"))
        .arg("-w")
        .arg(container_root)
        .arg(redocly_image)
        .arg("lint")
        .arg(spec);

    let success = docker::run_with_logging(&mut command, &log_path, output, timeout)?;
    append_status(
        root,
        "lint",
        "spec",
        "redocly",
        if success { "ok" } else { "fail" },
        &log_path,
    )?;
    Ok(success)
}
