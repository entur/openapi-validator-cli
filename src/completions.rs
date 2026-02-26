use anyhow::{Context, Result, bail};
use clap::CommandFactory;
use clap_complete::Shell;
use std::env;
use std::fs;
use std::path::PathBuf;

use crate::cli::Cli;
use crate::output::Output;

pub fn generate(shell: Shell) {
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "oav", &mut std::io::stdout());
}

pub fn install(shell_override: Option<Shell>, yes: bool, output: &Output) -> Result<()> {
    let shell = match shell_override {
        Some(s) => s,
        None => detect_shell()?,
    };

    let path = match completion_path(shell) {
        Some(p) => p,
        None => {
            output.print_warning(&format!(
                "Automatic installation is not supported for {shell}."
            ));
            output.println("Generate the script and add it to your shell config manually:");
            output.println(&format!("  oav completions generate {shell}"));
            return Ok(());
        }
    };

    let mut buf = Vec::new();
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "oav", &mut buf);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    fs::write(&path, &buf)
        .with_context(|| format!("Failed to write completions to {}", path.display()))?;
    output.print_success(&format!(
        "Installed {shell} completions to {}",
        path.display()
    ));

    if shell == Shell::Zsh {
        maybe_patch_zshrc(yes, output)?;
    }

    print_reload_hint(shell, output);
    Ok(())
}

pub fn uninstall(shell_override: Option<Shell>, yes: bool, output: &Output) -> Result<()> {
    let shell = match shell_override {
        Some(s) => s,
        None => detect_shell()?,
    };

    let path = match completion_path(shell) {
        Some(p) => p,
        None => {
            output.println(&format!(
                "No known completion path for {shell}. Nothing to remove."
            ));
            return Ok(());
        }
    };

    if !yes {
        if !path.exists() {
            output.println(&format!(
                "No completion file found at {}. Nothing to do.",
                path.display()
            ));
            return Ok(());
        }
        let confirm = dialoguer::Confirm::new()
            .with_prompt(format!("Remove {}?", path.display()))
            .default(true)
            .interact()?;
        if !confirm {
            output.println("Cancelled.");
            return Ok(());
        }
    }

    match fs::remove_file(&path) {
        Ok(()) => output.print_success(&format!("Removed {}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            output.println(&format!(
                "No completion file found at {}. Nothing to do.",
                path.display()
            ));
            return Ok(());
        }
        Err(e) => {
            return Err(
                anyhow::Error::new(e).context(format!("Failed to remove {}", path.display()))
            );
        }
    }

    if shell == Shell::Zsh {
        output.print_warning(
            "Any fpath lines added to ~/.zshrc were left in place. Remove them manually if desired.",
        );
    }

    Ok(())
}

fn detect_shell() -> Result<Shell> {
    let shell_env = env::var("SHELL").context("$SHELL is not set — pass --shell explicitly")?;
    let basename = shell_env.rsplit('/').next().unwrap_or(&shell_env);

    match basename {
        "bash" => Ok(Shell::Bash),
        "zsh" => Ok(Shell::Zsh),
        "fish" => Ok(Shell::Fish),
        "elvish" => Ok(Shell::Elvish),
        "pwsh" | "powershell" => Ok(Shell::PowerShell),
        other => bail!("Unrecognized shell '{other}' — pass --shell explicitly"),
    }
}

fn completion_path(shell: Shell) -> Option<PathBuf> {
    let home = dirs_path()?;
    match shell {
        Shell::Bash => Some(home.join(".local/share/bash-completion/completions/oav")),
        Shell::Zsh => Some(home.join(".zsh/completions/_oav")),
        Shell::Fish => Some(home.join(".config/fish/completions/oav.fish")),
        _ => None,
    }
}

fn dirs_path() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

const FPATH_LINE: &str = "fpath=(~/.zsh/completions $fpath); autoload -Uz compinit && compinit";

fn maybe_patch_zshrc(yes: bool, output: &Output) -> Result<()> {
    let Some(home) = dirs_path() else {
        return Ok(());
    };
    let zshrc = home.join(".zshrc");

    if zshrc.exists() {
        let contents = fs::read_to_string(&zshrc)?;
        if contents.contains("~/.zsh/completions") || contents.contains(".zsh/completions") {
            return Ok(());
        }
    }

    let should_append = if yes {
        true
    } else {
        dialoguer::Confirm::new()
            .with_prompt("Add fpath entry to ~/.zshrc?")
            .default(true)
            .interact()?
    };

    if should_append {
        use std::io::Write;
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&zshrc)
            .context("Failed to open ~/.zshrc")?;
        writeln!(f)?;
        writeln!(f, "# oav shell completions")?;
        writeln!(f, "{FPATH_LINE}")?;
        output.print_success("Updated ~/.zshrc with fpath entry.");
    } else {
        output.println("To enable completions, add this line to ~/.zshrc:");
        output.println(&format!("  {FPATH_LINE}"));
    }

    Ok(())
}

fn print_reload_hint(shell: Shell, output: &Output) {
    match shell {
        Shell::Bash => output.println("Reload with: source ~/.bashrc"),
        Shell::Zsh => output.println("Reload with: exec zsh"),
        Shell::Fish => output.println("Completions will be available in new fish sessions."),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_path_bash() {
        let path = completion_path(Shell::Bash);
        let path = path.expect("bash should have a completion path");
        assert!(path.ends_with(".local/share/bash-completion/completions/oav"));
    }

    #[test]
    fn completion_path_zsh() {
        let path = completion_path(Shell::Zsh);
        let path = path.expect("zsh should have a completion path");
        assert!(path.ends_with(".zsh/completions/_oav"));
    }

    #[test]
    fn completion_path_fish() {
        let path = completion_path(Shell::Fish);
        let path = path.expect("fish should have a completion path");
        assert!(path.ends_with(".config/fish/completions/oav.fish"));
    }

    #[test]
    fn completion_path_unsupported_shells_return_none() {
        assert!(completion_path(Shell::PowerShell).is_none());
        assert!(completion_path(Shell::Elvish).is_none());
    }

    #[test]
    fn fpath_line_is_valid_zsh_syntax() {
        assert!(FPATH_LINE.contains("fpath="));
        assert!(FPATH_LINE.contains("compinit"));
    }
}
