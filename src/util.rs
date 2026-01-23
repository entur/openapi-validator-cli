use anyhow::{Context, Result, bail};
use include_dir::{Dir, DirEntry};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const OAV_DIR: &str = ".oav";

/// Convert a path to a POSIX-style string for use in container paths.
/// On Windows, backslashes are converted to forward slashes.
pub fn to_posix_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub fn ensure_oav_dir(root: &Path) -> Result<()> {
    fs::create_dir_all(root.join(OAV_DIR)).context("Failed to create .oav directory")?;
    Ok(())
}

pub fn prepare_runtime_dirs(root: &Path) -> Result<()> {
    let oav_dir = root.join(OAV_DIR);
    fs::create_dir_all(oav_dir.join("reports").join("lint"))?;
    fs::create_dir_all(oav_dir.join("reports").join("generate").join("server"))?;
    fs::create_dir_all(oav_dir.join("reports").join("generate").join("client"))?;
    fs::create_dir_all(oav_dir.join("reports").join("compile").join("server"))?;
    fs::create_dir_all(oav_dir.join("reports").join("compile").join("client"))?;
    fs::create_dir_all(oav_dir.join("generated"))?;
    fs::write(oav_dir.join("status.tsv"), "")?;
    Ok(())
}

pub fn extract_assets(root: &Path, assets: &Dir) -> Result<()> {
    let target = root.join(OAV_DIR);
    fs::create_dir_all(&target).context("Failed to create .oav directory")?;
    write_assets(&target, assets)?;
    Ok(())
}

fn write_assets(target: &Path, dir: &Dir) -> Result<()> {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(child) => {
                let dest = target.join(child.path());
                fs::create_dir_all(&dest)
                    .with_context(|| format!("Failed to create {}", dest.display()))?;
                write_assets(target, child)?;
            }
            DirEntry::File(file) => {
                let dest = target.join(file.path());
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("Failed to create {}", parent.display()))?;
                }
                fs::write(&dest, file.contents())
                    .with_context(|| format!("Failed to write {}", dest.display()))?;
                set_script_permissions(&dest)?;
            }
        }
    }
    Ok(())
}

fn set_script_permissions(path: &Path) -> Result<()> {
    if path.extension().and_then(|ext| ext.to_str()) != Some("sh") {
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perm = fs::Permissions::from_mode(0o755);
        fs::set_permissions(path, perm)
            .with_context(|| format!("Failed to set permissions on {}", path.display()))?;
    }
    Ok(())
}

// Gitignore management

pub fn ensure_gitignore(root: &Path, ignore_config: bool) -> Result<()> {
    let mut entries = vec![".oav/"];
    if ignore_config {
        entries.push(".oavc");
    }
    add_gitignore_entries(root, &entries)
}

pub fn add_gitignore_entries(root: &Path, entries: &[&str]) -> Result<()> {
    let path = root.join(".gitignore");
    let mut content = if path.exists() {
        fs::read_to_string(&path).context("Failed to read .gitignore")?
    } else {
        String::new()
    };

    let mut changed = false;
    for entry in entries {
        if !content.lines().any(|line| line.trim() == *entry) {
            if !content.ends_with('\n') && !content.is_empty() {
                content.push('\n');
            }
            content.push_str(entry);
            content.push('\n');
            changed = true;
        }
    }

    if changed {
        fs::write(&path, content).context("Failed to update .gitignore")?;
    }
    Ok(())
}

pub fn remove_gitignore_entries(root: &Path, entries: &[&str]) -> Result<()> {
    let path = root.join(".gitignore");
    if !path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(&path).context("Failed to read .gitignore")?;
    let kept: Vec<&str> = content
        .lines()
        .filter(|line| !entries.iter().any(|entry| line.trim() == *entry))
        .collect();
    let mut new_content = kept.join("\n");
    if !new_content.is_empty() {
        new_content.push('\n');
    }
    fs::write(&path, new_content).context("Failed to update .gitignore")?;
    Ok(())
}

// Spec discovery

