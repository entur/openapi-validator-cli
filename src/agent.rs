use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::output::Output;

const SKILL_DIR: &str = "oav";

struct SkillFile {
    filename: &'static str,
    content: &'static str,
}

const SKILL_FILES: &[SkillFile] = &[
    SkillFile {
        filename: "SKILL.md",
        content: include_str!("../assets/skills/oav/SKILL.md"),
    },
    SkillFile {
        filename: "reference.md",
        content: include_str!("../assets/skills/oav/reference.md"),
    },
];

fn skill_dir(root: &Path) -> std::path::PathBuf {
    root.join(".claude").join("skills").join(SKILL_DIR)
}

pub fn is_installed(root: &Path) -> bool {
    skill_dir(root).exists()
}

pub fn install(root: &Path, force: bool, output: &Output) -> Result<()> {
    let dir = skill_dir(root);

    if dir.exists() && !force {
        output.print_warning("oav skill already installed — use --force to overwrite");
        output.println(&format!("    {}", dir.display()));
        return Ok(());
    }

    fs::create_dir_all(&dir).with_context(|| format!("Failed to create {}", dir.display()))?;

    for file in SKILL_FILES {
        let path = dir.join(file.filename);
        fs::write(&path, file.content)
            .with_context(|| format!("Failed to write {}", path.display()))?;
    }

    output.print_success("Installed oav agent skill.");
    output.print_detail("Location", &dir.display().to_string());
    output.print_detail("Slash command", "/oav");
    Ok(())
}

pub fn uninstall(root: &Path, output: &Output) -> Result<()> {
    let dir = skill_dir(root);

    if !dir.exists() {
        output.println("oav skill is not installed. Nothing to do.");
        return Ok(());
    }

    fs::remove_dir_all(&dir).with_context(|| format!("Failed to remove {}", dir.display()))?;

    output.print_success(&format!("Removed {}.", dir.display()));

    // Clean up parent dirs if now empty
    let skills_dir = root.join(".claude").join("skills");
    let claude_dir = root.join(".claude");
    remove_if_empty(&skills_dir);
    remove_if_empty(&claude_dir);

    Ok(())
}

fn remove_if_empty(dir: &Path) {
    if dir.exists()
        && let Ok(mut entries) = dir.read_dir()
        && entries.next().is_none()
    {
        let _ = fs::remove_dir(dir);
    }
}
