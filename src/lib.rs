mod agent;
mod cli;
mod completions;
mod config;
mod custom;
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

use cli::{AgentCommand, Cli, Commands, CompletionsCommand, ConfigCommand};
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
    jobs: Option<String>,
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
            jobs,
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
                jobs,
            },
        ),
        Commands::Config { command } => cmd_config(&root, &output, command),
        Commands::Clean { nuke, yes } => cmd_clean(&root, &output, nuke, yes),
        Commands::Completions { command } => {
            return (
                cli.output,
                match command {
                    CompletionsCommand::Generate { shell } => {
                        completions::generate(shell);
                        Ok(())
                    }
                    CompletionsCommand::Install { shell, yes } => {
                        completions::install(shell, yes, &output)
                    }
                    CompletionsCommand::Uninstall { shell, yes } => {
                        completions::uninstall(shell, yes, &output)
                    }
                },
            );
        }
        Commands::Agent { command } => match command {
            None | Some(AgentCommand::Install { force: false }) => {
                agent::install(&root, false, &output)
            }
            Some(AgentCommand::Install { force: true }) => agent::install(&root, true, &output),
            Some(AgentCommand::Uninstall) => agent::uninstall(&root, &output),
        },
    };

    (cli.output, result)
}

fn cmd_init(root: &Path, output: &Output, args: InitArgs) -> Result<()> {
    let is_fresh = !root.join(CONFIG_FILE).exists();
    let no_flags = args.spec.is_none()
        && args.mode.is_none()
        && args.server_generators.is_none()
        && args.client_generators.is_none()
        && args.search_depth.is_none()
        && !args.ignore_config;
    let interactive = !output.quiet && !output.json;

    if is_fresh && no_flags && interactive {
        return cmd_init_interactive(root, output, args);
    }

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

    let custom_defs = load_custom_defs(root, &cfg)?;

    if let Some(gens) = args.server_generators {
        let gens: Vec<String> = gens
            .iter()
            .map(|g| g.trim().to_string())
            .filter(|g| !g.is_empty())
            .collect();
        let all = generators::all_server_names(&custom_defs);
        let refs: Vec<&str> = all.iter().map(|s| s.as_str()).collect();
        config::validate_generators("server", &gens, &refs)?;
        cfg.server_generators = gens;
    }
    if let Some(gens) = args.client_generators {
        let gens: Vec<String> = gens
            .iter()
            .map(|g| g.trim().to_string())
            .filter(|g| !g.is_empty())
            .collect();
        let all = generators::all_client_names(&custom_defs);
        let refs: Vec<&str> = all.iter().map(|s| s.as_str()).collect();
        config::validate_generators("client", &gens, &refs)?;
        cfg.client_generators = gens;
    }

    let spec = cfg.spec.clone().ok_or_else(|| {
        anyhow::anyhow!("No OpenAPI spec found. Pass --spec or set spec in .oavc.")
    })?;
    let spec_path = util::normalize_spec_path(root, &spec)?;
    cfg.spec = Some(spec_path.to_string_lossy().to_string());

    config::validate(&cfg, &custom_defs)?;
    config::write(root, &cfg)?;
    util::extract_assets(root, &ASSETS)?;

    output.print_success("Initialized OpenAPI Validator.");
    output.print_detail("Config", &root.join(CONFIG_FILE).display().to_string());
    output.print_detail("Workspace", &root.join(OAV_DIR).display().to_string());
    print_agent_hint(root, output);
    Ok(())
}

