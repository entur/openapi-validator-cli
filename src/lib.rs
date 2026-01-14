use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use include_dir::{include_dir, Dir, DirEntry};
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write as IoWrite};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");

const OAV_DIR: &str = ".oav";
const CONFIG_FILE: &str = ".oavc";

#[derive(Parser, Debug)]
#[command(name = "openapi-validator", version, about = "OpenAPI Validator CLI")]
struct Cli {
    #[arg(short, long, global = true, conflicts_with = "quiet")]
    verbose: bool,
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    quiet: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Init {
        #[arg(long)]
        spec: Option<String>,
        #[arg(long)]
        mode: Option<Mode>,
        #[arg(long, value_delimiter = ',')]
        server_generators: Option<Vec<String>>,
        #[arg(long, value_delimiter = ',')]
        client_generators: Option<Vec<String>>,
        #[arg(long)]
        ignore_config: bool,
    },
    Validate {
        #[arg(long)]
        spec: Option<String>,
        #[arg(long)]
        mode: Option<Mode>,
        #[arg(long, value_delimiter = ',')]
        server_generators: Option<Vec<String>>,
        #[arg(long, value_delimiter = ',')]
        client_generators: Option<Vec<String>>,
        #[arg(long)]
        skip_lint: bool,
        #[arg(long)]
        skip_generate: bool,
        #[arg(long)]
        skip_compile: bool,
    },
    Config {
        #[command(subcommand)]
        command: Option<ConfigCommand>,
    },
    Clean,
}

#[derive(Subcommand, Debug)]
enum ConfigCommand {
    Get { key: ConfigKey },
    Set { key: ConfigKey, value: String },
    Edit,
    Print,
    Ignore,
    Unignore,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum ConfigKey {
    Spec,
    Mode,
    Lint,
    Generate,
    Compile,
    ServerGenerators,
    ClientGenerators,
    GeneratorImage,
    RedoclyImage,
}

#[derive(ValueEnum, Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Mode {
    Server,
    Client,
    Both,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
struct Config {
    spec: Option<String>,
    mode: Mode,
    lint: bool,
    generate: bool,
    compile: bool,
    server_generators: Vec<String>,
    client_generators: Vec<String>,
    generator_image: String,
    redocly_image: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            spec: None,
            mode: Mode::Server,
            lint: true,
            generate: true,
            compile: true,
            server_generators: Vec::new(),
            client_generators: Vec::new(),
            generator_image: "openapitools/openapi-generator-cli:v7.17.0".to_string(),
            redocly_image: "redocly/cli:1.25.5".to_string(),
        }
    }
}

struct Output {
    verbose: bool,
    quiet: bool,
    color: bool,
    progress: bool,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let root = env::current_dir().context("Failed to determine current directory")?;
    let output = Output::new(cli.verbose, cli.quiet);

    match cli.command {
        Commands::Init {
            spec,
            mode,
            server_generators,
            client_generators,
            ignore_config,
        } => cmd_init(
            &root,
            &output,
            spec,
            mode,
            server_generators,
            client_generators,
            ignore_config,
        ),
        Commands::Validate {
            spec,
            mode,
            server_generators,
            client_generators,
            skip_lint,
            skip_generate,
            skip_compile,
        } => cmd_validate(
            &root,
            &output,
            spec,
            mode,
            server_generators,
            client_generators,
            skip_lint,
            skip_generate,
            skip_compile,
        ),
        Commands::Config { command } => cmd_config(&root, &output, command),
        Commands::Clean => cmd_clean(&root, &output),
    }
}

fn cmd_init(
    root: &Path,
    output: &Output,
    spec: Option<String>,
    mode: Option<Mode>,
    server_generators: Option<Vec<String>>,
    client_generators: Option<Vec<String>>,
    ignore_config: bool,
) -> Result<()> {
    ensure_oav_dir(root)?;
    ensure_gitignore(root, ignore_config)?;

    let mut config = load_config(root)?;
    if let Some(spec) = spec {
        config.spec = Some(spec);
    }
    if config.spec.is_none() {
        config.spec = discover_spec(root)?;
    }
    if let Some(mode) = mode {
        config.mode = mode;
    }
    if let Some(gens) = server_generators {
        config.server_generators = gens;
    }
    if let Some(gens) = client_generators {
        config.client_generators = gens;
    }

    let spec = match config.spec.clone() {
        Some(spec) => spec,
        None => bail!("No OpenAPI spec found. Pass --spec or set spec in .oavc."),
    };
    let spec_path = normalize_spec_path(root, &spec)?;
    config.spec = Some(spec_path.to_string_lossy().to_string());

    write_config(root, &config)?;
    extract_assets(root)?;

    output.println("Initialized OpenAPI Validator.");
    output.println(&format!("Config: {}", root.join(CONFIG_FILE).display()));
    output.println(&format!("Workspace: {}", root.join(OAV_DIR).display()));
    Ok(())
}

