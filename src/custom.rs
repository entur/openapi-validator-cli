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

fn is_safe_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-')
}

fn validate_def(def: &CustomGeneratorDef, path: &Path) -> Result<()> {
    if def.name.trim().is_empty() {
        bail!("Custom generator in {} has an empty name", path.display());
    }
    if !is_safe_name(&def.name) {
        bail!(
            "Custom generator '{}' in {} has an invalid name \
             (must match [a-z0-9][a-z0-9._-]*)",
            def.name,
            path.display()
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ── is_safe_name ────────────────────────────────────────────────────

    #[test]
    fn safe_name_valid_cases() {
        for name in ["my-gen", "swagger-ts-api", "gen.v2", "a", "1foo", "a_b"] {
            assert!(is_safe_name(name), "expected '{name}' to be safe");
        }
    }

    #[test]
    fn safe_name_empty() {
        assert!(!is_safe_name(""));
    }

    #[test]
    fn safe_name_uppercase() {
        assert!(!is_safe_name("MyGen"));
    }

    #[test]
    fn safe_name_leading_dash() {
        assert!(!is_safe_name("-foo"));
    }

    #[test]
    fn safe_name_contains_slash() {
        assert!(!is_safe_name("a/b"));
    }

    #[test]
    fn safe_name_space() {
        assert!(!is_safe_name("a b"));
    }

    #[test]
    fn safe_name_dot_dot() {
        // ".." starts with '.', which is not alphanumeric → rejected
        assert!(!is_safe_name(".."));
    }

    // ── load ────────────────────────────────────────────────────────────

    fn valid_yaml(name: &str, scope: &str) -> String {
        format!(
            "name: {name}\nscope: {scope}\ngenerate:\n  image: img:latest\n  command: gen cmd\n"
        )
    }

    fn write_def(dir: &Path, filename: &str, yaml: &str) {
        fs::write(dir.join(filename), yaml).unwrap();
    }

    #[test]
    fn load_valid_single_def() {
        let tmp = TempDir::new().unwrap();
        let custom = tmp.path().join("custom");
        fs::create_dir(&custom).unwrap();
        write_def(&custom, "my-gen.yaml", &valid_yaml("my-gen", "server"));

        let defs = load(tmp.path(), "custom").unwrap();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "my-gen");
        assert_eq!(defs[0].scope, "server");
        assert_eq!(defs[0].generate.image, "img:latest");
    }

    #[test]
    fn load_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let custom = tmp.path().join("custom");
        fs::create_dir(&custom).unwrap();

        let defs = load(tmp.path(), "custom").unwrap();
        assert!(defs.is_empty());
    }

    #[test]
    fn load_nonexistent_dir() {
        let tmp = TempDir::new().unwrap();
        let err = load(tmp.path(), "nope").unwrap_err();
        assert!(
            err.to_string().contains("does not exist"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn load_builtin_name_collision() {
        let tmp = TempDir::new().unwrap();
        let custom = tmp.path().join("custom");
        fs::create_dir(&custom).unwrap();
        write_def(&custom, "spring.yaml", &valid_yaml("spring", "server"));

        let err = load(tmp.path(), "custom").unwrap_err();
        assert!(
            err.to_string().contains("collides with built-in"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn load_duplicate_names_same_scope() {
        let tmp = TempDir::new().unwrap();
        let custom = tmp.path().join("custom");
        fs::create_dir(&custom).unwrap();
        write_def(&custom, "a.yaml", &valid_yaml("my-gen", "client"));
        write_def(&custom, "b.yaml", &valid_yaml("my-gen", "client"));

        let err = load(tmp.path(), "custom").unwrap_err();
        assert!(
            err.to_string().contains("Duplicate custom generator"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn load_invalid_scope() {
        let tmp = TempDir::new().unwrap();
        let custom = tmp.path().join("custom");
        fs::create_dir(&custom).unwrap();
        write_def(&custom, "gen.yaml", &valid_yaml("my-gen", "both"));

        let err = load(tmp.path(), "custom").unwrap_err();
        assert!(
            err.to_string().contains("invalid scope"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn load_unsafe_name() {
        let tmp = TempDir::new().unwrap();
        let custom = tmp.path().join("custom");
        fs::create_dir(&custom).unwrap();
        write_def(&custom, "gen.yaml", &valid_yaml("Bad-Name", "server"));

        let err = load(tmp.path(), "custom").unwrap_err();
        assert!(
            err.to_string().contains("invalid name"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn load_missing_generate_image() {
        let tmp = TempDir::new().unwrap();
        let custom = tmp.path().join("custom");
        fs::create_dir(&custom).unwrap();
        write_def(&custom, "gen.yaml", "name: my-gen\nscope: server\n");

        let err = load(tmp.path(), "custom").unwrap_err();
        assert!(
            err.to_string().contains("Failed to parse"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn load_skips_non_yaml_files() {
        let tmp = TempDir::new().unwrap();
        let custom = tmp.path().join("custom");
        fs::create_dir(&custom).unwrap();
        write_def(&custom, "readme.txt", "not yaml at all");
        write_def(&custom, "my-gen.yml", &valid_yaml("my-gen", "server"));

        let defs = load(tmp.path(), "custom").unwrap();
        assert_eq!(defs.len(), 1);
    }

    #[test]
    fn load_same_name_different_scope_ok() {
        let tmp = TempDir::new().unwrap();
        let custom = tmp.path().join("custom");
        fs::create_dir(&custom).unwrap();
        write_def(&custom, "a.yaml", &valid_yaml("my-gen", "server"));
        write_def(&custom, "b.yaml", &valid_yaml("my-gen", "client"));

        let defs = load(tmp.path(), "custom").unwrap();
        assert_eq!(defs.len(), 2);
    }
}
