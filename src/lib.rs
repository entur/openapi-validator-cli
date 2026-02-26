mod cli;
mod config;
mod docker;
mod generators;
mod json_report;
mod output;
mod steps;
mod util;

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_VALIDATION_FAILURE: i32 = 1;
pub const EXIT_INFRA_ERROR: i32 = 2;

use anyhow::{Context, Result, bail};
use clap::Parser;
use include_dir::{Dir, include_dir};
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use cli::{Cli, Commands, ConfigCommand};
use config::{CONFIG_FILE, Config};
use output::Output;
use util::OAV_DIR;

pub use cli::OutputFormat;

static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");

struct InitArgs {
    spec: Option<String>,
    mode: Option<cli::Mode>,
    server_generators: Option<Vec<String>>,
    client_generators: Option<Vec<String>>,
    ignore_config: bool,
    search_depth: Option<usize>,
}

struct ValidateArgs {
    spec: Option<String>,
    mode: Option<cli::Mode>,
    server_generators: Option<Vec<String>>,
    client_generators: Option<Vec<String>>,
    skip_lint: bool,
    skip_generate: bool,
    skip_compile: bool,
    linter: Option<cli::Linter>,
    ruleset: Option<String>,
    docker_timeout: Option<u64>,
    search_depth: Option<usize>,
}

/// Run the CLI. Returns the requested output format so the caller can
/// format errors appropriately.
pub fn run() -> (OutputFormat, Result<()>) {
    let cli = Cli::parse();
    let json = cli.output == OutputFormat::Json;
    let root = match env::current_dir().context("Failed to determine current directory") {
        Ok(r) => r,
        Err(e) => return (cli.output, Err(e)),
    };
    let output = Output::new(cli.verbose, cli.quiet, json, cli.color);

    let result = match cli.command {
        Commands::Init {
            spec,
            mode,
            server_generators,
            client_generators,
            ignore_config,
            search_depth,
        } => cmd_init(
            &root,
            &output,
            InitArgs {
                spec,
                mode,
                server_generators,
                client_generators,
                ignore_config,
                search_depth,
            },
        ),
        Commands::Validate {
            spec,
            mode,
            server_generators,
            client_generators,
            skip_lint,
            skip_generate,
            skip_compile,
            linter,
            ruleset,
            docker_timeout,
            search_depth,
        } => cmd_validate(
            &root,
            &output,
            ValidateArgs {
                spec,
                mode,
                server_generators,
                client_generators,
                skip_lint,
                skip_generate,
                skip_compile,
                linter,
                ruleset,
                docker_timeout,
                search_depth,
            },
        ),
        Commands::Config { command } => cmd_config(&root, &output, command),
        Commands::Clean => cmd_clean(&root, &output),
    };

    (cli.output, result)
}

fn cmd_init(root: &Path, output: &Output, args: InitArgs) -> Result<()> {
    let mut cfg = config::load(root)?;
    util::ensure_oav_dir(root)?;
    if cfg.manage_gitignore {
        util::add_gitignore_entries(root, &[".oav/"])?;
        if args.ignore_config {
            util::add_gitignore_entries(root, &[".oavc"])?;
        }
    }
    if let Some(d) = args.search_depth {
        if d == 0 {
            bail!("--search-depth must be greater than 0");
        }
        cfg.search_depth = d;
    }
    if let Some(s) = args.spec {
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() {
            bail!("--spec cannot be blank");
        }
        cfg.spec = Some(trimmed);
    }
    if cfg.spec.is_none() {
        cfg.spec = util::discover_spec(root, output.quiet, cfg.search_depth)?;
    }
    if let Some(m) = args.mode {
        cfg.mode = m;
    }
    if let Some(gens) = args.server_generators {
        let gens: Vec<String> = gens
            .iter()
            .map(|g| g.trim().to_string())
            .filter(|g| !g.is_empty())
            .collect();
        config::validate_generators("server", &gens, &generators::server_names())?;
        cfg.server_generators = gens;
    }
    if let Some(gens) = args.client_generators {
        let gens: Vec<String> = gens
            .iter()
            .map(|g| g.trim().to_string())
            .filter(|g| !g.is_empty())
            .collect();
        config::validate_generators("client", &gens, &generators::client_names())?;
        cfg.client_generators = gens;
    }

    let spec = cfg.spec.clone().ok_or_else(|| {
        anyhow::anyhow!("No OpenAPI spec found. Pass --spec or set spec in .oavc.")
    })?;
    let spec_path = util::normalize_spec_path(root, &spec)?;
    cfg.spec = Some(spec_path.to_string_lossy().to_string());

    config::validate(&cfg)?;
    config::write(root, &cfg)?;
    util::extract_assets(root, &ASSETS)?;

    output.println("Initialized OpenAPI Validator.");
    output.println(&format!("Config: {}", root.join(CONFIG_FILE).display()));
    output.println(&format!("Workspace: {}", root.join(OAV_DIR).display()));
    Ok(())
}

