use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::cli::Mode;
use crate::config::Config;
use crate::docker;
use crate::output::Output;
use crate::util::{OAV_DIR, append_status, write_log_header};

const SUPPORTED_SERVER_GENERATORS: [&str; 6] = [
    "aspnetcore",
    "go-server",
    "kotlin-spring",
    "python-fastapi",
    "spring",
    "typescript-nestjs",
];

const SUPPORTED_CLIENT_GENERATORS: [&str; 8] = [
    "csharp",
    "go",
    "java",
    "kotlin",
    "python",
    "typescript-axios",
    "typescript-fetch",
    "typescript-node",
];

struct Task {
    scope: String,
    service: String,
    name: String,
}

pub fn run(root: &Path, config: &Config, output: &Output) -> Result<bool> {
    let reports_root = root.join(OAV_DIR).join("reports").join("compile");
    fs::create_dir_all(&reports_root).context("Failed to create compile reports directory")?;

    let mut tasks = Vec::new();

    if matches!(config.mode, Mode::Server | Mode::Both) {
        tasks.extend(resolve_tasks(
            "server",
            &config.server_generators,
            &SUPPORTED_SERVER_GENERATORS,
            "build-",
        )?);
    }

    if matches!(config.mode, Mode::Client | Mode::Both) {
        tasks.extend(resolve_tasks(
            "client",
            &config.client_generators,
            &SUPPORTED_CLIENT_GENERATORS,
            "build-client-",
        )?);
    }

    let mut failures = 0;
    for task in tasks {
        let report_dir = reports_root.join(&task.scope);
        fs::create_dir_all(&report_dir)?;
        let log_path = report_dir.join(format!("{}.log", task.service));
        let project_dir = root.join(OAV_DIR);
        let compose_path = project_dir.join("docker-compose.yaml");
        let command_line = format!(
            "$ docker compose -f {compose} --project-directory {project} run --rm {service}",
            compose = compose_path.display(),
            project = project_dir.display(),
            service = task.service
        );
        write_log_header(&log_path, &command_line)?;

        output.substep_start(&format!("Compile {} {}", task.scope, task.name));
        let mut command = Command::new("docker");
        command
            .arg("compose")
            .arg("-f")
            .arg(&compose_path)
            .arg("--project-directory")
            .arg(&project_dir)
            .arg("run")
            .arg("--rm")
            .arg(&task.service);

        let success = docker::run_with_logging(&mut command, &log_path, output)?;
        append_status(
            root,
            "compile",
            &task.scope,
            &task.name,
            if success { "ok" } else { "fail" },
            &log_path,
        )?;
        output.substep_finish(&format!("Compile {} {}", task.scope, task.name), success);
        if !success {
            failures += 1;
        }
    }

    Ok(failures == 0)
}

fn resolve_tasks(
    scope: &str,
    requested: &[String],
    supported: &[&str],
    prefix: &str,
) -> Result<Vec<Task>> {
    let names: Vec<String> = if !requested.is_empty() {
        requested
            .iter()
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
            .collect()
    } else {
        supported.iter().map(|name| (*name).to_string()).collect()
    };

    let mut tasks = Vec::new();
    for name in names {
        if !supported.contains(&name.as_str()) {
            bail!("Unsupported {scope} generator for compile: {name}");
        }
        tasks.push(Task {
            scope: scope.to_string(),
            service: format!("{prefix}{name}"),
            name,
        });
    }
    Ok(tasks)
}
