use anyhow::{Context, Result, bail};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

use crate::cli::{Linter, Mode};
use crate::custom::CustomGeneratorDef;
use crate::generators;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Jobs {
    Auto,
    Fixed(usize), // guaranteed >= 1
}

impl Serialize for Jobs {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Jobs::Auto => serializer.serialize_str("auto"),
            Jobs::Fixed(n) => serializer.serialize_u64(*n as u64),
        }
    }
}

impl<'de> Deserialize<'de> for Jobs {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct JobsVisitor;

        impl<'de> Visitor<'de> for JobsVisitor {
            type Value = Jobs;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("\"auto\" or a positive integer")
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Jobs, E> {
                if value == 0 {
                    return Err(E::custom("jobs must be \"auto\" or a positive integer"));
                }
                Ok(Jobs::Fixed(value as usize))
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Jobs, E> {
                if value <= 0 {
                    return Err(E::custom("jobs must be \"auto\" or a positive integer"));
                }
                Ok(Jobs::Fixed(value as usize))
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Jobs, E> {
                if value.eq_ignore_ascii_case("auto") {
                    Ok(Jobs::Auto)
                } else {
                    Err(E::custom("jobs must be \"auto\" or a positive integer"))
                }
            }
        }

        deserializer.deserialize_any(JobsVisitor)
    }
}

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
    pub linter: Linter,
    pub spectral_image: String,
    pub spectral_ruleset: String,
    pub spectral_fail_severity: String,
    pub manage_gitignore: bool,
    pub custom_generators_dir: Option<String>,
    pub docker_timeout: u64,
    pub search_depth: usize,
    pub jobs: Jobs,
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
            linter: Linter::Spectral,
            spectral_image: "stoplight/spectral:6".to_string(),
            spectral_ruleset:
                "https://raw.githubusercontent.com/entur/api-guidelines/refs/tags/v2/.spectral.yml"
                    .to_string(),
            spectral_fail_severity: "error".to_string(),
            manage_gitignore: true,
            custom_generators_dir: None,
            docker_timeout: 300,
            search_depth: 4,
            jobs: Jobs::Auto,
        }
    }
}

pub fn validate(config: &Config, custom: &[CustomGeneratorDef]) -> Result<()> {
    if config.docker_timeout == 0 {
        bail!("docker_timeout must be greater than 0");
    }
    if config.search_depth == 0 {
        bail!("search_depth must be greater than 0");
    }
    if let Jobs::Fixed(0) = config.jobs {
        bail!("jobs must be \"auto\" or a positive integer");
    }
    let server_owned = generators::all_server_names(custom);
    let client_owned = generators::all_client_names(custom);
    let all_server: Vec<&str> = server_owned.iter().map(|s| s.as_str()).collect();
    let all_client: Vec<&str> = client_owned.iter().map(|s| s.as_str()).collect();
    validate_generators("server", &config.server_generators, &all_server)?;
    validate_generators("client", &config.client_generators, &all_client)?;
    Ok(())
}

pub fn resolve_jobs(jobs: Jobs) -> usize {
    match jobs {
        Jobs::Fixed(n) => n,
        Jobs::Auto => std::thread::available_parallelism()
            .map(|n| n.get().min(4))
            .unwrap_or(1),
    }
}

pub fn load(root: &Path) -> Result<Config> {
    let path = root.join(CONFIG_FILE);
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = fs::read_to_string(&path).context("Failed to read .oavc")?;
    let config = yaml_serde::from_str(&content).context("Failed to parse .oavc")?;
    Ok(config)
}

pub fn write(root: &Path, config: &Config) -> Result<()> {
    let path = root.join(CONFIG_FILE);
    let content = yaml_serde::to_string(config).context("Failed to serialize config")?;
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
        "linter" => println!("{}", config.linter.as_str()),
        "spectral_image" | "spectral-image" => println!("{}", config.spectral_image),
        "spectral_ruleset" | "spectral-ruleset" => println!("{}", config.spectral_ruleset),
        "spectral_fail_severity" | "spectral-fail-severity" => {
            println!("{}", config.spectral_fail_severity)
        }
        "manage_gitignore" | "manage-gitignore" => println!("{}", config.manage_gitignore),
        "custom_generators_dir" | "custom-generators-dir" => {
            if let Some(dir) = &config.custom_generators_dir {
                println!("{dir}");
            }
        }
        "docker_timeout" | "docker-timeout" => println!("{}", config.docker_timeout),
        "search_depth" | "search-depth" => println!("{}", config.search_depth),
        "jobs" => match config.jobs {
            Jobs::Auto => println!("auto"),
            Jobs::Fixed(n) => println!("{n}"),
        },
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
    let yaml = yaml_serde::to_string(value).context("Failed to serialize value")?;
    // Remove trailing newline and print inline
    print!("{}", yaml.trim_end());
    println!();
    Ok(())
}