pub fn normalize_spec_path(root: &Path, spec: &str) -> Result<PathBuf> {
    let spec_path = PathBuf::from(spec);
    let absolute = if spec_path.is_absolute() {
        spec_path
    } else {
        root.join(&spec_path)
    };
    if !absolute.exists() {
        bail!("Spec file not found: {}", absolute.display());
    }
    let relative = absolute
        .strip_prefix(root)
        .context("Spec path must be inside the repository")?;
    Ok(relative.to_path_buf())
}

pub fn discover_spec(root: &Path) -> Result<Option<String>> {
    for name in ["openapi.yaml", "openapi.yml"] {
        let candidate = root.join(name);
        if candidate.is_file() {
            return Ok(Some(name.to_string()));
        }
    }

    let mut matches = Vec::new();
    let walker = walkdir::WalkDir::new(root)
        .max_depth(4)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !should_skip_entry(entry));

    for entry in walker.filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !is_yaml(path) || !is_openapi_spec(path) {
            continue;
        }
        if let Ok(rel) = path.strip_prefix(root) {
            matches.push(rel.to_string_lossy().to_string());
        }
    }

    if matches.is_empty() {
        return Ok(None);
    }

    matches.sort();
    select_spec_from_candidates(matches)
}

fn is_yaml(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_lowercase()),
        Some(ext) if ext == "yaml" || ext == "yml"
    )
}

fn is_openapi_spec(path: &Path) -> bool {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return false,
    };
    let mut content = String::new();
    if file.read_to_string(&mut content).is_err() {
        return false;
    }
    let doc: serde_yaml::Value = match serde_yaml::from_str(&content) {
        Ok(doc) => doc,
        Err(_) => return false,
    };
    match doc {
        serde_yaml::Value::Mapping(mapping) => mapping
            .keys()
            .filter_map(|key| key.as_str())
            .any(|key| key == "openapi"),
        _ => false,
    }
}

fn should_skip_entry(entry: &walkdir::DirEntry) -> bool {
    if entry.depth() == 0 || !entry.file_type().is_dir() {
        return false;
    }
    matches!(
        entry.file_name().to_str().unwrap_or_default(),
        ".git" | ".oav" | "target" | "node_modules" | ".idea" | ".vscode"
    )
}

fn select_spec_from_candidates(candidates: Vec<String>) -> Result<Option<String>> {
    println!("No default OpenAPI spec found.");
    println!("Select a spec to use:");
    for (idx, path) in candidates.iter().enumerate() {
        println!("  {}) {}", idx + 1, path);
    }
    println!("  q) quit");

    let mut input = String::new();
    loop {
        print!("Select [1-{}] or q: ", candidates.len());
        io::stdout().flush().context("Failed to flush stdout")?;
        input.clear();
        io::stdin()
            .read_line(&mut input)
            .context("Failed to read input")?;
        let trimmed = input.trim();
        if trimmed.eq_ignore_ascii_case("q") {
            return Ok(None);
        }
        if let Ok(choice) = trimmed.parse::<usize>() {
            if choice >= 1 && choice <= candidates.len() {
                return Ok(Some(candidates[choice - 1].clone()));
            }
        }
        println!("Invalid selection.");
    }
}

// Logging utilities

pub fn write_log_header(log_path: &Path, command_line: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(log_path)
        .context("Failed to create log file")?;
    writeln!(file, "{command_line}")?;
    writeln!(file)?;
    Ok(())
}

pub fn append_status(
    root: &Path,
    stage: &str,
    scope: &str,
    target: &str,
    status: &str,
    log_path: &Path,
) -> Result<()> {
    let status_path = root.join(OAV_DIR).join("status.tsv");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(status_path)
        .context("Failed to open status file")?;
    writeln!(
        file,
        "{stage}\t{scope}\t{target}\t{status}\t{}",
        log_path.display()
    )?;
    Ok(())
}

pub fn append_error(log_path: &Path, message: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .context("Failed to write error log")?;
    writeln!(file, "{message}")?;
    Ok(())
}