fn cmd_validate(
    root: &Path,
    output: &Output,
    spec_override: Option<String>,
    mode_override: Option<Mode>,
    server_generators: Option<Vec<String>>,
    client_generators: Option<Vec<String>>,
    skip_lint: bool,
    skip_generate: bool,
    skip_compile: bool,
) -> Result<()> {
    ensure_oav_dir(root)?;
    ensure_gitignore(root, false)?;
    extract_assets(root)?;

    let mut config = load_config(root)?;
    if let Some(spec) = spec_override {
        config.spec = Some(spec);
    }
    if let Some(mode) = mode_override {
        config.mode = mode;
    }
    if let Some(gens) = server_generators {
        config.server_generators = gens;
    }
    if let Some(gens) = client_generators {
        config.client_generators = gens;
    }
    if skip_lint {
        config.lint = false;
    }
    if skip_generate {
        config.generate = false;
    }
    if skip_compile {
        config.compile = false;
    }

    let spec = if let Some(spec) = config.spec.clone() {
        spec
    } else if let Some(spec) = discover_spec(root)? {
        spec
    } else {
        bail!("No OpenAPI spec found. Pass --spec or set spec in .oavc.");
    };

    let spec_path = normalize_spec_path(root, &spec)?;
    config.spec = Some(spec_path.to_string_lossy().to_string());

    if config.lint || config.generate || config.compile {
        ensure_docker()?;
    }

    prepare_runtime_dirs(root)?;
    write_config(root, &config)?;

    let mut failures = 0;
    if config.lint {
        let success = run_step(output, "Lint", true, || {
            run_lint(root, &spec_path, &config.redocly_image, output)
        })?;
        if !success {
            failures += 1;
        }
    }

    if config.generate {
        let success = run_step(output, "Generate", false, || {
            run_generate(root, &spec_path, &config, output)
        })?;
        if !success {
            failures += 1;
        }
    }

    if config.compile {
        if config.generate {
            let success =
                run_step(output, "Compile", false, || run_compile(root, &config, output))?;
            if !success {
                failures += 1;
            }
        } else {
            output.println("Skipping compile (generate disabled)");
        }
    }

    let _ = run_step(output, "Report", true, || run_dashboard(root, output));

    if failures > 0 {
        bail!("Validation finished with failures. See .oav/reports for details.");
    }

    if !output.quiet {
        output.println("Validation complete.");
    }
    output.println_always(&format!(
        "Generated: {}",
        root.join(OAV_DIR).join("generated").display()
    ));
    output.println_always(&format!(
        "Reports: {}",
        root.join(OAV_DIR).join("reports").display()
    ));
    output.println_always(&format!(
        "Dashboard: {}",
        root.join(OAV_DIR).join("reports").join("dashboard.html").display()
    ));
    Ok(())
}

fn cmd_config(root: &Path, output: &Output, command: Option<ConfigCommand>) -> Result<()> {
    ensure_gitignore(root, false)?;

    match command.unwrap_or(ConfigCommand::Print) {
        ConfigCommand::Get { key } => {
            let config = load_config(root)?;
            print_config_value(&config, key)?;
        }
        ConfigCommand::Set { key, value } => {
            let mut config = load_config(root)?;
            set_config_value(&mut config, key, value)?;
            write_config(root, &config)?;
            output.println(&format!("Updated {}", root.join(CONFIG_FILE).display()));
        }
        ConfigCommand::Edit => {
            let path = root.join(CONFIG_FILE);
            if !path.exists() {
                write_config(root, &Config::default())?;
            }
            let editor = env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
            let status = Command::new(editor)
                .arg(&path)
                .status()
                .context("Failed to open editor")?;
            if !status.success() {
                bail!("Editor exited with a non-zero status");
            }
        }
        ConfigCommand::Print => {
            let config = load_config(root)?;
            let yaml = serde_yaml::to_string(&config).context("Failed to serialize config")?;
            print!("{yaml}");
        }
        ConfigCommand::Ignore => {
            ensure_gitignore(root, true)?;
            output.println("Added .oavc to .gitignore.");
        }
        ConfigCommand::Unignore => {
            remove_gitignore_entries(root, &[".oavc"])?;
            output.println("Removed .oavc from .gitignore.");
        }
    }
    Ok(())
}