pub fn get_json_value(config: &Config, key: &str) -> Result<serde_json::Value> {
    let (base, subkey) = parse_key(key);

    let value = match base {
        "spec" => serde_json::to_value(&config.spec)?,
        "mode" => serde_json::Value::String(config.mode.as_str().to_string()),
        "lint" => serde_json::Value::Bool(config.lint),
        "generate" => serde_json::Value::Bool(config.generate),
        "compile" => serde_json::Value::Bool(config.compile),
        "server_generators" | "server-generators" => {
            serde_json::to_value(&config.server_generators)?
        }
        "client_generators" | "client-generators" => {
            serde_json::to_value(&config.client_generators)?
        }
        "generator_overrides" | "generator-overrides" => {
            if let Some(subkey) = subkey {
                match config.generator_overrides.get(subkey) {
                    Some(v) => serde_json::Value::String(v.clone()),
                    None => serde_json::Value::Null,
                }
            } else {
                serde_json::to_value(&config.generator_overrides)?
            }
        }
        "generator_image" | "generator-image" => {
            serde_json::Value::String(config.generator_image.clone())
        }
        "redocly_image" | "redocly-image" => {
            serde_json::Value::String(config.redocly_image.clone())
        }
        "linter" => serde_json::Value::String(config.linter.as_str().to_string()),
        "spectral_image" | "spectral-image" => {
            serde_json::Value::String(config.spectral_image.clone())
        }
        "spectral_ruleset" | "spectral-ruleset" => {
            serde_json::Value::String(config.spectral_ruleset.clone())
        }
        "spectral_fail_severity" | "spectral-fail-severity" => {
            serde_json::Value::String(config.spectral_fail_severity.clone())
        }
        "manage_gitignore" | "manage-gitignore" => serde_json::Value::Bool(config.manage_gitignore),
        "custom_generators_dir" | "custom-generators-dir" => {
            serde_json::to_value(&config.custom_generators_dir)?
        }
        "docker_timeout" | "docker-timeout" => serde_json::to_value(config.docker_timeout)?,
        "search_depth" | "search-depth" => serde_json::to_value(config.search_depth)?,
        "jobs" => match config.jobs {
            Jobs::Auto => serde_json::Value::String("auto".to_string()),
            Jobs::Fixed(n) => serde_json::to_value(n)?,
        },
        _ => bail!("Unknown config key: {key}"),
    };
    Ok(value)
}

