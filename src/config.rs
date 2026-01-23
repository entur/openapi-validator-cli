use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::cli::{ConfigKey, Mode};

pub const CONFIG_FILE: &str = ".oavc";

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Config {
    pub spec: Option<String>,
    pub mode: Mode,
    pub lint: bool,
    pub generate: bool,
    pub compile: bool,
    pub server_generators: Vec<String>,
    pub client_generators: Vec<String>,
    pub generator_image: String,
    pub redocly_image: String,
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

pub fn load(root: &Path) -> Result<Config> {
    let path = root.join(CONFIG_FILE);
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = fs::read_to_string(&path).context("Failed to read .oavc")?;
    let config = serde_yaml::from_str(&content).context("Failed to parse .oavc")?;
    Ok(config)
}

pub fn write(root: &Path, config: &Config) -> Result<()> {
    let path = root.join(CONFIG_FILE);
    let content = serde_yaml::to_string(config).context("Failed to serialize config")?;
    fs::write(&path, content).context("Failed to write .oavc")?;
    Ok(())
}

pub fn print_value(config: &Config, key: ConfigKey) -> Result<()> {
    match key {
        ConfigKey::Spec => {
            if let Some(spec) = &config.spec {
                println!("{spec}");
            }
        }
        ConfigKey::Mode => println!("{}", config.mode.as_str()),
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

pub fn set_value(config: &mut Config, key: ConfigKey, value: String) -> Result<()> {
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
