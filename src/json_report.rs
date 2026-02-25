use serde::Serialize;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::steps::StatusEntry;
use crate::util::OAV_DIR;

#[derive(Serialize)]
pub struct ValidateReport {
    pub spec: String,
    pub mode: String,
    pub phases: Phases,
    pub summary: Summary,
}

#[derive(Serialize)]
pub struct Phases {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lint: Option<LintResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate: Option<Vec<StepResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compile: Option<Vec<StepResult>>,
}

#[derive(Serialize)]
pub struct LintResult {
    pub linter: String,
    pub status: String,
    pub log: String,
}

#[derive(Serialize)]
pub struct StepResult {
    pub generator: String,
    pub scope: String,
    pub status: String,
    pub log: String,
}

#[derive(Serialize)]
pub struct Summary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
}

#[derive(Serialize)]
pub struct ConfigValue {
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Serialize)]
pub struct GeneratorList {
    pub server: Vec<String>,
    pub client: Vec<String>,
}

const MAX_LOG_BYTES: u64 = 100_000;

fn read_log(path: &Path) -> String {
    match File::open(path) {
        Ok(file) => {
            let mut buf = Vec::new();
            let _ = file.take(MAX_LOG_BYTES).read_to_end(&mut buf);
            String::from_utf8_lossy(&buf).into_owned()
        }
        Err(_) => String::new(),
    }
}

pub fn build_validate_report(
    root: &Path,
    spec: &str,
    mode: &str,
    entries: &[StatusEntry],
) -> ValidateReport {
    let lint_entries: Vec<&StatusEntry> = entries.iter().filter(|e| e.stage == "lint").collect();
    let gen_entries: Vec<&StatusEntry> = entries.iter().filter(|e| e.stage == "generate").collect();
    let compile_entries: Vec<&StatusEntry> =
        entries.iter().filter(|e| e.stage == "compile").collect();

    let lint = if lint_entries.is_empty() {
        None
    } else {
        let e = lint_entries[0];
        Some(LintResult {
            linter: e.target.clone(),
            status: normalize_status(&e.status),
            log: read_log(&root.join(OAV_DIR).join(&e.log_path)),
        })
    };

    let generate = if gen_entries.is_empty() {
        None
    } else {
        Some(
            gen_entries
                .iter()
                .map(|e| StepResult {
                    generator: e.target.clone(),
                    scope: e.scope.clone(),
                    status: normalize_status(&e.status),
                    log: read_log(&root.join(OAV_DIR).join(&e.log_path)),
                })
                .collect(),
        )
    };

    let compile = if compile_entries.is_empty() {
        None
    } else {
        Some(
            compile_entries
                .iter()
                .map(|e| StepResult {
                    generator: e.target.clone(),
                    scope: e.scope.clone(),
                    status: normalize_status(&e.status),
                    log: read_log(&root.join(OAV_DIR).join(&e.log_path)),
                })
                .collect(),
        )
    };

    let passed = entries.iter().filter(|e| e.status == "ok").count();
    let failed = entries.iter().filter(|e| e.status == "fail").count();

    ValidateReport {
        spec: spec.to_string(),
        mode: mode.to_string(),
        phases: Phases {
            lint,
            generate,
            compile,
        },
        summary: Summary {
            total: entries.len(),
            passed,
            failed,
        },
    }
}

fn normalize_status(status: &str) -> String {
    match status {
        "ok" => "pass".to_string(),
        other => other.to_string(),
    }
}
