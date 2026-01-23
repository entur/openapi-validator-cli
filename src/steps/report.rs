use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use crate::output::Output;
use crate::util::OAV_DIR;

#[derive(Debug)]
pub struct StatusEntry {
    pub stage: String,
    pub scope: String,
    pub target: String,
    pub status: String,
    pub log_path: String,
}

pub fn load_status_entries(status_path: &Path) -> Result<Vec<StatusEntry>> {
    if !status_path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(status_path).context("Failed to read status file")?;
    let entries = content
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 5 {
                Some(StatusEntry {
                    stage: parts[0].to_string(),
                    scope: parts[1].to_string(),
                    target: parts[2].to_string(),
                    status: parts[3].to_string(),
                    log_path: parts[4].to_string(),
                })
            } else {
                None
            }
        })
        .collect();
    Ok(entries)
}

pub fn run(root: &Path, output: &Output) -> Result<bool> {
    let reports_dir = root.join(OAV_DIR).join("reports");
    fs::create_dir_all(&reports_dir).context("Failed to create reports directory")?;
    let status_path = root.join(OAV_DIR).join("status.tsv");
    let output_path = reports_dir.join("dashboard.html");

    let entries = load_status_entries(&status_path)?;
    let html = generate_html(&entries);

    if let Err(err) = fs::write(&output_path, html) {
        if !output.quiet {
            eprintln!("Report generation failed: {err}");
        }
        return Ok(false);
    }

    Ok(true)
}

fn generate_html(entries: &[StatusEntry]) -> String {
    let total = entries.len();
    let passed = entries.iter().filter(|e| e.status == "ok").count();
    let failed = entries.iter().filter(|e| e.status == "fail").count();

    let mut html = String::from(HTML_HEAD);
    html.push_str(&format!(
        r#"    <div class="stat">
      <div class="stat-value">{total}</div>
      <div class="stat-label">Total</div>
    </div>
    <div class="stat pass">
      <div class="stat-value">{passed}</div>
      <div class="stat-label">Passed</div>
    </div>
    <div class="stat fail">
      <div class="stat-value">{failed}</div>
      <div class="stat-label">Failed</div>
    </div>
  </div>
"#
    ));

    for section in ["lint", "generate", "compile"] {
        let section_entries: Vec<&StatusEntry> =
            entries.iter().filter(|e| e.stage == section).collect();
        if section_entries.is_empty() {
            continue;
        }

        let title = match section {
            "lint" => "Lint",
            "generate" => "Generate",
            "compile" => "Compile",
            _ => section,
        };

        html.push_str(&format!(
            r#"  <div class="section">
    <h2>{title}</h2>
    <table class="result-table">
      <thead>
        <tr><th>Scope</th><th>Target</th><th>Status</th><th>Log</th></tr>
      </thead>
      <tbody>
"#
        ));

        for entry in section_entries {
            let badge = html_escape(&entry.status);
            let scope = html_escape(&entry.scope);
            let target = html_escape(&entry.target);
            let log_path = Path::new(&entry.log_path);
            let log_basename = log_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("log");
            let log_content = html_escape(&read_log_snippet(log_path));

            html.push_str(&format!(
                r#"        <tr>
          <td>{scope}</td>
          <td>{target}</td>
          <td><span class="badge {badge}">{badge}</span></td>
          <td>
            <details>
              <summary>{log_basename}</summary>
              <pre><code>{log_content}</code></pre>
            </details>
          </td>
        </tr>
"#
            ));
        }

        html.push_str(
            r#"      </tbody>
    </table>
  </div>
"#,
        );
    }

    html.push_str(HTML_FOOTER);
    html
}

fn read_log_snippet(path: &Path) -> String {
    match File::open(path) {
        Ok(file) => {
            let mut content = Vec::new();
            let _ = file.take(100000).read_to_end(&mut content);
            String::from_utf8_lossy(&content).to_string()
        }
        Err(_) => format!("Log file not found: {}", path.display()),
    }
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const HTML_HEAD: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>OpenAPI Validator Report</title>
  <style>
    :root {
      --bg: #0d1117; --fg: #c9d1d9; --border: #30363d;
      --green: #238636; --red: #da3633; --yellow: #d29922;
      --link: #58a6ff; --code-bg: #161b22;
    }
    * { box-sizing: border-box; }
    body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
           background: var(--bg); color: var(--fg); margin: 0; padding: 20px; line-height: 1.5; }
    h1, h2, h3 { margin-top: 0; font-weight: 600; }
    h1 { border-bottom: 1px solid var(--border); padding-bottom: 10px; }
    .summary { display: flex; gap: 20px; margin-bottom: 30px; flex-wrap: wrap; }
    .stat { background: var(--code-bg); border: 1px solid var(--border); border-radius: 6px;
            padding: 16px 24px; text-align: center; min-width: 120px; }
    .stat-value { font-size: 2em; font-weight: 600; }
    .stat-label { color: #8b949e; font-size: 0.9em; }
    .stat.pass .stat-value { color: var(--green); }
    .stat.fail .stat-value { color: var(--red); }
    .section { margin-bottom: 30px; }
    .result-table { width: 100%; border-collapse: collapse; background: var(--code-bg);
                   border: 1px solid var(--border); border-radius: 6px; overflow: hidden; }
    .result-table th, .result-table td { padding: 12px; text-align: left; border-bottom: 1px solid var(--border); }
    .result-table th { background: var(--bg); font-weight: 600; }
    .result-table tr:last-child td { border-bottom: none; }
    .badge { display: inline-block; padding: 2px 8px; border-radius: 12px; font-size: 0.85em; font-weight: 500; }
    .badge.ok { background: var(--green); color: #fff; }
    .badge.fail { background: var(--red); color: #fff; }
    details { background: var(--code-bg); border: 1px solid var(--border); border-radius: 6px; margin-top: 10px; }
    summary { padding: 12px; cursor: pointer; font-weight: 500; }
    summary:hover { background: var(--border); }
    pre { margin: 0; padding: 16px; overflow-x: auto; font-size: 0.85em;
          background: var(--bg); border-top: 1px solid var(--border); max-height: 500px; overflow-y: auto; }
    code { font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, monospace; }
    a { color: var(--link); text-decoration: none; }
    a:hover { text-decoration: underline; }
    .empty { color: #8b949e; font-style: italic; }
  </style>
</head>
<body>
  <h1>OpenAPI Validator Report</h1>
  <div class="summary">
"#;

const HTML_FOOTER: &str = r#"  <footer style="margin-top: 40px; padding-top: 20px; border-top: 1px solid var(--border); color: #8b949e; font-size: 0.85em;">
    Generated by OpenAPI Validator.
  </footer>
</body>
</html>
"#;
