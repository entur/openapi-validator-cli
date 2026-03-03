use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::cli::Mode;
use crate::config::{self, Config};
use crate::custom::{self, CustomGeneratorDef};
use crate::docker;
use crate::generators;
use crate::output::Output;
use crate::util::{OAV_DIR, append_status, write_log_header};

use super::TaskResult;

enum CompileTask {
    Builtin(BuiltinTask),
    Custom {
        name: String,
        scope: String,
        block: custom::CompileBlock,
    },
}

struct BuiltinTask {
    scope: String,
    service: String,
    name: String,
}

pub fn run(
    root: &Path,
    config: &Config,
    output: &Output,
    custom_defs: &[CustomGeneratorDef],
) -> Result<bool> {
    let reports_root = root.join(OAV_DIR).join("reports").join("compile");
    fs::create_dir_all(&reports_root).context("Failed to create compile reports directory")?;
    let timeout = Duration::from_secs(config.docker_timeout);

    let mut tasks = Vec::new();

    if matches!(config.mode, Mode::Server | Mode::Both) {
        tasks.extend(resolve_compile_tasks(
            "server",
            &config.server_generators,
            generators::SERVER_GENERATORS,
            custom_defs,
        )?);
    }

    if matches!(config.mode, Mode::Client | Mode::Both) {
        tasks.extend(resolve_compile_tasks(
            "client",
            &config.client_generators,
            generators::CLIENT_GENERATORS,
            custom_defs,
        )?);
    }

    let jobs = config::resolve_jobs(config.jobs);
    if jobs <= 1 {
        return run_sequential(root, &tasks, &reports_root, output, timeout);
    }

    run_parallel(root, &tasks, &reports_root, output, timeout, jobs)
}

fn compile_task_name(task: &CompileTask) -> &str {
    match task {
        CompileTask::Builtin(t) => &t.name,
        CompileTask::Custom { name, .. } => name,
    }
}

fn compile_task_scope(task: &CompileTask) -> &str {
    match task {
        CompileTask::Builtin(t) => &t.scope,
        CompileTask::Custom { scope, .. } => scope,
    }
}

fn run_compile_task(
    root: &Path,
    task: &CompileTask,
    reports_root: &Path,
    output: &Output,
    timeout: Duration,
    quiet: bool,
) -> Result<TaskResult> {
    match task {
        CompileTask::Builtin(t) => {
            run_single_builtin_compile(root, t, reports_root, output, timeout, quiet)
        }
        CompileTask::Custom { name, scope, block } => {
            run_single_custom_compile(root, name, scope, block, reports_root, timeout, quiet)
        }
    }
}

fn run_single_builtin_compile(
    root: &Path,
    task: &BuiltinTask,
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

fn run_single_custom_compile(
    root: &Path,
    name: &str,
    scope: &str,
    block: &custom::CompileBlock,
    reports_root: &Path,
    timeout: Duration,
    quiet: bool,
) -> Result<TaskResult> {
    let report_dir = reports_root.join(scope);
    fs::create_dir_all(&report_dir)?;
    let log_path = report_dir.join(format!("{name}.log"));
    let workdir = format!("/work/.oav/generated/{scope}/{name}");

    let command_line = format!(
        "$ docker run --rm -v {root}:/work -w {workdir} {image} sh -c \"{cmd}\"",
        root = root.display(),
        image = block.image,
        cmd = block.command,
    );
    write_log_header(&log_path, &command_line)?;

    let mut command = Command::new("docker");
    command
        .arg("run")
        .arg("--rm")
        .arg("-v")
        .arg(format!("{}:/work", root.display()))
        .arg("-w")
        .arg(&workdir)
        .arg(&block.image)
        .arg("sh")
        .arg("-c")
        .arg(&block.command);

    let _ = quiet; // verbose streaming handled by builtin path; custom always logs to file
    let success = docker::run_with_logging_quiet(&mut command, &log_path, timeout)?;

    Ok(TaskResult {
        name: name.to_string(),
        scope: scope.to_string(),
        success,
        log_path,
    })
}

fn run_sequential(
    root: &Path,
    tasks: &[CompileTask],
    reports_root: &Path,
    output: &Output,
    timeout: Duration,
) -> Result<bool> {
    let mut failures = 0;
    for task in tasks {
        let name = compile_task_name(task);
        let scope = compile_task_scope(task);
        let label = format!("Compile {scope} {name}");
        output.substep_start(&label);

        let result = run_compile_task(root, task, reports_root, output, timeout, false)?;

        append_status(
            root,
            "compile",
            scope,
            name,
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
    tasks: &[CompileTask],
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
                    let name = compile_task_name(task);
                    let scope = compile_task_scope(task);
                    let label = format!("Compile {scope} {name}");
                    let spinner = mp_ref.map(|m| output.add_parallel_spinner(m, &label));
                    s.spawn(move || {
                        let result =
                            run_compile_task(root, task, reports_root, output, timeout, true);
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

fn resolve_compile_tasks(
    scope: &str,
    requested: &[String],
    builtin_defs: &[generators::GeneratorDef],
    custom_defs: &[CustomGeneratorDef],
) -> Result<Vec<CompileTask>> {
    let scope_custom: Vec<&CustomGeneratorDef> =
        custom_defs.iter().filter(|d| d.scope == scope).collect();

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
        let mut all: Vec<String> = builtin_defs.iter().map(|d| d.name.to_string()).collect();
        for d in &scope_custom {
            all.push(d.name.clone());
        }
        all
    };

    let mut tasks = Vec::new();
    for name in names {
        if let Some(def) = builtin_defs.iter().find(|d| d.name == name) {
            tasks.push(CompileTask::Builtin(BuiltinTask {
                scope: scope.to_string(),
                service: format!("{}{}", def.compile_prefix, name),
                name,
            }));
        } else if let Some(cdef) = scope_custom.iter().find(|d| d.name == name) {
            if let Some(block) = &cdef.compile {
                tasks.push(CompileTask::Custom {
                    name,
                    scope: scope.to_string(),
                    block: block.clone(),
                });
            }
            // No compile block → silently skip
        } else {
            bail!("Unknown {scope} generator for compile: '{name}'");
        }
    }
    Ok(tasks)
}
