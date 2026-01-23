mod compile;
mod generate;
mod lint;
mod report;

pub use compile::run as compile;
pub use generate::run as generate;
pub use lint::run as lint;
pub use report::{load_status_entries, run as report};

use anyhow::Result;

use crate::output::Output;

pub fn run_step(
    output: &Output,
    label: &str,
    show_spinner: bool,
    show_summary: bool,
    action: impl FnOnce() -> Result<bool>,
) -> Result<bool> {
    let spinner = if show_spinner {
        output.start_spinner(label)
    } else {
        None
    };
    let result = action();
    let success = result.as_ref().map(|ok| *ok).unwrap_or(false);
    if show_summary {
        output.finish_spinner(spinner.as_ref(), label, success);
    } else if let Some(spinner) = spinner.as_ref() {
        spinner.finish_and_clear();
    }
    result
}