fn cmd_init_interactive(root: &Path, output: &Output, args: InitArgs) -> Result<()> {
    let term = console::Term::stderr();
    let theme = dialoguer::theme::ColorfulTheme::default();
    let cancelled = || anyhow::anyhow!("Setup cancelled.");

    let mut cfg = config::load(root)?;
    util::ensure_oav_dir(root)?;
    if cfg.manage_gitignore {
        util::add_gitignore_entries(root, &[".oav/"])?;
        if args.ignore_config {
            util::add_gitignore_entries(root, &[".oavc"])?;
        }
    }

    let custom_defs = load_custom_defs(root, &cfg)?;

    // 1. Spec discovery
    let spec = match util::discover_spec(root, false, cfg.search_depth)? {
        Some(s) => s,
        None => {
            let input: String = dialoguer::Input::with_theme(&theme)
                .with_prompt("No OpenAPI spec found — enter path")
                .interact_on(&term)?;
            let trimmed = input.trim().to_string();
            if trimmed.is_empty() {
                bail!("Spec path cannot be blank");
            }
            trimmed
        }
    };
    let spec_path = util::normalize_spec_path(root, &spec)?;
    cfg.spec = Some(spec_path.to_string_lossy().to_string());

    // 2. Mode selection
    let mode_items = ["server", "client", "both"];
    let mode_idx = dialoguer::Select::with_theme(&theme)
        .with_prompt("Validation mode")
        .items(mode_items)
        .default(0)
        .interact_on_opt(&term)?
        .ok_or_else(cancelled)?;
    cfg.mode = match mode_idx {
        0 => cli::Mode::Server,
        1 => cli::Mode::Client,
        _ => cli::Mode::Both,
    };

    // 3. Generator selection (built-in + custom)
    if matches!(cfg.mode, cli::Mode::Server | cli::Mode::Both) {
        let names = generators::all_server_names(&custom_defs);
        let display: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let selections = dialoguer::MultiSelect::with_theme(&theme)
            .with_prompt(
                "Server generators (space to toggle, enter to confirm, or leave empty for all)",
            )
            .items(&display)
            .interact_on_opt(&term)?
            .ok_or_else(cancelled)?;
        if !selections.is_empty() {
            cfg.server_generators = selections.iter().map(|&i| names[i].clone()).collect();
        }
    }
    if matches!(cfg.mode, cli::Mode::Client | cli::Mode::Both) {
        let names = generators::all_client_names(&custom_defs);
        let display: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let selections = dialoguer::MultiSelect::with_theme(&theme)
            .with_prompt(
                "Client generators (space to toggle, enter to confirm, or leave empty for all)",
            )
            .items(&display)
            .interact_on_opt(&term)?
            .ok_or_else(cancelled)?;
        if !selections.is_empty() {
            cfg.client_generators = selections.iter().map(|&i| names[i].clone()).collect();
        }
    }

    // 4. Linter selection
    let linter_items = ["spectral", "redocly", "none"];
    let linter_idx = dialoguer::Select::with_theme(&theme)
        .with_prompt("Linter")
        .items(linter_items)
        .default(0)
        .interact_on_opt(&term)?
        .ok_or_else(cancelled)?;
    cfg.linter = match linter_idx {
        0 => cli::Linter::Spectral,
        1 => cli::Linter::Redocly,
        _ => cli::Linter::None,
    };

    // 5. Write config and finish
    config::validate(&cfg, &custom_defs)?;
    config::write(root, &cfg)?;
    util::extract_assets(root, &ASSETS)?;

    output.print_success("Initialized OpenAPI Validator.");
    output.print_detail("Config", &root.join(CONFIG_FILE).display().to_string());
    output.print_detail("Workspace", &root.join(OAV_DIR).display().to_string());
    print_agent_hint(root, output);
    Ok(())
}