fn cmd_validate(root: &Path, output: &Output, args: ValidateArgs) -> Result<()> {
    let mut cfg = config::load(root)?;
    util::ensure_oav_dir(root)?;
    if cfg.manage_gitignore {
        util::add_gitignore_entries(root, &[".oav/"])?;
    }
    util::extract_assets(root, &ASSETS)?;
    if let Some(t) = args.docker_timeout {
        if t == 0 {
            bail!("--docker-timeout must be greater than 0");
        }
        cfg.docker_timeout = t;
    }
    if let Some(d) = args.search_depth {
        if d == 0 {
            bail!("--search-depth must be greater than 0");
        }
        cfg.search_depth = d;
    }
    if let Some(s) = args.spec {
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() {
            bail!("--spec cannot be blank");
        }
        cfg.spec = Some(trimmed);
    }
    if let Some(m) = args.mode {
        cfg.mode = m;
    }
    if let Some(gens) = args.server_generators {
        let gens: Vec<String> = gens
            .iter()
            .map(|g| g.trim().to_string())
            .filter(|g| !g.is_empty())
            .collect();
        config::validate_generators("server", &gens, &generators::server_names())?;
        cfg.server_generators = gens;
    }
    if let Some(gens) = args.client_generators {
        let gens: Vec<String> = gens
            .iter()
            .map(|g| g.trim().to_string())
            .filter(|g| !g.is_empty())
            .collect();
        config::validate_generators("client", &gens, &generators::client_names())?;
        cfg.client_generators = gens;
    }
    if args.skip_lint {
        cfg.lint = false;
    }
    if args.skip_generate {
        cfg.generate = false;
    }
    if args.skip_compile {
        cfg.compile = false;
    }
    if let Some(l) = args.linter {
        cfg.linter = l;
    }
    if let Some(r) = args.ruleset {
        cfg.spectral_ruleset = r;
    }

    let spec = if let Some(s) = cfg.spec.clone() {
        s
    } else if let Some(s) = util::discover_spec(root, output.quiet, cfg.search_depth)? {
        s
    } else {
        bail!("No OpenAPI spec found. Pass --spec or set spec in .oavc.");
    };

    let spec_path = util::normalize_spec_path(root, &spec)?;
    cfg.spec = Some(spec_path.to_string_lossy().to_string());

    if cfg.lint || cfg.generate || cfg.compile {
        docker::ensure_available()?;
    }

    config::validate(&cfg)?;
    util::prepare_runtime_dirs(root)?;
    config::write(root, &cfg)?;

    let mut failures = 0;

    if cfg.lint && cfg.linter != cli::Linter::None {
        let success = steps::run_step(output, "Lint", true, true, || {
            steps::lint(root, &spec_path, &cfg, output)
        })?;
        if !success {
            failures += 1;
        }
    }

    if cfg.generate {
        output.phase_header("Generate");
        let success = steps::run_step(output, "Generate", false, false, || {
            steps::generate(root, &spec_path, &cfg, output)
        })?;
        if !success {
            failures += 1;
        }
    }

    if cfg.compile {
        if cfg.generate {
            output.phase_header("Compile");
            let success = steps::run_step(output, "Compile", false, false, || {
                steps::compile(root, &cfg, output)
            })?;
            if !success {
                failures += 1;
            }
        } else {
            output.println("Skipping compile (generate disabled)");
        }
    }

    let _ = steps::run_step(output, "Report", true, true, || steps::report(root, output));

    // Summary
    let status_path = root.join(OAV_DIR).join("status.tsv");
    let entries = steps::load_status_entries(&status_path).unwrap_or_default();
    let passed = entries.iter().filter(|e| e.status == "ok").count();
    let failed = entries.iter().filter(|e| e.status == "fail").count();

    if output.json {
        let spec_display = cfg.spec.as_deref().unwrap_or("");
        let report =
            json_report::build_validate_report(root, spec_display, cfg.mode.as_str(), &entries);
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("failed to serialize report")
        );
        if failures > 0 {
            std::process::exit(EXIT_VALIDATION_FAILURE);
        }
        return Ok(());
    }

    output.print_summary(passed, failed);

    output.println("");
    output.println_ignore_quiet(&format!(
        "Dashboard: {}",
        root.join(OAV_DIR)
            .join("reports")
            .join("dashboard.html")
            .display()
    ));

    if failures > 0 {
        output.print_error("Validation failed. See dashboard for details.");
        std::process::exit(EXIT_VALIDATION_FAILURE);
    }

    Ok(())
}

