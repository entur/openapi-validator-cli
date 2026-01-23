use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::docker;
use crate::output::Output;
use crate::util::{OAV_DIR, append_status, to_posix_path, write_log_header};

pub fn run(root: &Path, spec_path: &Path, redocly_image: &str, output: &Output) -> Result<bool> {
    let reports_dir = root.join(OAV_DIR).join("reports").join("lint");
    fs::create_dir_all(&reports_dir).context("Failed to create lint reports directory")?;
    let log_path = reports_dir.join("redocly.log");

    let workspace = root.to_string_lossy().to_string();
    let container_root = format!("/work/{OAV_DIR}");
    let spec = format!("/work/{}", to_posix_path(spec_path));
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

    let success = docker::run_with_logging(&mut command, &log_path, output)?;
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