fn cmd_validate(root: &Path, output: &Output, args: ValidateArgs) -> Result<()> {
    let mut cfg = config::load(root)?;
    util::ensure_oav_dir(root)?;
    if cfg.manage_gitignore {
        util::add_gitignore_entries(root, &[".oav/"])?;
    }
    util::extract_assets(root, &ASSETS)?;

    let custom_defs = load_custom_defs(root, &cfg)?;

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
    if let Some(j) = args.jobs {
        cfg.jobs = parse_jobs_arg(&j)?;
    }
    let resolved_jobs = config::resolve_jobs(cfg.jobs);
    if output.verbose && resolved_jobs > 1 {
        output.print_warning("--verbose forces sequential execution (--jobs 1)");
        cfg.jobs = config::Jobs::Fixed(1);
    } else {
        cfg.jobs = config::Jobs::Fixed(resolved_jobs);
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
        let all = generators::all_server_names(&custom_defs);
        let refs: Vec<&str> = all.iter().map(|s| s.as_str()).collect();
        config::validate_generators("server", &gens, &refs)?;
        cfg.server_generators = gens;
    }
    if let Some(gens) = args.client_generators {
        let gens: Vec<String> = gens
            .iter()
            .map(|g| g.trim().to_string())
            .filter(|g| !g.is_empty())
            .collect();
        let all = generators::all_client_names(&custom_defs);
        let refs: Vec<&str> = all.iter().map(|s| s.as_str()).collect();
        config::validate_generators("client", &gens, &refs)?;
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

    config::validate(&cfg, &custom_defs)?;
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
            steps::generate(root, &spec_path, &cfg, output, &custom_defs)
        })?;
        if !success {
            failures += 1;
        }
    }

    if cfg.compile {
        if cfg.generate {
            output.phase_header("Compile");
            let success = steps::run_step(output, "Compile", false, false, || {
                steps::compile(root, &cfg, output, &custom_defs)
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
    output.print_detail_ignore_quiet(
        "Dashboard",
        &root
            .join(OAV_DIR)
            .join("reports")
            .join("dashboard.html")
            .display()
            .to_string(),
    );

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
            // Best-effort: if custom dir is broken, generator-list validation
            // falls back to built-ins only. This keeps set/edit usable for recovery.
            let custom_defs = load_custom_defs(root, &cfg).unwrap_or_default();
            config::set_value(&mut cfg, &key, value, &custom_defs)?;
            config::write(root, &cfg)?;
            output.print_success(&format!("Updated {}", root.join(CONFIG_FILE).display()));
        }
        ConfigCommand::Validate => {
            let cfg = config::load(root)?;
            let custom_defs = load_custom_defs(root, &cfg)?;
            config::validate(&cfg, &custom_defs)?;
            if output.json {
                println!(r#"{{"valid":true}}"#);
            } else {
                output.print_success("Config is valid.");
            }
        }
        ConfigCommand::ListGenerators => {
            let cfg = config::load(root)?;
            let custom_defs = load_custom_defs(root, &cfg)?;
            let custom_info: Vec<json_report::CustomGeneratorInfo> = custom_defs
                .iter()
                .map(|d| json_report::CustomGeneratorInfo {
                    name: d.name.clone(),
                    scope: d.scope.clone(),
                    has_compile: d.compile.is_some(),
                })
                .collect();

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
                    custom: custom_info,
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
                if !custom_defs.is_empty() {
                    println!();
                    println!("Custom generators:");
                    for d in &custom_defs {
                        let compile_marker = if d.compile.is_some() {
                            ""
                        } else {
                            " (no compile)"
                        };
                        println!("  {} [{}]{}", d.name, d.scope, compile_marker);
                    }
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
                let yaml = yaml_serde::to_string(&cfg).context("Failed to serialize config")?;
                print!("{yaml}");
            }
        }
        ConfigCommand::Ignore => {
            util::ensure_gitignore(root, true)?;
            let mut cfg = config::load(root)?;
            cfg.manage_gitignore = true;
            config::write(root, &cfg)?;
            output.print_success(
                "Added .oavc to .gitignore and enabled automatic gitignore management.",
            );
        }
        ConfigCommand::Unignore => {
            util::remove_gitignore_entries(root, &[".oavc"])?;
            let mut cfg = config::load(root)?;
            cfg.manage_gitignore = false;
            config::write(root, &cfg)?;
            output.print_success(
                "Removed .oavc from .gitignore and disabled automatic gitignore management.",
            );
        }
    }
    Ok(())
}

fn parse_jobs_arg(raw: &str) -> Result<config::Jobs> {
    let trimmed = raw.trim();
    if trimmed.eq_ignore_ascii_case("auto") {
        return Ok(config::Jobs::Auto);
    }
    let n: usize = trimmed.parse().map_err(|_| {
        anyhow::anyhow!("Invalid --jobs value: {raw} (expected \"auto\" or a positive integer)")
    })?;
    if n == 0 {
        bail!("--jobs must be \"auto\" or a positive integer");
    }
    Ok(config::Jobs::Fixed(n))
}

fn load_custom_defs(root: &Path, cfg: &Config) -> Result<Vec<custom::CustomGeneratorDef>> {
    match &cfg.custom_generators_dir {
        Some(dir) => custom::load(root, dir),
        None => Ok(Vec::new()),
    }
}

fn print_agent_hint(root: &Path, output: &Output) {
    if !agent::is_installed(root) {
        output.println("");
        output.println("  Tip: Run `oav agent install` to set up AI agent integration.");
    }
}

fn cmd_clean(root: &Path, output: &Output, nuke: bool, yes: bool) -> Result<()> {
    if !nuke {
        let path = root.join(OAV_DIR);
        if path.exists() {
            warn_modified_configs(root, output);
            fs::remove_dir_all(&path).context("Failed to remove .oav directory")?;
            output.print_success(&format!("Removed {}", path.display()));
        } else {
            output.println("No .oav directory found.");
        }
        return Ok(());
    }

    // --nuke: remove everything oav created
    let oav_path = root.join(OAV_DIR);
    let config_path = root.join(CONFIG_FILE);
    let has_oav = oav_path.exists();
    let has_config = config_path.exists();
    let gitignore_path = root.join(".gitignore");
    let has_gitignore_entries = gitignore_path.exists() && {
        let content = fs::read_to_string(&gitignore_path)
            .with_context(|| format!("Failed to read {}", gitignore_path.display()))?;
        content
            .lines()
            .any(|l| l.trim() == ".oav/" || l.trim() == ".oavc")
    };

    if !has_oav && !has_config && !has_gitignore_entries {
        output.println("Nothing to clean.");
        return Ok(());
    }

    if has_oav {
        warn_modified_configs(root, output);
    }

    if !yes {
        let term = console::Term::stderr();
        if !term.is_term() {
            bail!("--nuke requires confirmation; rerun with --yes in non-interactive environments");
        }
        let confirmed = dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(
                "This will remove .oav/, .oavc, and oav entries from .gitignore. Continue?",
            )
            .default(false)
            .interact_on(&term)?;
        if !confirmed {
            output.println("Aborted.");
            return Ok(());
        }
    }

    let mut removed = Vec::new();

    if has_oav {
        fs::remove_dir_all(&oav_path).context("Failed to remove .oav directory")?;
        removed.push(".oav/");
    }
    if has_config {
        fs::remove_file(&config_path).context("Failed to remove .oavc")?;
        removed.push(".oavc");
    }
    if has_gitignore_entries {
        util::remove_gitignore_entries(root, &[".oav/", ".oavc"])?;
        // Clean up the .gitignore file itself if oav entries were the only content
        let gi = fs::read_to_string(&gitignore_path)
            .with_context(|| format!("Failed to read {}", gitignore_path.display()))?;
        if gi.trim().is_empty() {
            fs::remove_file(&gitignore_path).context("Failed to remove empty .gitignore")?;
            removed.push(".gitignore");
        } else {
            removed.push(".gitignore entries");
        }
    }

    output.print_success(&format!("Removed {}", removed.join(", ")));
    Ok(())
}

fn warn_modified_configs(root: &Path, output: &Output) {
    let modified = util::find_modified_generator_configs(root, &ASSETS);
    if modified.is_empty() {
        return;
    }
    output.print_warning("The following generator configs have been modified from defaults:");
    for path in &modified {
        output.println(&format!("  .oav/{path}"));
    }
    output.println("");
    output.println("  Use generator_overrides in .oavc to preserve configs outside .oav/.");
    output.println("");
}
