mod cli;
mod config;
mod docker;
mod output;
mod steps;
mod util;

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

static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");

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
    mode: Option<cli::Mode>,
    server_generators: Option<Vec<String>>,
    client_generators: Option<Vec<String>>,
    ignore_config: bool,
) -> Result<()> {
    util::ensure_oav_dir(root)?;
    util::ensure_gitignore(root, ignore_config)?;

    let mut cfg = config::load(root)?;
    if let Some(s) = spec {
        cfg.spec = Some(s);
    }
    if cfg.spec.is_none() {
        cfg.spec = util::discover_spec(root)?;
    }
    if let Some(m) = mode {
        cfg.mode = m;
    }
    if let Some(gens) = server_generators {
        cfg.server_generators = gens;
    }
    if let Some(gens) = client_generators {
        cfg.client_generators = gens;
    }

    let spec = cfg.spec.clone().ok_or_else(|| {
        anyhow::anyhow!("No OpenAPI spec found. Pass --spec or set spec in .oavc.")
    })?;
    let spec_path = util::normalize_spec_path(root, &spec)?;
    cfg.spec = Some(spec_path.to_string_lossy().to_string());

    config::write(root, &cfg)?;
    util::extract_assets(root, &ASSETS)?;

    output.println("Initialized OpenAPI Validator.");
    output.println(&format!("Config: {}", root.join(CONFIG_FILE).display()));
    output.println(&format!("Workspace: {}", root.join(OAV_DIR).display()));
    Ok(())
}

fn cmd_validate(
    root: &Path,
    output: &Output,
    spec_override: Option<String>,
    mode_override: Option<cli::Mode>,
    server_generators: Option<Vec<String>>,
    client_generators: Option<Vec<String>>,
    skip_lint: bool,
    skip_generate: bool,
    skip_compile: bool,
) -> Result<()> {
    util::ensure_oav_dir(root)?;
    util::ensure_gitignore(root, false)?;
    util::extract_assets(root, &ASSETS)?;

    let mut cfg = config::load(root)?;
    if let Some(s) = spec_override {
        cfg.spec = Some(s);
    }
    if let Some(m) = mode_override {
        cfg.mode = m;
    }
    if let Some(gens) = server_generators {
        cfg.server_generators = gens;
    }
    if let Some(gens) = client_generators {
        cfg.client_generators = gens;
    }
    if skip_lint {
        cfg.lint = false;
    }
    if skip_generate {
        cfg.generate = false;
    }
    if skip_compile {
        cfg.compile = false;
    }

    let spec = if let Some(s) = cfg.spec.clone() {
        s
    } else if let Some(s) = util::discover_spec(root)? {
        s
    } else {
        bail!("No OpenAPI spec found. Pass --spec or set spec in .oavc.");
    };

    let spec_path = util::normalize_spec_path(root, &spec)?;
    cfg.spec = Some(spec_path.to_string_lossy().to_string());

    if cfg.lint || cfg.generate || cfg.compile {
        docker::ensure_available()?;
    }

    util::prepare_runtime_dirs(root)?;
    config::write(root, &cfg)?;

    let mut failures = 0;

    if cfg.lint {
        let success = steps::run_step(output, "Lint", true, true, || {
            steps::lint(root, &spec_path, &cfg.redocly_image, output)
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

    output.print_summary(passed, failed);

    println!();
    output.println_always(&format!(
        "Dashboard: {}",
        root.join(OAV_DIR)
            .join("reports")
            .join("dashboard.html")
            .display()
    ));

    if failures > 0 {
        output.print_error("Validation failed. See dashboard for details.");
        std::process::exit(1);
    }

    Ok(())
}

fn cmd_config(root: &Path, output: &Output, command: Option<ConfigCommand>) -> Result<()> {
    util::ensure_gitignore(root, false)?;

    match command.unwrap_or(ConfigCommand::Print) {
        ConfigCommand::Get { key } => {
            let cfg = config::load(root)?;
            config::print_value(&cfg, key)?;
        }
        ConfigCommand::Set { key, value } => {
            let mut cfg = config::load(root)?;
            config::set_value(&mut cfg, key, value)?;
            config::write(root, &cfg)?;
            output.println(&format!("Updated {}", root.join(CONFIG_FILE).display()));
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
            let yaml = serde_yaml::to_string(&cfg).context("Failed to serialize config")?;
            print!("{yaml}");
        }
        ConfigCommand::Ignore => {
            util::ensure_gitignore(root, true)?;
            output.println("Added .oavc to .gitignore.");
        }
        ConfigCommand::Unignore => {
            util::remove_gitignore_entries(root, &[".oavc"])?;
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