fn cmd_config(root: &Path, output: &Output, command: Option<ConfigCommand>) -> Result<()> {
    match command.unwrap_or(ConfigCommand::Print) {
        ConfigCommand::Get { key } => {
            let cfg = config::load(root)?;
            if output.json {
                let value = config::get_json_value(&cfg, &key)?;
                let cv = json_report::ConfigValue { key, value };
                println!(
                    "{}",
                    serde_json::to_string_pretty(&cv).expect("failed to serialize config value")
                );
            } else {
                config::print_value(&cfg, &key)?;
            }
        }
        ConfigCommand::Set { key, value } => {
            let mut cfg = config::load(root)?;
            config::set_value(&mut cfg, &key, value)?;
            config::write(root, &cfg)?;
            output.println(&format!("Updated {}", root.join(CONFIG_FILE).display()));
        }
        ConfigCommand::Validate => {
            let cfg = config::load(root)?;
            config::validate(&cfg)?;
            if output.json {
                println!(r#"{{"valid":true}}"#);
            } else {
                output.println("Config is valid.");
            }
        }
        ConfigCommand::ListGenerators => {
            if output.json {
                let list = json_report::GeneratorList {
                    server: generators::server_names()
                        .into_iter()
                        .map(String::from)
                        .collect(),
                    client: generators::client_names()
                        .into_iter()
                        .map(String::from)
                        .collect(),
                };
                println!(
                    "{}",
                    serde_json::to_string_pretty(&list)
                        .expect("failed to serialize generator list")
                );
            } else {
                println!("Server generators:");
                for g in generators::SERVER_GENERATORS {
                    println!("  {}", g.name);
                }
                println!();
                println!("Client generators:");
                for g in generators::CLIENT_GENERATORS {
                    println!("  {}", g.name);
                }
            }
        }
        ConfigCommand::Edit => {
            let path = root.join(CONFIG_FILE);
            if !path.exists() {
                config::write(root, &Config::default())?;
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
            let cfg = config::load(root)?;
            if output.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&cfg).expect("failed to serialize config")
                );
            } else {
                let yaml = serde_yaml::to_string(&cfg).context("Failed to serialize config")?;
                print!("{yaml}");
            }
        }
        ConfigCommand::Ignore => {
            util::ensure_gitignore(root, true)?;
            let mut cfg = config::load(root)?;
            cfg.manage_gitignore = true;
            config::write(root, &cfg)?;
            output.println("Added .oavc to .gitignore and enabled automatic gitignore management.");
        }
        ConfigCommand::Unignore => {
            util::remove_gitignore_entries(root, &[".oavc"])?;
            let mut cfg = config::load(root)?;
            cfg.manage_gitignore = false;
            config::write(root, &cfg)?;
            output.println(
                "Removed .oavc from .gitignore and disabled automatic gitignore management.",
            );
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
