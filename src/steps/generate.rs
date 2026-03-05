use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::cli::Mode;
use crate::config::{self, Config};
use crate::custom::CustomGeneratorDef;
use crate::docker;
use crate::output::Output;
use crate::util::{OAV_DIR, append_error, append_status, to_posix_path, write_log_header};

use super::TaskResult;

enum GenerateTask {
    Builtin { name: String, config_path: PathBuf },
    Custom { def: CustomGeneratorDef },
}

struct ScopeContext<'a> {
    root: &'a Path,
    spec_path: &'a Path,
    generator_image: &'a str,
    scope: &'a str,
    config_dir: &'a Path,
    requested: &'a [String],
    overrides: &'a HashMap<String, String>,
    custom_defs: &'a [CustomGeneratorDef],
    reports_root: &'a Path,
    output: &'a Output,
    timeout: Duration,
    jobs: usize,
}

pub fn run(
    root: &Path,
    spec_path: &Path,
    config: &Config,
    output: &Output,
    custom: &[CustomGeneratorDef],
) -> Result<bool> {
    let reports_root = root.join(OAV_DIR).join("reports").join("generate");
    let server_dir = root.join(OAV_DIR).join("generators").join("server");
    let client_dir = root.join(OAV_DIR).join("generators").join("client");
    let timeout = Duration::from_secs(config.docker_timeout);

    let mut failures = 0;

    if matches!(config.mode, Mode::Server | Mode::Both)
        && !run_for_scope(&ScopeContext {
            root,
            spec_path,
            generator_image: &config.generator_image,
            scope: "server",
            config_dir: &server_dir,
            requested: &config.server_generators,
            overrides: &config.generator_overrides,
            custom_defs: custom,
            reports_root: &reports_root,
            output,
            timeout,
            jobs: config::resolve_jobs(config.jobs),
        })?
    {
        failures += 1;
    }

    if matches!(config.mode, Mode::Client | Mode::Both)
        && !run_for_scope(&ScopeContext {
            root,
            spec_path,
            generator_image: &config.generator_image,
            scope: "client",
            config_dir: &client_dir,
            requested: &config.client_generators,
            overrides: &config.generator_overrides,
            custom_defs: custom,
            reports_root: &reports_root,
            output,
            timeout,
            jobs: config::resolve_jobs(config.jobs),
        })?
    {
        failures += 1;
    }

    Ok(failures == 0)
}

fn run_single_generator(
    ctx: &ScopeContext,
    name: &str,
    config_path: &Path,
    quiet: bool,
) -> Result<TaskResult> {
    let report_dir = ctx.reports_root.join(ctx.scope);
    let log_path = report_dir.join(format!("{name}.log"));
    let config_rel = config_path
        .strip_prefix(ctx.root)
        .context("Generator config path is outside repository")?;
    let container_config = format!("/work/{}", to_posix_path(config_rel));
    let container_spec = format!("/work/{}", to_posix_path(ctx.spec_path));

    let command_line = format!(
        "$ docker run --rm {user} -v {root}:/work -w /work/{oav} {image} generate -i {spec} -c {config}",
        user = docker::user_flag(),
        root = ctx.root.display(),
        oav = OAV_DIR,
        image = ctx.generator_image,
        spec = container_spec,
        config = container_config
    )
    .replace("  ", " ");
    write_log_header(&log_path, &command_line)?;

    let mut command = Command::new("docker");
    command
        .arg("run")
        .arg("--rm")
        .args(docker::user_args())
        .arg("-v")
        .arg(format!("{}:/work", ctx.root.display()))
        .arg("-w")
        .arg(format!("/work/{OAV_DIR}"))
        .arg(ctx.generator_image)
        .arg("generate")
        .arg("-i")
        .arg(container_spec)
        .arg("-c")
        .arg(container_config);

    let success = if quiet {
        docker::run_with_logging_quiet(&mut command, &log_path, ctx.timeout)?
    } else {
        docker::run_with_logging(&mut command, &log_path, ctx.output, ctx.timeout)?
    };

    Ok(TaskResult {
        name: name.to_string(),
        scope: ctx.scope.to_string(),
        success,
        log_path,
    })
}