pub fn set_value(
    config: &mut Config,
    key: &str,
    value: String,
    custom: &[CustomGeneratorDef],
) -> Result<()> {
    let (base, subkey) = parse_key(key);

    match base {
        "spec" => {
            validate_not_blank("spec", &value)?;
            config.spec = Some(value.trim().to_string());
        }
        "mode" => config.mode = parse_mode(&value)?,
        "lint" => config.lint = parse_bool(&value)?,
        "generate" => config.generate = parse_bool(&value)?,
        "compile" => config.compile = parse_bool(&value)?,
        "server_generators" | "server-generators" => {
            let gens: Vec<String> = parse_yaml_list(&value)
                .context("Invalid YAML list for server_generators (example: [spring, kotlin])")?
                .into_iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let all = generators::all_server_names(custom);
            let refs: Vec<&str> = all.iter().map(|s| s.as_str()).collect();
            validate_generators("server", &gens, &refs)?;
            config.server_generators = gens;
        }
        "client_generators" | "client-generators" => {
            let gens: Vec<String> = parse_yaml_list(&value)
                .context("Invalid YAML list for client_generators (example: [typescript, swift])")?
                .into_iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let all = generators::all_client_names(custom);
            let refs: Vec<&str> = all.iter().map(|s| s.as_str()).collect();
            validate_generators("client", &gens, &refs)?;
            config.client_generators = gens;
        }
        "generator_overrides" | "generator-overrides" => {
            if let Some(subkey) = subkey {
                let subkey = subkey.trim();
                let trimmed = value.trim().to_string();
                validate_not_blank("generator_overrides key", subkey)?;
                if trimmed.is_empty() {
                    config.generator_overrides.remove(subkey);
                } else {
                    config
                        .generator_overrides
                        .insert(subkey.to_string(), trimmed);
                }
            } else {
                config.generator_overrides = parse_yaml_map(&value).context(
                    "Invalid YAML map for generator_overrides (example: {spring: ./path.yaml})",
                )?;
            }
        }
        "generator_image" | "generator-image" => {
            validate_not_blank("generator_image", &value)?;
            config.generator_image = value.trim().to_string();
        }
        "redocly_image" | "redocly-image" => {
            validate_not_blank("redocly_image", &value)?;
            config.redocly_image = value.trim().to_string();
        }
        "linter" => config.linter = parse_linter(&value)?,
        "spectral_image" | "spectral-image" => {
            validate_not_blank("spectral_image", &value)?;
            config.spectral_image = value.trim().to_string();
        }
        "spectral_ruleset" | "spectral-ruleset" => {
            validate_not_blank("spectral_ruleset", &value)?;
            config.spectral_ruleset = value.trim().to_string();
        }
        "spectral_fail_severity" | "spectral-fail-severity" => {
            config.spectral_fail_severity = parse_fail_severity(&value)?;
        }
        "manage_gitignore" | "manage-gitignore" => config.manage_gitignore = parse_bool(&value)?,
        "custom_generators_dir" | "custom-generators-dir" => {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                config.custom_generators_dir = None;
            } else {
                config.custom_generators_dir = Some(trimmed);
            }
        }
        "docker_timeout" | "docker-timeout" => {
            let secs: u64 = value.trim().parse().map_err(|_| {
                anyhow::anyhow!("Invalid docker_timeout: {value} (expected positive integer)")
            })?;
            if secs == 0 {
                bail!("docker_timeout must be greater than 0");
            }
            config.docker_timeout = secs;
        }
        "search_depth" | "search-depth" => {
            let depth: usize = value.trim().parse().map_err(|_| {
                anyhow::anyhow!("Invalid search_depth: {value} (expected positive integer)")
            })?;
            if depth == 0 {
                bail!("search_depth must be greater than 0");
            }
            config.search_depth = depth;
        }
        "jobs" => {
            let trimmed = value.trim();
            if trimmed.eq_ignore_ascii_case("auto") {
                config.jobs = Jobs::Auto;
            } else {
                let n: usize = trimmed.parse().map_err(|_| {
                    anyhow::anyhow!(
                        "Invalid jobs: {value} (expected \"auto\" or a positive integer)"
                    )
                })?;
                if n == 0 {
                    bail!("jobs must be \"auto\" or a positive integer");
                }
                config.jobs = Jobs::Fixed(n);
            }
        }
        _ => bail!("Unknown config key: {key}"),
    }
    Ok(())
}

fn validate_not_blank(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{field} cannot be blank");
    }
    Ok(())
}

pub fn validate_generators(scope: &str, generators: &[String], supported: &[&str]) -> Result<()> {
    for generator in generators {
        let name = generator.trim();
        if name.is_empty() {
            continue;
        }
        if !supported.contains(&name) {
            bail!(
                "Unsupported {scope} generator: '{name}'. Valid options: {}",
                supported.join(", ")
            );
        }
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

fn parse_linter(raw: &str) -> Result<Linter> {
    match raw.trim().to_lowercase().as_str() {
        "spectral" => Ok(Linter::Spectral),
        "redocly" => Ok(Linter::Redocly),
        "none" => Ok(Linter::None),
        _ => bail!("Invalid linter: {raw} (expected spectral, redocly, or none)"),
    }
}

fn parse_fail_severity(raw: &str) -> Result<String> {
    let trimmed = raw.trim().to_lowercase();
    match trimmed.as_str() {
        "error" | "warn" | "info" | "hint" => Ok(trimmed),
        _ => bail!("Invalid fail severity: {raw} (expected error, warn, info, or hint)"),
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
    yaml_serde::from_str(raw).context("Failed to parse as YAML list")
}

fn parse_yaml_map(raw: &str) -> Result<HashMap<String, String>> {
    if raw.trim().is_empty() {
        return Ok(HashMap::new());
    }
    yaml_serde::from_str(raw).context("Failed to parse as YAML map")
}