fn cmd_clean(root: &Path, output: &Output) -> Result<()> {
    let path = root.join(OAV_DIR);
    if path.exists() {
        fs::remove_dir_all(&path).context("Failed to remove .oav directory")?;
        output.println(&format!("Removed {}", path.display()));
    } else {
        output.println("No .oav directory found.");
    }
    Ok(())
}

fn extract_assets(root: &Path) -> Result<()> {
    let target = root.join(OAV_DIR);
    fs::create_dir_all(&target).context("Failed to create .oav directory")?;
    write_assets(&target, &ASSETS)?;
    Ok(())
}

fn write_assets(target: &Path, dir: &Dir) -> Result<()> {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(child) => {
                let dest = target.join(child.path());
                fs::create_dir_all(&dest)
                    .with_context(|| format!("Failed to create {}", dest.display()))?;
                write_assets(target, child)?;
            }
            DirEntry::File(file) => {
                let dest = target.join(file.path());
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("Failed to create {}", parent.display()))?;
                }
                fs::write(&dest, file.contents())
                    .with_context(|| format!("Failed to write {}", dest.display()))?;
                set_script_permissions(&dest)?;
            }
        }
    }
    Ok(())
}

fn set_script_permissions(path: &Path) -> Result<()> {
    if path.extension().and_then(|ext| ext.to_str()) != Some("sh") {
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perm = fs::Permissions::from_mode(0o755);
        fs::set_permissions(path, perm)
            .with_context(|| format!("Failed to set permissions on {}", path.display()))?;
    }
    Ok(())
}

fn ensure_oav_dir(root: &Path) -> Result<()> {
    fs::create_dir_all(root.join(OAV_DIR)).context("Failed to create .oav directory")?;
    Ok(())
}

fn ensure_gitignore(root: &Path, ignore_config: bool) -> Result<()> {
    let mut entries = vec![".oav/"];
    if ignore_config {
        entries.push(".oavc");
    }
    add_gitignore_entries(root, &entries)
}

fn add_gitignore_entries(root: &Path, entries: &[&str]) -> Result<()> {
    let path = root.join(".gitignore");
    let mut content = if path.exists() {
        fs::read_to_string(&path).context("Failed to read .gitignore")?
    } else {
        String::new()
    };

    let mut changed = false;
    for entry in entries {
        if !content.lines().any(|line| line.trim() == *entry) {
            if !content.ends_with('\n') && !content.is_empty() {
                content.push('\n');
            }
            content.push_str(entry);
            content.push('\n');
            changed = true;
        }
    }

    if changed {
        fs::write(&path, content).context("Failed to update .gitignore")?;
    }
    Ok(())
}

fn remove_gitignore_entries(root: &Path, entries: &[&str]) -> Result<()> {
    let path = root.join(".gitignore");
    if !path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(&path).context("Failed to read .gitignore")?;
    let mut kept: Vec<&str> = Vec::new();
    for line in content.lines() {
        if entries.iter().any(|entry| line.trim() == *entry) {
            continue;
        }
        kept.push(line);
    }
    let mut new_content = kept.join("\n");
    if !new_content.is_empty() {
        new_content.push('\n');
    }
    fs::write(&path, new_content).context("Failed to update .gitignore")?;
    Ok(())
}

fn prepare_runtime_dirs(root: &Path) -> Result<()> {
    let oav_dir = root.join(OAV_DIR);
    fs::create_dir_all(oav_dir.join("reports").join("lint"))?;
    fs::create_dir_all(oav_dir.join("reports").join("generate").join("server"))?;
    fs::create_dir_all(oav_dir.join("reports").join("generate").join("client"))?;
    fs::create_dir_all(oav_dir.join("reports").join("compile").join("server"))?;
    fs::create_dir_all(oav_dir.join("reports").join("compile").join("client"))?;
    fs::create_dir_all(oav_dir.join("generated"))?;
    fs::write(oav_dir.join("status.tsv"), "")?;
    Ok(())
}