fn run_for_scope(ctx: &ScopeContext) -> Result<bool> {
    let report_dir = ctx.reports_root.join(ctx.scope);
    fs::create_dir_all(&report_dir).context("Failed to create generate report directory")?;
    let error_log = report_dir.join("_errors.log");

    let tasks = match resolve_generate_tasks(ctx) {
        Ok(tasks) => tasks,
        Err(err) => {
            append_error(&error_log, &err.to_string())?;
            append_status(
                ctx.root, "generate", ctx.scope, "_config_", "fail", &error_log,
            )?;
            return Ok(false);
        }
    };

    if ctx.jobs <= 1 {
        return run_sequential(ctx, &tasks);
    }

    run_parallel(ctx, &tasks)
}

fn resolve_generate_tasks(ctx: &ScopeContext) -> Result<Vec<GenerateTask>> {
    let builtin_names: Vec<&str> = match ctx.scope {
        "server" => crate::generators::server_names(),
        "client" => crate::generators::client_names(),
        _ => Vec::new(),
    };
    let scope_custom: Vec<&CustomGeneratorDef> = ctx
        .custom_defs
        .iter()
        .filter(|d| d.scope == ctx.scope)
        .collect();

    if ctx.requested.is_empty() {
        // All builtins + all custom for this scope
        let builtin_configs = resolve_configs(ctx.root, ctx.config_dir, &[], ctx.overrides)?;
        let mut tasks: Vec<GenerateTask> = builtin_configs
            .into_iter()
            .map(|(name, config_path)| GenerateTask::Builtin { name, config_path })
            .collect();
        for def in &scope_custom {
            tasks.push(GenerateTask::Custom {
                def: (*def).clone(),
            });
        }
        return Ok(tasks);
    }

    // Collect which builtins are requested so we can resolve their configs in one pass.
    let builtin_requested: Vec<String> = ctx
        .requested
        .iter()
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty() && builtin_names.contains(&n.as_str()))
        .collect();

    let builtin_configs: HashMap<String, PathBuf> = if builtin_requested.is_empty() {
        HashMap::new()
    } else {
        resolve_configs(ctx.root, ctx.config_dir, &builtin_requested, ctx.overrides)?
            .into_iter()
            .collect()
    };

    // Build task list in request order.
    let mut tasks = Vec::new();
    for name in ctx.requested {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        if let Some(config_path) = builtin_configs.get(name) {
            tasks.push(GenerateTask::Builtin {
                name: name.to_string(),
                config_path: config_path.clone(),
            });
        } else if let Some(def) = find_custom_by_name(&scope_custom, name) {
            tasks.push(GenerateTask::Custom { def: def.clone() });
        } else {
            anyhow::bail!("Unknown {} generator: '{}'", ctx.scope, name);
        }
    }

    if tasks.is_empty() {
        anyhow::bail!("No generators resolved for scope '{}'", ctx.scope);
    }

    Ok(tasks)
}

/// Wrapper for `custom::find_by_name` that works on a slice of references.
fn find_custom_by_name<'a>(
    defs: &'a [&'a CustomGeneratorDef],
    name: &str,
) -> Option<&'a CustomGeneratorDef> {
    defs.iter().find(|d| d.name == name).copied()
}

fn run_custom_generator(
    ctx: &ScopeContext,
    def: &CustomGeneratorDef,
    quiet: bool,
) -> Result<TaskResult> {
    let report_dir = ctx.reports_root.join(ctx.scope);
    let log_path = report_dir.join(format!("{}.log", def.name));
    let container_spec = format!("/work/{}", to_posix_path(ctx.spec_path));
    let resolved_command = def.generate.command.replace("{spec}", &container_spec);

    let cmd_args = shell_words::split(&resolved_command)
        .with_context(|| format!("Failed to parse generate command for '{}'", def.name))?;
    let command_line = format!(
        "$ docker run --rm {user} -v {root}:/work {image} {cmd}",
        user = docker::user_flag(),
        root = ctx.root.display(),
        image = def.generate.image,
        cmd = resolved_command,
    )
    .replace("  ", " ");
    write_log_header(&log_path, &command_line)?;

    let mut command = Command::new("docker");
    command
        .arg("run")
        .arg("--rm")
        .args(docker::user_args())
        .arg("-v")
        .arg(format!("{}:/work", ctx.root.display()))
        .arg(&def.generate.image)
        .args(&cmd_args);

    let success = if quiet {
        docker::run_with_logging_quiet(&mut command, &log_path, ctx.timeout)?
    } else {
        docker::run_with_logging(&mut command, &log_path, ctx.output, ctx.timeout)?
    };

    Ok(TaskResult {
        name: def.name.clone(),
        scope: ctx.scope.to_string(),
        success,
        log_path,
    })
}

