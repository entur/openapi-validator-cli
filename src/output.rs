use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use std::env;
use std::io::{self, Write};
use std::time::Duration;

pub struct Output {
    pub verbose: bool,
    pub quiet: bool,
    color: bool,
    progress: bool,
}

impl Output {
    pub fn new(verbose: bool, quiet: bool) -> Self {
        let is_tty = atty::is(atty::Stream::Stdout);
        let color = is_tty && env::var_os("NO_COLOR").is_none();
        let progress = is_tty && !verbose && !quiet;
        Self {
            verbose,
            quiet,
            color,
            progress,
        }
    }

    pub fn start_spinner(&self, label: &str) -> Option<ProgressBar> {
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
        if self.quiet {
            return;
        }
        println!("{} {label}", self.status_icon(success));
    }

    pub fn phase_header(&self, label: &str) {
        if self.quiet {
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
        if self.quiet {
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
        if self.quiet {
            return;
        }
        let status = self.status_icon(success);
        if self.progress {
            print!("\r{status}   {label}\x1B[K\n");
        } else {
            println!("{status}   {label}");
        }
    }

    pub fn println(&self, message: &str) {
        if !self.quiet {
            println!("{message}");
        }
    }

    pub fn println_always(&self, message: &str) {
        println!("{message}");
    }

    pub fn print_error(&self, message: &str) {
        if self.color {
            eprintln!("{} {}", "error:".red().bold(), message);
        } else {
            eprintln!("error: {message}");
        }
    }

    pub fn print_summary(&self, passed: usize, failed: usize) {
        if self.quiet {
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