fn run_generate(root: &Path, spec_path: &Path, config: &Config, output: &Output) -> Result<bool> {
    let mut failures = 0;
    let reports_root = root.join(OAV_DIR).join("reports").join("generate");
    let server_dir = root.join(OAV_DIR).join("generators").join("server");
    let client_dir = root.join(OAV_DIR).join("generators").join("client");

    match config.mode {
        Mode::Server => {
            if !run_generate_for_label(
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
        Mode::Client => {
            if !run_generate_for_label(
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
        Mode::Both => {
            if !run_generate_for_label(
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
            if !run_generate_for_label(
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
    }

    Ok(failures == 0)
}

fn run_generate_for_label(
    root: &Path,
    spec_path: &Path,
    generator_image: &str,
    label: &str,
    config_dir: &Path,
    requested: &[String],
    reports_root: &Path,
    output: &Output,
) -> Result<bool> {
    let report_dir = reports_root.join(label);
    fs::create_dir_all(&report_dir).context("Failed to create generate report directory")?;
    let error_log = report_dir.join("_errors.log");

    let configs = match resolve_generator_configs(config_dir, requested) {
        Ok(configs) => configs,
        Err(err) => {
            append_error(&error_log, &err.to_string())?;
            append_status(root, "generate", label, "_config_", "fail", &error_log)?;
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
        let container_config = format!("/work/{}", config_rel.to_string_lossy());
        let container_spec = format!("/work/{}", spec_path.to_string_lossy());

        let command_line = format!(
            "$ docker run --rm {user} -v {root}:/work -w /work/{oav} {image} generate -i {spec} -c {config}",
            user = docker_user_flag(),
            root = root.display(),
            oav = OAV_DIR,
            image = generator_image,
            spec = container_spec,
            config = container_config
        )
        .replace("  ", " ");
        write_log_header(&log_path, &command_line)?;

        output.substep_start(&format!("Generate {label} {name}"));
        let mut command = Command::new("docker");
        command
            .arg("run")
            .arg("--rm")
            .args(docker_user_args())
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

        let success = run_command_with_logging(&mut command, &log_path, output)?;
        append_status(
            root,
            "generate",
            label,
            name,
            if success { "ok" } else { "fail" },
            &log_path,
        )?;
        output.substep_finish(&format!("Generate {label} {name}"), success);
        if !success {
            failures += 1;
        }
    }

    Ok(failures == 0)
}

fn resolve_generator_configs(config_dir: &Path, requested: &[String]) -> Result<Vec<PathBuf>> {
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

fn run_compile(root: &Path, config: &Config, output: &Output) -> Result<bool> {
    let reports_root = root.join(OAV_DIR).join("reports").join("compile");
    fs::create_dir_all(&reports_root).context("Failed to create compile reports directory")?;

    let mut tasks = Vec::new();
    match config.mode {
        Mode::Server => {
            tasks.extend(resolve_compile_tasks(
                "server",
                &config.server_generators,
                &SUPPORTED_SERVER_GENERATORS,
                "build-",
            )?);
        }
        Mode::Client => {
            tasks.extend(resolve_compile_tasks(
                "client",
                &config.client_generators,
                &SUPPORTED_CLIENT_GENERATORS,
                "build-client-",
            )?);
        }
        Mode::Both => {
            tasks.extend(resolve_compile_tasks(
                "server",
                &config.server_generators,
                &SUPPORTED_SERVER_GENERATORS,
                "build-",
            )?);
            tasks.extend(resolve_compile_tasks(
                "client",
                &config.client_generators,
                &SUPPORTED_CLIENT_GENERATORS,
                "build-client-",
            )?);
        }
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

        let success = run_command_with_logging(&mut command, &log_path, output)?;
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

fn run_lint(
    root: &Path,
    spec_path: &Path,
    redocly_image: &str,
    output: &Output,
) -> Result<bool> {
    let reports_dir = root.join(OAV_DIR).join("reports").join("lint");
    fs::create_dir_all(&reports_dir).context("Failed to create lint reports directory")?;
    let log_path = reports_dir.join("redocly.log");

    let workspace = root.to_string_lossy().to_string();
    let container_root = format!("/work/{}", OAV_DIR);
    let spec = format!("/work/{}", spec_path.to_string_lossy());
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

    let success = run_command_with_logging(&mut command, &log_path, output)?;
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

fn run_dashboard(root: &Path, output: &Output) -> Result<bool> {
    let reports_dir = root.join(OAV_DIR).join("reports");
    fs::create_dir_all(&reports_dir).context("Failed to create reports directory")?;
    let status_path = root.join(OAV_DIR).join("status.tsv");
    let output_path = reports_dir.join("dashboard.html");

    let entries = load_status_entries(&status_path)?;
    let total = entries.len();
    let passed = entries.iter().filter(|entry| entry.status == "ok").count();
    let failed = entries.iter().filter(|entry| entry.status == "fail").count();

    let mut html = String::new();
    html.push_str(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>OpenAPI Validator Report</title>
  <style>
    :root {
      --bg: #0d1117; --fg: #c9d1d9; --border: #30363d;
      --green: #238636; --red: #da3633; --yellow: #d29922;
      --link: #58a6ff; --code-bg: #161b22;
    }
    * { box-sizing: border-box; }
    body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
           background: var(--bg); color: var(--fg); margin: 0; padding: 20px; line-height: 1.5; }
    h1, h2, h3 { margin-top: 0; font-weight: 600; }
    h1 { border-bottom: 1px solid var(--border); padding-bottom: 10px; }
    .summary { display: flex; gap: 20px; margin-bottom: 30px; flex-wrap: wrap; }
    .stat { background: var(--code-bg); border: 1px solid var(--border); border-radius: 6px;
            padding: 16px 24px; text-align: center; min-width: 120px; }
    .stat-value { font-size: 2em; font-weight: 600; }
    .stat-label { color: #8b949e; font-size: 0.9em; }
    .stat.pass .stat-value { color: var(--green); }
    .stat.fail .stat-value { color: var(--red); }
    .section { margin-bottom: 30px; }
    .result-table { width: 100%; border-collapse: collapse; background: var(--code-bg);
                   border: 1px solid var(--border); border-radius: 6px; overflow: hidden; }
    .result-table th, .result-table td { padding: 12px; text-align: left; border-bottom: 1px solid var(--border); }
    .result-table th { background: var(--bg); font-weight: 600; }
    .result-table tr:last-child td { border-bottom: none; }
    .badge { display: inline-block; padding: 2px 8px; border-radius: 12px; font-size: 0.85em; font-weight: 500; }
    .badge.ok { background: var(--green); color: #fff; }
    .badge.fail { background: var(--red); color: #fff; }
    details { background: var(--code-bg); border: 1px solid var(--border); border-radius: 6px; margin-top: 10px; }
    summary { padding: 12px; cursor: pointer; font-weight: 500; }
    summary:hover { background: var(--border); }
    pre { margin: 0; padding: 16px; overflow-x: auto; font-size: 0.85em;
          background: var(--bg); border-top: 1px solid var(--border); max-height: 500px; overflow-y: auto; }
    code { font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, monospace; }
    a { color: var(--link); text-decoration: none; }
    a:hover { text-decoration: underline; }
    .empty { color: #8b949e; font-style: italic; }
  </style>
</head>
<body>
  <h1>OpenAPI Validator Report</h1>
  <div class="summary">
"#,
    );

    html.push_str(&format!(
        r#"    <div class="stat">
      <div class="stat-value">{total}</div>
      <div class="stat-label">Total</div>
    </div>
    <div class="stat pass">
      <div class="stat-value">{passed}</div>
      <div class="stat-label">Passed</div>
    </div>
    <div class="stat fail">
      <div class="stat-value">{failed}</div>
      <div class="stat-label">Failed</div>
    </div>
  </div>
"#,
    ));

    for section in ["lint", "generate", "compile"] {
        let title = match section {
            "lint" => "Lint",
            "generate" => "Generate",
            "compile" => "Compile",
            _ => section,
        };
        let section_entries: Vec<&StatusEntry> =
            entries.iter().filter(|entry| entry.stage == section).collect();
        if section_entries.is_empty() {
            continue;
        }

        html.push_str(&format!(
            r#"  <div class="section">
    <h2>{title}</h2>
    <table class="result-table">
      <thead>
        <tr><th>Scope</th><th>Target</th><th>Status</th><th>Log</th></tr>
      </thead>
      <tbody>
"#
        ));

        for entry in section_entries {
            let badge = html_escape(&entry.status);
            let scope = html_escape(&entry.scope);
            let target = html_escape(&entry.target);
            let log_path = Path::new(&entry.log_path);
            let log_basename = log_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("log");
            let log_content = html_escape(&read_log_snippet(log_path));

            html.push_str(&format!(
                r#"        <tr>
          <td>{scope}</td>
          <td>{target}</td>
          <td><span class="badge {badge}">{badge}</span></td>
          <td>
            <details>
              <summary>{log_basename}</summary>
              <pre><code>{log_content}</code></pre>
            </details>
          </td>
        </tr>
"#
            ));
        }

        html.push_str(
            r#"      </tbody>
    </table>
  </div>
"#,
        );
    }

    html.push_str(
        r#"  <footer style="margin-top: 40px; padding-top: 20px; border-top: 1px solid var(--border); color: #8b949e; font-size: 0.85em;">
    Generated by OpenAPI Validator.
  </footer>
</body>
</html>
"#,
    );

    if let Err(err) = fs::write(&output_path, html) {
        if !output.quiet {
            eprintln!("Report generation failed: {err}");
        }
        return Ok(false);
    }

    Ok(true)
}

fn run_command_with_logging(
    command: &mut Command,
    log_path: &Path,
    output: &Output,
) -> Result<bool> {
    if output.verbose {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .context("Failed to start Docker command")?;
        let stdout = child.stdout.take().context("Missing stdout")?;
        let stderr = child.stderr.take().context("Missing stderr")?;

        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .context("Failed to open log file")?;
        let log = Arc::new(Mutex::new(log_file));

        let out_log = Arc::clone(&log);
        let out_handle = thread::spawn(move || stream_output(stdout, io::stdout(), out_log));
        let err_log = Arc::clone(&log);
        let err_handle = thread::spawn(move || stream_output(stderr, io::stderr(), err_log));

        let status = child.wait().context("Failed to wait for command")?;
        let _ = out_handle.join();
        let _ = err_handle.join();
        Ok(status.success())
    } else {
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .context("Failed to open log file")?;
        let log_err = log_file.try_clone().context("Failed to clone log file")?;
        command.stdout(Stdio::from(log_file)).stderr(Stdio::from(log_err));
        let status = command.status().context("Failed to run Docker command")?;
        Ok(status.success())
    }
}

fn stream_output<R: Read + Send + 'static>(
    mut reader: R,
    mut writer: impl IoWrite + Send + 'static,
    log: Arc<Mutex<File>>,
) -> io::Result<()> {
    let mut buffer = [0u8; 8192];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        {
            let mut file = log.lock().map_err(|_| {
                io::Error::new(io::ErrorKind::Other, "Log file lock poisoned")
            })?;
            file.write_all(&buffer[..count])?;
        }
        writer.write_all(&buffer[..count])?;
        writer.flush()?;
    }
    Ok(())
}

fn write_log_header(log_path: &Path, command_line: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(log_path)
        .context("Failed to create log file")?;
    writeln!(file, "{command_line}")?;
    writeln!(file)?;
    Ok(())
}

fn append_status(
    root: &Path,
    stage: &str,
    scope: &str,
    target: &str,
    status: &str,
    log_path: &Path,
) -> Result<()> {
    let status_path = root.join(OAV_DIR).join("status.tsv");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(status_path)
        .context("Failed to open status file")?;
    writeln!(
        file,
        "{stage}\t{scope}\t{target}\t{status}\t{}",
        log_path.display()
    )?;
    Ok(())
}

fn append_error(log_path: &Path, message: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .context("Failed to write error log")?;
    writeln!(file, "{message}")?;
    Ok(())
}

fn docker_user_args() -> Vec<String> {
    #[cfg(unix)]
    {
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        return vec!["--user".to_string(), format!("{uid}:{gid}")];
    }
    #[cfg(not(unix))]
    {
        Vec::new()
    }
}

fn docker_user_flag() -> String {
    #[cfg(unix)]
    {
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        return format!("--user {uid}:{gid}");
    }
    #[cfg(not(unix))]
    {
        String::new()
    }
}

struct CompileTask {
    scope: String,
    service: String,
    name: String,
}

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

fn resolve_compile_tasks(
    scope: &str,
    requested: &[String],
    supported: &[&str],
    prefix: &str,
) -> Result<Vec<CompileTask>> {
    let mut tasks = Vec::new();
    let names: Vec<String> = if !requested.is_empty() {
        requested
            .iter()
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
            .collect()
    } else {
        supported.iter().map(|name| (*name).to_string()).collect()
    };

    for name in names {
        if !supported.contains(&name.as_str()) {
            bail!("Unsupported {scope} generator for compile: {name}");
        }
        let service = format!("{prefix}{name}");
        tasks.push(CompileTask {
            scope: scope.to_string(),
            service,
            name,
        });
    }
    Ok(tasks)
}

#[derive(Debug)]
struct StatusEntry {
    stage: String,
    scope: String,
    target: String,
    status: String,
    log_path: String,
}

fn load_status_entries(status_path: &Path) -> Result<Vec<StatusEntry>> {
    if !status_path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(status_path).context("Failed to read status file")?;
    let mut entries = Vec::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 5 {
            continue;
        }
        entries.push(StatusEntry {
            stage: parts[0].to_string(),
            scope: parts[1].to_string(),
            target: parts[2].to_string(),
            status: parts[3].to_string(),
            log_path: parts[4].to_string(),
        });
    }
    Ok(entries)
}

fn read_log_snippet(path: &Path) -> String {
    let file = File::open(path);
    let mut content = Vec::new();
    if let Ok(file) = file {
        let _ = file.take(100000).read_to_end(&mut content);
        return String::from_utf8_lossy(&content).to_string();
    }
    format!("Log file not found: {}", path.display())
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn load_config(root: &Path) -> Result<Config> {
    let path = root.join(CONFIG_FILE);
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = fs::read_to_string(&path).context("Failed to read .oavc")?;
    let config = serde_yaml::from_str(&content).context("Failed to parse .oavc")?;
    Ok(config)
}

fn write_config(root: &Path, config: &Config) -> Result<()> {
    let path = root.join(CONFIG_FILE);
    let content = serde_yaml::to_string(config).context("Failed to serialize config")?;
    fs::write(&path, content).context("Failed to write .oavc")?;
    Ok(())
}

fn print_config_value(config: &Config, key: ConfigKey) -> Result<()> {
    match key {
        ConfigKey::Spec => {
            if let Some(spec) = &config.spec {
                println!("{spec}");
            }
        }
        ConfigKey::Mode => println!("{}", mode_to_string(config.mode)),
        ConfigKey::Lint => println!("{}", config.lint),
        ConfigKey::Generate => println!("{}", config.generate),
        ConfigKey::Compile => println!("{}", config.compile),
        ConfigKey::ServerGenerators => println!("{}", config.server_generators.join(",")),
        ConfigKey::ClientGenerators => println!("{}", config.client_generators.join(",")),
        ConfigKey::GeneratorImage => println!("{}", config.generator_image),
        ConfigKey::RedoclyImage => println!("{}", config.redocly_image),
    }
    Ok(())
}

fn set_config_value(config: &mut Config, key: ConfigKey, value: String) -> Result<()> {
    match key {
        ConfigKey::Spec => config.spec = Some(value),
        ConfigKey::Mode => config.mode = parse_mode(&value)?,
        ConfigKey::Lint => config.lint = parse_bool(&value)?,
        ConfigKey::Generate => config.generate = parse_bool(&value)?,
        ConfigKey::Compile => config.compile = parse_bool(&value)?,
        ConfigKey::ServerGenerators => config.server_generators = parse_list(&value),
        ConfigKey::ClientGenerators => config.client_generators = parse_list(&value),
        ConfigKey::GeneratorImage => config.generator_image = value,
        ConfigKey::RedoclyImage => config.redocly_image = value,
    }
    Ok(())
}

fn parse_mode(raw: &str) -> Result<Mode> {
    match raw.trim().to_lowercase().as_str() {
        "server" => Ok(Mode::Server),
        "client" => Ok(Mode::Client),
        "both" => Ok(Mode::Both),
        _ => bail!("Invalid mode: {raw} (expected server, client, or both)"),
    }
}

fn parse_bool(raw: &str) -> Result<bool> {
    match raw.trim().to_lowercase().as_str() {
        "true" | "1" | "yes" | "y" => Ok(true),
        "false" | "0" | "no" | "n" => Ok(false),
        _ => bail!("Invalid boolean: {raw} (expected true/false)"),
    }
}

fn parse_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(|item| item.to_string())
        .collect()
}

fn mode_to_string(mode: Mode) -> String {
    match mode {
        Mode::Server => "server",
        Mode::Client => "client",
        Mode::Both => "both",
    }
    .to_string()
}

fn ensure_docker() -> Result<()> {
    let status = Command::new("docker")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => bail!("Docker is installed but not responding. Is the daemon running?"),
        Err(_) => bail!("Docker not found in PATH."),
    }
}

fn normalize_spec_path(root: &Path, spec: &str) -> Result<PathBuf> {
    let spec_path = PathBuf::from(spec);
    let absolute = if spec_path.is_absolute() {
        spec_path
    } else {
        root.join(&spec_path)
    };
    if !absolute.exists() {
        bail!("Spec file not found: {}", absolute.display());
    }
    let relative = absolute
        .strip_prefix(root)
        .context("Spec path must be inside the repository")?;
    Ok(relative.to_path_buf())
}

fn discover_spec(root: &Path) -> Result<Option<String>> {
    for name in ["openapi.yaml", "openapi.yml"] {
        let candidate = root.join(name);
        if candidate.is_file() {
            return Ok(Some(name.to_string()));
        }
    }

    let mut matches = Vec::new();
    let walker = walkdir::WalkDir::new(root)
        .max_depth(4)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !should_skip_entry(entry));

    for entry in walker.filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !is_yaml(path) {
            continue;
        }
        if !is_openapi_spec(path) {
            continue;
        }
        if let Ok(rel) = path.strip_prefix(root) {
            matches.push(rel.to_string_lossy().to_string());
        }
    }

    if matches.is_empty() {
        return Ok(None);
    }

    matches.sort();
    select_spec_from_candidates(matches)
}

fn is_yaml(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_lowercase()),
        Some(ext) if ext == "yaml" || ext == "yml"
    )
}

fn is_openapi_spec(path: &Path) -> bool {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return false,
    };
    let mut content = String::new();
    if file.read_to_string(&mut content).is_err() {
        return false;
    }
    let doc: serde_yaml::Value = match serde_yaml::from_str(&content) {
        Ok(doc) => doc,
        Err(_) => return false,
    };
    match doc {
        serde_yaml::Value::Mapping(mapping) => mapping
            .keys()
            .filter_map(|key| key.as_str())
            .any(|key| key == "openapi"),
        _ => false,
    }
}

fn should_skip_entry(entry: &walkdir::DirEntry) -> bool {
    if entry.depth() == 0 {
        return false;
    }
    if !entry.file_type().is_dir() {
        return false;
    }
    match entry.file_name().to_str().unwrap_or_default() {
        ".git" | ".oav" | "target" | "node_modules" | ".idea" | ".vscode" => true,
        _ => false,
    }
}

fn select_spec_from_candidates(candidates: Vec<String>) -> Result<Option<String>> {
    println!("No default OpenAPI spec found.");
    println!("Select a spec to use:");
    for (idx, path) in candidates.iter().enumerate() {
        println!("  {}) {}", idx + 1, path);
    }
    println!("  q) quit");

    let mut input = String::new();
    loop {
        print!("Select [1-{}] or q: ", candidates.len());
        io::stdout().flush().context("Failed to flush stdout")?;
        input.clear();
        io::stdin()
            .read_line(&mut input)
            .context("Failed to read input")?;
        let trimmed = input.trim();
        if trimmed.eq_ignore_ascii_case("q") {
            return Ok(None);
        }
        if let Ok(choice) = trimmed.parse::<usize>() {
            if choice >= 1 && choice <= candidates.len() {
                return Ok(Some(candidates[choice - 1].clone()));
            }
        }
        println!("Invalid selection.");
    }
}

fn run_step(
    output: &Output,
    label: &str,
    show_spinner: bool,
    action: impl FnOnce() -> Result<bool>,
) -> Result<bool> {
    let spinner = output.start_step(label, show_spinner);
    let result = action();
    let success = result.as_ref().map(|ok| *ok).unwrap_or(false);
    output.finish_step(spinner.as_ref(), label, success);
    result
}

impl Output {
    fn new(verbose: bool, quiet: bool) -> Self {
        let is_tty = atty::is(atty::Stream::Stdout);
        let color = is_tty && env::var_os("NO_COLOR").is_none();
        let progress = is_tty && !verbose && !quiet;
        Self {
            verbose,
            quiet,
            color,
            progress,
        }
    }

    fn start_step(&self, label: &str, show_spinner: bool) -> Option<ProgressBar> {
        if show_spinner && self.progress {
            let spinner = ProgressBar::new_spinner();
            let style = ProgressStyle::with_template("{spinner} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner())
                .tick_strings(&["-", "\\", "|", "/"]);
            spinner.set_style(style);
            spinner.set_message(label.to_string());
            spinner.enable_steady_tick(std::time::Duration::from_millis(100));
            Some(spinner)
        } else {
            if self.verbose && !self.quiet {
                println!("==> {label}");
            }
            None
        }
    }

    fn finish_step(&self, spinner: Option<&ProgressBar>, label: &str, success: bool) {
        if self.quiet {
            if let Some(spinner) = spinner {
                spinner.finish_and_clear();
            }
            return;
        }

        let status = self.format_status(success);
        let message = format!("{status} {label}");
        match spinner {
            Some(spinner) => spinner.finish_with_message(message),
            None => println!("{message}"),
        }
    }

    fn println(&self, message: &str) {
        if !self.quiet {
            println!("{message}");
        }
    }

    fn println_always(&self, message: &str) {
        println!("{message}");
    }

    fn substep_start(&self, label: &str) {
        if !self.quiet {
            println!("{label}...");
        }
    }

    fn substep_finish(&self, label: &str, success: bool) {
        if !self.quiet {
            let status = self.format_status(success);
            println!("{status} {label}");
        }
    }

    fn format_status(&self, success: bool) -> String {
        if self.color {
            if success {
                "✓".green().to_string()
            } else {
                "✗".red().to_string()
            }
        } else if success {
            "OK".to_string()
        } else {
            "FAIL".to_string()
        }
    }
}
