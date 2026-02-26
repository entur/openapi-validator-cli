use console::Term;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use std::env;
use std::io::{self, IsTerminal, Write};
use std::time::Duration;

use crate::cli::ColorMode;

pub struct Output {
    pub verbose: bool,
    pub quiet: bool,
    pub json: bool,
    color: bool,
    progress: bool,
}

impl Output {
    pub fn new(verbose: bool, quiet: bool, json: bool, color_mode: ColorMode) -> Self {
        let is_tty = io::stdout().is_terminal();
        let color = match color_mode {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => is_tty && env::var_os("NO_COLOR").is_none(),
        };
        let progress = is_tty && !verbose && !quiet && !json;
        Self {
            verbose,
            quiet,
            json,
            color,
            progress,
        }
    }

    pub fn start_spinner(&self, label: &str) -> Option<ProgressBar> {
        if self.json {
            return None;
        }
        if self.progress {
            let spinner = ProgressBar::new_spinner();
            let style = ProgressStyle::with_template("{spinner} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner())
                .tick_strings(&["-", "\\", "|", "/"]);
            spinner.set_style(style);
            spinner.set_message(label.to_string());
            spinner.enable_steady_tick(Duration::from_millis(100));
            Some(spinner)
        } else {
            if self.verbose && !self.quiet {
                println!("==> {label}");
            }
            None
        }
    }

    pub fn finish_spinner(&self, spinner: Option<&ProgressBar>, label: &str, success: bool) {
        if let Some(spinner) = spinner {
            spinner.finish_and_clear();
        }
        if self.quiet || self.json {
            return;
        }
        println!("{} {label}", self.status_icon(success));
    }

    pub fn phase_header(&self, label: &str) {
        if self.quiet || self.json {
            return;
        }
        println!();
        if self.color {
            println!("{}", label.bold());
        } else {
            println!("{label}");
        }
    }

    pub fn substep_start(&self, label: &str) {
        if self.quiet || self.json {
            return;
        }
        if self.progress {
            print!("  {label}...");
            let _ = io::stdout().flush();
        } else if self.verbose {
            println!("{label}...");
        }
    }

    pub fn substep_finish(&self, label: &str, success: bool) {
        if self.quiet || self.json {
            return;
        }
        let status = self.status_icon(success);
        if self.progress {
            let term = Term::stdout();
            let _ = term.clear_line();
            println!("{status}   {label}");
        } else {
            println!("{status}   {label}");
        }
    }

    pub fn println(&self, message: &str) {
        if !self.quiet && !self.json {
            println!("{message}");
        }
    }

    pub fn print_success(&self, message: &str) {
        if self.quiet || self.json {
            return;
        }
        if self.color {
            println!("{} {message}", "✓".green());
        } else {
            println!("{message}");
        }
    }

    pub fn print_detail(&self, label: &str, value: &str) {
        if self.quiet || self.json {
            return;
        }
        if self.color {
            println!("  {} {value}", format!("{label}:").dimmed());
        } else {
            println!("  {label}: {value}");
        }
    }

    pub fn print_detail_ignore_quiet(&self, label: &str, value: &str) {
        if self.json {
            return;
        }
        if self.color {
            println!("  {} {value}", format!("{label}:").dimmed());
        } else {
            println!("  {label}: {value}");
        }
    }

    pub fn print_error(&self, message: &str) {
        if self.color {
            eprintln!("{} {}", "error:".red().bold(), message);
        } else {
            eprintln!("error: {message}");
        }
    }

    pub fn print_warning(&self, message: &str) {
        if self.color {
            eprintln!("{} {}", "warning:".yellow().bold(), message);
        } else {
            eprintln!("warning: {message}");
        }
    }

    pub fn multi_progress(&self) -> Option<MultiProgress> {
        if self.progress {
            Some(MultiProgress::new())
        } else {
            None
        }
    }

    pub fn add_parallel_spinner(&self, mp: &MultiProgress, label: &str) -> Option<ProgressBar> {
        if self.progress {
            let spinner = mp.add(ProgressBar::new_spinner());
            let style = ProgressStyle::with_template("  {spinner} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner())
                .tick_strings(&["-", "\\", "|", "/"]);
            spinner.set_style(style);
            spinner.set_message(label.to_string());
            spinner.enable_steady_tick(Duration::from_millis(100));
            Some(spinner)
        } else {
            None
        }
    }

    pub fn finish_parallel_spinner(
        &self,
        mp: &MultiProgress,
        spinner: Option<ProgressBar>,
        label: &str,
        success: bool,
    ) {
        if let Some(spinner) = spinner {
            spinner.finish_and_clear();
        }
        if self.quiet || self.json {
            return;
        }
        let status = self.status_icon(success);
        let _ = mp.println(format!("{status}   {label}"));
    }

    pub fn print_summary(&self, passed: usize, failed: usize) {
        if self.quiet || self.json {
            return;
        }
        println!();
        if self.color {
            let passed_str = format!("{passed} passed").green().to_string();
            let failed_str = if failed > 0 {
                format!("{failed} failed").red().to_string()
            } else {
                format!("{failed} failed").dimmed().to_string()
            };
            println!("{passed_str}, {failed_str}");
        } else {
            println!("{passed} passed, {failed} failed");
        }
    }

    fn status_icon(&self, success: bool) -> String {
        if self.color {
            if success {
                "✓".green().to_string()
            } else {
                "✗".red().to_string()
            }
        } else if success {
            "OK".to_string()
        } else {
            "FAIL".to_string()
        }
    }
}
