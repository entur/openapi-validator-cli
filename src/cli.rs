use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(name = "openapi-validator", version, about = "OpenAPI Validator CLI")]
pub struct Cli {
    #[arg(short, long, global = true, conflicts_with = "quiet")]
    pub verbose: bool,
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,
    #[command(subcommand)]
    pub command: Commands,
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
pub enum ConfigCommand {
    Get { key: ConfigKey },
    Set { key: ConfigKey, value: String },
    Edit,
    Print,
    Ignore,
    Unignore,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum ConfigKey {
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
