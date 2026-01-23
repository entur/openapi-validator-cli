use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::cli::Mode;

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
    pub generator_overrides: HashMap<String, String>,
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
            generator_overrides: HashMap::new(),
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

pub fn print_value(config: &Config, key: &str) -> Result<()> {
    let (base, subkey) = parse_key(key);

    match base {
        "spec" => {
            if let Some(spec) = &config.spec {
                println!("{spec}");
            }
        }
        "mode" => println!("{}", config.mode.as_str()),
        "lint" => println!("{}", config.lint),
        "generate" => println!("{}", config.generate),
        "compile" => println!("{}", config.compile),
        "server_generators" | "server-generators" => {
            print_yaml(&config.server_generators)?;
        }
        "client_generators" | "client-generators" => {
            print_yaml(&config.client_generators)?;
        }
        "generator_overrides" | "generator-overrides" => {
            if let Some(subkey) = subkey {
                if let Some(value) = config.generator_overrides.get(subkey) {
                    println!("{value}");
                }
            } else {
                print_yaml(&config.generator_overrides)?;
            }
        }
        "generator_image" | "generator-image" => println!("{}", config.generator_image),
        "redocly_image" | "redocly-image" => println!("{}", config.redocly_image),
        _ => bail!("Unknown config key: {key}"),
    }
    Ok(())
}

fn parse_key(key: &str) -> (&str, Option<&str>) {
    match key.split_once('.') {
        Some((base, subkey)) => (base, Some(subkey)),
        None => (key, None),
    }
}

fn print_yaml<T: Serialize>(value: &T) -> Result<()> {
    let yaml = serde_yaml::to_string(value).context("Failed to serialize value")?;
    // Remove trailing newline and print inline
    print!("{}", yaml.trim_end());
    println!();
    Ok(())
}

pub fn set_value(config: &mut Config, key: &str, value: String) -> Result<()> {
    let (base, subkey) = parse_key(key);

    match base {
        "spec" => config.spec = Some(value),
        "mode" => config.mode = parse_mode(&value)?,
        "lint" => config.lint = parse_bool(&value)?,
        "generate" => config.generate = parse_bool(&value)?,
        "compile" => config.compile = parse_bool(&value)?,
        "server_generators" | "server-generators" => {
            config.server_generators = parse_yaml_list(&value)
                .context("Invalid YAML list for server_generators (example: [spring, kotlin])")?;
        }
        "client_generators" | "client-generators" => {
            config.client_generators = parse_yaml_list(&value).context(
                "Invalid YAML list for client_generators (example: [typescript, swift])",
            )?;
        }
        "generator_overrides" | "generator-overrides" => {
            if let Some(subkey) = subkey {
                if value.is_empty() {
                    config.generator_overrides.remove(subkey);
                } else {
                    config.generator_overrides.insert(subkey.to_string(), value);
                }
            } else {
                config.generator_overrides = parse_yaml_map(&value).context(
                    "Invalid YAML map for generator_overrides (example: {spring: ./path.yaml})",
                )?;
            }
        }
        "generator_image" | "generator-image" => config.generator_image = value,
        "redocly_image" | "redocly-image" => config.redocly_image = value,
        _ => bail!("Unknown config key: {key}"),
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

fn parse_yaml_list(raw: &str) -> Result<Vec<String>> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_yaml::from_str(raw).context("Failed to parse as YAML list")
}

fn parse_yaml_map(raw: &str) -> Result<HashMap<String, String>> {
    if raw.trim().is_empty() {
        return Ok(HashMap::new());
    }
    serde_yaml::from_str(raw).context("Failed to parse as YAML map")
}
