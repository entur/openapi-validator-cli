use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cli::Mode;
use crate::config::Config;
use crate::docker;
use crate::output::Output;
use crate::util::{append_error, append_status, to_posix_path, write_log_header, OAV_DIR};

pub fn run(root: &Path, spec_path: &Path, config: &Config, output: &Output) -> Result<bool> {
    let reports_root = root.join(OAV_DIR).join("reports").join("generate");
    let server_dir = root.join(OAV_DIR).join("generators").join("server");
    let client_dir = root.join(OAV_DIR).join("generators").join("client");

    let mut failures = 0;

    if matches!(config.mode, Mode::Server | Mode::Both) {
        if !run_for_scope(
            root,
            spec_path,
            &config.generator_image,
            "server",
            &server_dir,
            &config.server_generators,
            &reports_root,
            output,
        )? {
            failures += 1;
        }
    }

    if matches!(config.mode, Mode::Client | Mode::Both) {
        if !run_for_scope(
            root,
            spec_path,
            &config.generator_image,
            "client",
            &client_dir,
            &config.client_generators,
            &reports_root,
            output,
        )? {
            failures += 1;
        }
    }

    Ok(failures == 0)
}

fn run_for_scope(
    root: &Path,
    spec_path: &Path,
    generator_image: &str,
    scope: &str,
    config_dir: &Path,
    requested: &[String],
    reports_root: &Path,
    output: &Output,
) -> Result<bool> {
    let report_dir = reports_root.join(scope);
    fs::create_dir_all(&report_dir).context("Failed to create generate report directory")?;
    let error_log = report_dir.join("_errors.log");

    let configs = match resolve_configs(config_dir, requested) {
        Ok(configs) => configs,
        Err(err) => {
            append_error(&error_log, &err.to_string())?;
            append_status(root, "generate", scope, "_config_", "fail", &error_log)?;
            return Ok(false);
        }
    };

    let mut failures = 0;
    for config_path in configs {
        let name = config_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("unknown");
        let log_path = report_dir.join(format!("{name}.log"));
        let config_rel = config_path
            .strip_prefix(root)
            .context("Generator config path is outside repository")?;
        let container_config = format!("/work/{}", to_posix_path(config_rel));
        let container_spec = format!("/work/{}", to_posix_path(spec_path));

        let command_line = format!(
            "$ docker run --rm {user} -v {root}:/work -w /work/{oav} {image} generate -i {spec} -c {config}",
            user = docker::user_flag(),
            root = root.display(),
            oav = OAV_DIR,
            image = generator_image,
            spec = container_spec,
            config = container_config
        )
        .replace("  ", " ");
        write_log_header(&log_path, &command_line)?;

        output.substep_start(&format!("Generate {scope} {name}"));
        let mut command = Command::new("docker");
        command
            .arg("run")
            .arg("--rm")
            .args(docker::user_args())
            .arg("-v")
            .arg(format!("{}:/work", root.display()))
            .arg("-w")
            .arg(format!("/work/{OAV_DIR}"))
            .arg(generator_image)
            .arg("generate")
            .arg("-i")
            .arg(container_spec)
            .arg("-c")
            .arg(container_config);

        let success = docker::run_with_logging(&mut command, &log_path, output)?;
        append_status(
            root,
            "generate",
            scope,
            name,
            if success { "ok" } else { "fail" },
            &log_path,
        )?;
        output.substep_finish(&format!("Generate {scope} {name}"), success);
        if !success {
            failures += 1;
        }
    }

    Ok(failures == 0)
}

fn resolve_configs(config_dir: &Path, requested: &[String]) -> Result<Vec<PathBuf>> {
    if !config_dir.is_dir() {
        bail!("Missing config directory: {}", config_dir.display());
    }

    let mut configs = Vec::new();
    if !requested.is_empty() {
        for raw in requested {
            let name = raw.trim();
            if name.is_empty() {
                continue;
            }
            let path = config_dir.join(format!("{name}.yaml"));
            if !path.is_file() {
                bail!("Missing generator config: {}", path.display());
            }
            configs.push(path);
        }
    } else {
        for entry in fs::read_dir(config_dir).context("Failed to read generator directory")? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("yaml") {
                configs.push(path);
            }
        }
    }

    configs.sort();
    if configs.is_empty() {
        bail!("No generator configs found under {}", config_dir.display());
    }
    Ok(configs)
}
