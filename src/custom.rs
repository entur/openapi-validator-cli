use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::fs;
use std::path::Path;

use crate::generators;

#[derive(Debug, Clone, Deserialize)]
pub struct CustomGeneratorDef {
    pub name: String,
    pub scope: String,
    pub generate: GenerateBlock,
    pub compile: Option<CompileBlock>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateBlock {
    pub image: String,
    pub command: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompileBlock {
    pub image: String,
    pub command: String,
}

pub fn load(root: &Path, dir: &str) -> Result<Vec<CustomGeneratorDef>> {
    let custom_dir = root.join(dir);
    if !custom_dir.is_dir() {
        bail!(
            "custom_generators_dir '{}' does not exist or is not a directory",
            custom_dir.display()
        );
    }

    let mut defs = Vec::new();

    let mut entries: Vec<_> = fs::read_dir(&custom_dir)
        .with_context(|| format!("Failed to read {}", custom_dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("Failed to iterate {}", custom_dir.display()))?;
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str());
        if ext != Some("yaml") && ext != Some("yml") {
            continue;
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let def: CustomGeneratorDef = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;

        validate_def(&def, &path)?;
        defs.push(def);
    }

    check_collisions(&defs)?;
    Ok(defs)
}

fn validate_def(def: &CustomGeneratorDef, path: &Path) -> Result<()> {
    if def.name.trim().is_empty() {
        bail!("Custom generator in {} has an empty name", path.display());
    }
    match def.scope.as_str() {
        "server" | "client" => {}
        other => bail!(
            "Custom generator '{}' has invalid scope '{}' (expected server or client)",
            def.name,
            other
        ),
    }
    if def.generate.image.trim().is_empty() {
        bail!(
            "Custom generator '{}' has an empty generate.image",
            def.name
        );
    }
    if def.generate.command.trim().is_empty() {
        bail!(
            "Custom generator '{}' has an empty generate.command",
            def.name
        );
    }
    if let Some(compile) = &def.compile {
        if compile.image.trim().is_empty() {
            bail!("Custom generator '{}' has an empty compile.image", def.name);
        }
        if compile.command.trim().is_empty() {
            bail!(
                "Custom generator '{}' has an empty compile.command",
                def.name
            );
        }
    }
    Ok(())
}

fn check_collisions(defs: &[CustomGeneratorDef]) -> Result<()> {
    let builtin_server = generators::server_names();
    let builtin_client = generators::client_names();

    let mut seen = std::collections::HashSet::new();
    for def in defs {
        let builtin_list = match def.scope.as_str() {
            "server" => &builtin_server,
            "client" => &builtin_client,
            _ => continue,
        };
        if builtin_list.contains(&def.name.as_str()) {
            bail!(
                "Custom generator '{}' collides with built-in {} generator",
                def.name,
                def.scope
            );
        }
        if !seen.insert((&def.name, &def.scope)) {
            bail!(
                "Duplicate custom generator name '{}' for scope '{}'",
                def.name,
                def.scope
            );
        }
    }
    Ok(())
}

pub fn server_names(defs: &[CustomGeneratorDef]) -> Vec<String> {
    defs.iter()
        .filter(|d| d.scope == "server")
        .map(|d| d.name.clone())
        .collect()
}

pub fn client_names(defs: &[CustomGeneratorDef]) -> Vec<String> {
    defs.iter()
        .filter(|d| d.scope == "client")
        .map(|d| d.name.clone())
        .collect()
}
