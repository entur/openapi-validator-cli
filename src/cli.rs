use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

/// Output format
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable output with colors and progress bars
    Human,
    /// JSON output for scripting and CI
    Json,
}

#[derive(Parser, Debug)]
#[command(name = "oav", version, about = "OpenAPI Validator CLI")]
pub struct Cli {
    /// Show full tool output (conflicts with -q and --output)
    #[arg(short, long, global = true, conflicts_with_all = ["quiet", "output"])]
    pub verbose: bool,
    /// Minimal output (conflicts with -v and --output)
    #[arg(short, long, global = true, conflicts_with_all = ["verbose", "output"])]
    pub quiet: bool,
    /// When to use colors: auto, always, never
    #[arg(long, global = true, default_value = "auto")]
    pub color: ColorMode,
    /// Output format: human or json
    #[arg(long, global = true, default_value = "human", conflicts_with_all = ["verbose", "quiet"])]
    pub output: OutputFormat,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum ColorMode {
    /// Detect from terminal (default)
    Auto,
    /// Force color output
    Always,
    /// Disable color output
    Never,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize project: scaffold .oav/ and .oavc config
    Init {
        /// Path to OpenAPI spec (launches interactive wizard if omitted)
        #[arg(long)]
        spec: Option<String>,
        /// Validation mode: server, client, or both
        #[arg(long)]
        mode: Option<Mode>,
        /// Comma-separated server generators to enable
        #[arg(long, value_delimiter = ',')]
        server_generators: Option<Vec<String>>,
        /// Comma-separated client generators to enable
        #[arg(long, value_delimiter = ',')]
        client_generators: Option<Vec<String>>,
        /// Add .oavc to .gitignore
        #[arg(long)]
        ignore_config: bool,
        /// Max directory depth when searching for spec files
        #[arg(long)]
        search_depth: Option<usize>,
    },
    /// Run lint, generate, and compile steps
    Validate {
        /// Path or URL to OpenAPI spec (overrides .oavc)
        #[arg(long)]
        spec: Option<String>,
        /// Validation mode: server, client, or both
        #[arg(long)]
        mode: Option<Mode>,
        /// Comma-separated server generators (overrides .oavc)
        #[arg(long, value_delimiter = ',')]
        server_generators: Option<Vec<String>>,
        /// Comma-separated client generators (overrides .oavc)
        #[arg(long, value_delimiter = ',')]
        client_generators: Option<Vec<String>>,
        /// Skip the lint step
        #[arg(long)]
        skip_lint: bool,
        /// Skip code generation
        #[arg(long)]
        skip_generate: bool,
        /// Skip compilation of generated code
        #[arg(long)]
        skip_compile: bool,
        /// Linter to use: spectral, redocly, or none
        #[arg(long)]
        linter: Option<Linter>,
        /// Custom Spectral ruleset URL or path
        #[arg(long)]
        ruleset: Option<String>,
        /// Docker command timeout in seconds
        #[arg(long)]
        docker_timeout: Option<u64>,
        /// Max directory depth when searching for spec files
        #[arg(long)]
        search_depth: Option<usize>,
        /// Parallel jobs: "auto" or a number (e.g. -j4)
        #[arg(short = 'j', long)]
        jobs: Option<String>,
    },
    /// Read, write, and manage .oavc config
    Config {
        #[command(subcommand)]
        command: Option<ConfigCommand>,
    },
    /// Remove generated artifacts
    Clean {
        /// Remove everything oav created: .oav/, .oavc, and gitignore entries
        #[arg(long)]
        nuke: bool,
        /// Skip confirmation prompt (use with --nuke)
        #[arg(long, short, requires = "nuke")]
        yes: bool,
    },
    /// Generate, install, or uninstall shell completions
    Completions {
        #[command(subcommand)]
        command: CompletionsCommand,
    },
    /// Manage AI agent integration (defaults to install)
    Agent {
        #[command(subcommand)]
        command: Option<AgentCommand>,
    },
}

#[derive(Subcommand, Debug)]
pub enum AgentCommand {
    /// Install the oav skill to .claude/skills/
    Install {
        /// Overwrite existing skill files
        #[arg(long)]
        force: bool,
    },
    /// Remove the installed oav skill
    Uninstall,
}

#[derive(Subcommand, Debug)]
pub enum CompletionsCommand {
    /// Print completion script to stdout
    Generate {
        /// Target shell
        shell: clap_complete::Shell,
    },
    /// Install completions for the current user
    Install {
        /// Override auto-detected shell (bash, zsh, fish, elvish, powershell)
        #[arg(long)]
        shell: Option<clap_complete::Shell>,
        /// Skip confirmation prompts
        #[arg(long, short)]
        yes: bool,
    },
    /// Remove installed completions
    Uninstall {
        /// Override auto-detected shell (bash, zsh, fish, elvish, powershell)
        #[arg(long)]
        shell: Option<clap_complete::Shell>,
        /// Skip confirmation prompts
        #[arg(long, short)]
        yes: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Get a config value. Use dot notation for map keys (e.g., generator_overrides.spring)
    Get { key: String },
    /// Set a config value. Use dot notation for map keys (e.g., generator_overrides.spring)
    Set { key: String, value: String },
    /// Open .oavc in $EDITOR
    Edit,
    /// Print the full config as YAML
    Print,
    /// Validate the current config file
    Validate,
    /// List all supported generators
    ListGenerators,
    /// Add .oavc to .gitignore
    Ignore,
    /// Remove .oavc from .gitignore
    Unignore,
}

#[derive(ValueEnum, Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Validate server generators only
    Server,
    /// Validate client generators only
    Client,
    /// Validate both server and client generators
    Both,
}

impl Mode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Server => "server",
            Mode::Client => "client",
            Mode::Both => "both",
        }
    }
}

#[derive(ValueEnum, Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Linter {
    /// Stoplight Spectral (default)
    Spectral,
    /// Redocly CLI
    Redocly,
    /// Skip linting entirely
    None,
}

impl Linter {
    pub fn as_str(&self) -> &'static str {
        match self {
            Linter::Spectral => "spectral",
            Linter::Redocly => "redocly",
            Linter::None => "none",
        }
    }
}