fn run_task(ctx: &ScopeContext, task: &GenerateTask, quiet: bool) -> Result<TaskResult> {
    match task {
        GenerateTask::Builtin { name, config_path } => {
            run_single_generator(ctx, name, config_path, quiet)
        }
        GenerateTask::Custom { def } => run_custom_generator(ctx, def, quiet),
    }
}

fn task_name(task: &GenerateTask) -> &str {
    match task {
        GenerateTask::Builtin { name, .. } => name,
        GenerateTask::Custom { def } => &def.name,
    }
}

fn run_sequential(ctx: &ScopeContext, tasks: &[GenerateTask]) -> Result<bool> {
    let mut failures = 0;
    for task in tasks {
        let name = task_name(task);
        let label = format!("Generate {} {name}", ctx.scope);
        ctx.output.substep_start(&label);

        let result = run_task(ctx, task, false)?;

        append_status(
            ctx.root,
            "generate",
            ctx.scope,
            name,
            if result.success { "ok" } else { "fail" },
            &result.log_path,
        )?;
        ctx.output.substep_finish(&label, result.success);
        if !result.success {
            failures += 1;
        }
    }
    Ok(failures == 0)
}

fn run_parallel(ctx: &ScopeContext, tasks: &[GenerateTask]) -> Result<bool> {
    let mp = ctx.output.multi_progress();
    let mp_ref = mp.as_ref();
    let mut all_failures = 0;

    for chunk in tasks.chunks(ctx.jobs) {
        let results: Vec<Result<TaskResult>> = std::thread::scope(|s| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|task| {
                    let name = task_name(task);
                    let label = format!("Generate {} {name}", ctx.scope);
                    let spinner = mp_ref.map(|m| ctx.output.add_parallel_spinner(m, &label));
                    s.spawn(move || {
                        let result = run_task(ctx, task, true);
                        if let Some(mp) = mp_ref {
                            let success = result.as_ref().map(|r| r.success).unwrap_or(false);
                            ctx.output.finish_parallel_spinner(
                                mp,
                                spinner.flatten(),
                                &label,
                                success,
                            );
                        }
                        result
                    })
                })
                .collect();

            handles
                .into_iter()
                .map(|h| h.join().expect("generator thread panicked"))
                .collect()
        });

        for result in results {
            let result = result?;
            append_status(
                ctx.root,
                "generate",
                ctx.scope,
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

fn resolve_configs(
    root: &Path,
    config_dir: &Path,
    requested: &[String],
    overrides: &HashMap<String, String>,
) -> Result<Vec<(String, PathBuf)>> {
    if !config_dir.is_dir() {
        bail!("Missing config directory: {}", config_dir.display());
    }

    let mut configs = Vec::new();

    // Determine which generators to use
    let generators: Vec<String> = if !requested.is_empty() {
        requested
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        // Use all yaml files in config_dir
        let mut names = Vec::new();
        for entry in fs::read_dir(config_dir).context("Failed to read generator directory")? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("yaml")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                names.push(stem.to_string());
            }
        }
        names
    };

    // Resolve each generator's config path
    for name in &generators {
        let path = if let Some(override_path) = overrides.get(name) {
            // User specified an override - resolve relative to root
            let resolved = root.join(override_path);
            if !resolved.is_file() {
                bail!(
                    "Generator override for '{}' points to invalid path: {}",
                    name,
                    override_path
                );
            }
            let canonical = resolved
                .canonicalize()
                .with_context(|| format!("Failed to resolve override path for '{}'", name))?;
            let canonical_root = root.canonicalize().context("Failed to resolve root path")?;
            if !canonical.starts_with(&canonical_root) {
                bail!(
                    "Generator override for '{}' resolves outside repository: {}",
                    name,
                    override_path
                );
            }
            canonical
        } else {
            // Use default from .oav/generators/{scope}/
            let default_path = config_dir.join(format!("{name}.yaml"));
            if !default_path.is_file() {
                bail!("Missing generator config: {}", default_path.display());
            }
            default_path
        };
        configs.push((name.clone(), path));
    }

    configs.sort_by(|(a, _), (b, _)| a.cmp(b));
    if configs.is_empty() {
        bail!("No generator configs found under {}", config_dir.display());
    }
    Ok(configs)
}
