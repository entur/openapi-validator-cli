use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::cli::Mode;
use crate::config::{self, Config};
use crate::docker;
use crate::generators;
use crate::output::Output;
use crate::util::{OAV_DIR, append_status, write_log_header};

use super::TaskResult;

struct Task {
    scope: String,
    service: String,
    name: String,
}

pub fn run(root: &Path, config: &Config, output: &Output) -> Result<bool> {
    let reports_root = root.join(OAV_DIR).join("reports").join("compile");
    fs::create_dir_all(&reports_root).context("Failed to create compile reports directory")?;
    let timeout = Duration::from_secs(config.docker_timeout);

    let mut tasks = Vec::new();

    if matches!(config.mode, Mode::Server | Mode::Both) {
        tasks.extend(resolve_tasks(
            "server",
            &config.server_generators,
            generators::SERVER_GENERATORS,
        )?);
    }

    if matches!(config.mode, Mode::Client | Mode::Both) {
        tasks.extend(resolve_tasks(
            "client",
            &config.client_generators,
            generators::CLIENT_GENERATORS,
        )?);
    }

    let jobs = config::resolve_jobs(config.jobs);
    if jobs <= 1 {
        return run_sequential(root, &tasks, &reports_root, output, timeout);
    }

    run_parallel(root, &tasks, &reports_root, output, timeout, jobs)
}

fn run_single_compile(
    root: &Path,
    task: &Task,
    reports_root: &Path,
    output: &Output,
    timeout: Duration,
    quiet: bool,
) -> Result<TaskResult> {
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

    let success = if quiet {
        docker::run_with_logging_quiet(&mut command, &log_path, timeout)?
    } else {
        docker::run_with_logging(&mut command, &log_path, output, timeout)?
    };

    Ok(TaskResult {
        name: task.name.clone(),
        scope: task.scope.clone(),
        success,
        log_path,
    })
}

fn run_sequential(
    root: &Path,
    tasks: &[Task],
    reports_root: &Path,
    output: &Output,
    timeout: Duration,
) -> Result<bool> {
    let mut failures = 0;
    for task in tasks {
        let label = format!("Compile {} {}", task.scope, task.name);
        output.substep_start(&label);

        let result = run_single_compile(root, task, reports_root, output, timeout, false)?;

        append_status(
            root,
            "compile",
            &task.scope,
            &task.name,
            if result.success { "ok" } else { "fail" },
            &result.log_path,
        )?;
        output.substep_finish(&label, result.success);
        if !result.success {
            failures += 1;
        }
    }

    Ok(failures == 0)
}

fn run_parallel(
    root: &Path,
    tasks: &[Task],
    reports_root: &Path,
    output: &Output,
    timeout: Duration,
    jobs: usize,
) -> Result<bool> {
    let mp = output.multi_progress();
    let mp_ref = mp.as_ref();
    let mut all_failures = 0;

    for chunk in tasks.chunks(jobs) {
        let results: Vec<Result<TaskResult>> = std::thread::scope(|s| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|task| {
                    let label = format!("Compile {} {}", task.scope, task.name);
                    let spinner = mp_ref.map(|m| output.add_parallel_spinner(m, &label));
                    s.spawn(move || {
                        let result =
                            run_single_compile(root, task, reports_root, output, timeout, true);
                        if let Some(mp) = mp_ref {
                            let success = result.as_ref().map(|r| r.success).unwrap_or(false);
                            output.finish_parallel_spinner(mp, spinner.flatten(), &label, success);
                        }
                        result
                    })
                })
                .collect();

            handles
                .into_iter()
                .map(|h| h.join().expect("compile thread panicked"))
                .collect()
        });

        for result in results {
            let result = result?;
            append_status(
                root,
                "compile",
                &result.scope,
                &result.name,
                if result.success { "ok" } else { "fail" },
                &result.log_path,
            )?;
            if !result.success {
                all_failures += 1;
            }
        }
    }

    Ok(all_failures == 0)
}

fn resolve_tasks(
    scope: &str,
    requested: &[String],
    defs: &[generators::GeneratorDef],
) -> Result<Vec<Task>> {
    let names: Vec<String> = if !requested.is_empty() {
        let filtered: Vec<String> = requested
            .iter()
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
            .collect();
        if filtered.is_empty() {
            bail!("No valid {scope} generators specified");
        }
        filtered
    } else {
        defs.iter().map(|d| d.name.to_string()).collect()
    };

    let mut tasks = Vec::new();
    for name in names {
        let def = defs
            .iter()
            .find(|d| d.name == name)
            .ok_or_else(|| anyhow::anyhow!("Unsupported {scope} generator for compile: {name}"))?;
        tasks.push(Task {
            scope: scope.to_string(),
            service: format!("{}{}", def.compile_prefix, name),
            name,
        });
    }
    Ok(tasks)
}
