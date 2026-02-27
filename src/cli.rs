use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Human,
    Json,
}

#[derive(Parser, Debug)]
#[command(name = "oav", version, about = "OpenAPI Validator CLI")]
pub struct Cli {
    #[arg(short, long, global = true, conflicts_with_all = ["quiet", "output"])]
    pub verbose: bool,
    #[arg(short, long, global = true, conflicts_with_all = ["verbose", "output"])]
    pub quiet: bool,
    #[arg(long, global = true, default_value = "auto")]
    pub color: ColorMode,
    #[arg(long, global = true, default_value = "human", conflicts_with_all = ["verbose", "quiet"])]
    pub output: OutputFormat,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
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
        #[arg(long)]
        search_depth: Option<usize>,
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
        #[arg(long)]
        linter: Option<Linter>,
        #[arg(long)]
        ruleset: Option<String>,
        #[arg(long)]
        docker_timeout: Option<u64>,
        #[arg(long)]
        search_depth: Option<usize>,
        #[arg(short = 'j', long)]
        jobs: Option<String>,
    },
    Config {
        #[command(subcommand)]
        command: Option<ConfigCommand>,
    },
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
    Get {
        key: String,
    },
    /// Set a config value. Use dot notation for map keys (e.g., generator_overrides.spring)
    Set {
        key: String,
        value: String,
    },
    Edit,
    Print,
    /// Validate the current config file
    Validate,
    /// List all supported generators
    ListGenerators,
    Ignore,
    Unignore,
}

#[derive(ValueEnum, Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Server,
    Client,
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
    Spectral,
    Redocly,
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
