use anyhow::{Context, Result, bail};
use indicatif::ProgressBar;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::output::Output;
use crate::util;

const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

/// Guard that clears a spinner on drop, ensuring it never leaks on early returns.
struct SpinnerGuard<'a> {
    spinner: Option<ProgressBar>,
    output: &'a Output,
    label: &'static str,
    success: bool,
}

impl<'a> SpinnerGuard<'a> {
    fn finish(&mut self, label: &'static str, success: bool) {
        self.label = label;
        self.success = success;
    }
}

impl Drop for SpinnerGuard<'_> {
    fn drop(&mut self) {
        self.output
            .finish_spinner(self.spinner.as_ref(), self.label, self.success);
    }
}

/// Fetch a spec from a URL and write it to `.oav/fetched-spec.{ext}`.
/// Returns the path to the fetched file, relative to `root`.
pub fn fetch_spec(root: &Path, url: &str, output: &Output) -> Result<PathBuf> {
    validate_url(url)?;

    let mut guard = SpinnerGuard {
        spinner: output.start_spinner(&format!("Fetching spec from {url}")),
        output,
        label: "Fetch failed",
        success: false,
    };

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(FETCH_TIMEOUT))
        .build()
        .into();

    let response = agent
        .get(url)
        .call()
        .with_context(|| format!("Failed to fetch spec from {url}"))?;

    let status = response.status();
    if status.as_u16() >= 300 {
        bail!("Failed to fetch spec from {url}: HTTP {status}");
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let body = response
        .into_body()
        .read_to_string()
        .with_context(|| format!("Failed to read response body from {url}"))?;

    if body.trim().is_empty() {
        bail!("Fetched spec from {url} is empty");
    }

    let ext = detect_extension(url, &content_type, &body);
    let is_json = ext == "json";

    if !util::looks_like_openapi(&body, is_json) {
        bail!("Fetched content from {url} does not appear to be an OpenAPI spec");
    }

    let dest = root.join(util::OAV_DIR).join(format!("fetched-spec.{ext}"));
    fs::write(&dest, &body)
        .with_context(|| format!("Failed to write fetched spec to {}", dest.display()))?;

    guard.finish("Fetched spec", true);

    // Return the path relative to root
    let relative = dest
        .strip_prefix(root)
        .context("Fetched spec path must be inside the repository")?;
    Ok(relative.to_path_buf())
}

/// Returns `true` if the string looks like a URL (any scheme with `://`).
pub fn looks_like_url(s: &str) -> bool {
    s.contains("://")
}

/// Returns `true` if the string is a supported (HTTP/HTTPS) URL.
pub fn is_http_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

fn validate_url(url: &str) -> Result<()> {
    if !is_http_url(url) {
        bail!("Spec URL must use HTTP or HTTPS");
    }
    Ok(())
}

/// Detect the file extension for the fetched spec.
/// Priority: URL path extension > Content-Type header > content sniffing.
fn detect_extension(url: &str, content_type: &str, body: &str) -> &'static str {
    // 1. URL path extension
    if let Some(ext) = extension_from_url(url) {
        return ext;
    }

    // 2. Content-Type header
    if let Some(ext) = extension_from_content_type(content_type) {
        return ext;
    }

    // 3. Content sniffing
    if body.trim_start().starts_with('{') {
        "json"
    } else {
        "yaml"
    }
}

fn extension_from_url(url: &str) -> Option<&'static str> {
    // Strip query string and fragment
    let path = url.split('?').next().unwrap_or(url);
    let path = path.split('#').next().unwrap_or(path);

    if path.ends_with(".json") {
        Some("json")
    } else if path.ends_with(".yaml") || path.ends_with(".yml") {
        Some("yaml")
    } else {
        None
    }
}

fn extension_from_content_type(ct: &str) -> Option<&'static str> {
    let lower = ct.to_lowercase();
    if lower.contains("json") {
        Some("json")
    } else if lower.contains("yaml") || lower.contains("yml") {
        Some("yaml")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_from_url() {
        assert_eq!(
            extension_from_url("https://example.com/spec.json"),
            Some("json")
        );
        assert_eq!(
            extension_from_url("https://example.com/spec.yaml"),
            Some("yaml")
        );
        assert_eq!(
            extension_from_url("https://example.com/spec.yml"),
            Some("yaml")
        );
        assert_eq!(
            extension_from_url("https://example.com/spec.json?v=1"),
            Some("json")
        );
        assert_eq!(
            extension_from_url("https://example.com/spec.yaml#section"),
            Some("yaml")
        );
        assert_eq!(
            extension_from_url("https://example.com/api/v3/openapi"),
            None
        );
    }

    #[test]
    fn test_extension_from_content_type() {
        assert_eq!(
            extension_from_content_type("application/json"),
            Some("json")
        );
        assert_eq!(
            extension_from_content_type("application/x-yaml"),
            Some("yaml")
        );
        assert_eq!(
            extension_from_content_type("text/yaml; charset=utf-8"),
            Some("yaml")
        );
        assert_eq!(extension_from_content_type("text/plain"), None);
    }

    #[test]
    fn test_detect_extension_url_takes_priority() {
        assert_eq!(
            detect_extension("https://example.com/spec.json", "text/yaml", "openapi: 3.0"),
            "json"
        );
    }

    #[test]
    fn test_detect_extension_content_type_fallback() {
        assert_eq!(
            detect_extension(
                "https://example.com/api/spec",
                "application/json",
                "openapi: 3.0"
            ),
            "json"
        );
    }

    #[test]
    fn test_detect_extension_sniff_fallback() {
        assert_eq!(
            detect_extension(
                "https://example.com/api/spec",
                "text/plain",
                r#"{"openapi":"3.0"}"#
            ),
            "json"
        );
        assert_eq!(
            detect_extension("https://example.com/api/spec", "text/plain", "openapi: 3.0"),
            "yaml"
        );
    }

    #[test]
    fn test_looks_like_url() {
        assert!(looks_like_url("ftp://example.com/spec.yaml"));
        assert!(looks_like_url("https://example.com/spec.yaml"));
        assert!(looks_like_url("http://example.com/spec.yaml"));
        assert!(!looks_like_url("/local/path/spec.yaml"));
        assert!(!looks_like_url("relative/path.yaml"));
    }

    #[test]
    fn test_is_http_url() {
        assert!(!is_http_url("ftp://example.com/spec.yaml"));
        assert!(!is_http_url("/local/path/spec.yaml"));
        assert!(is_http_url("https://example.com/spec.yaml"));
        assert!(is_http_url("http://example.com/spec.yaml"));
    }

    #[test]
    fn test_validate_url_rejects_non_http() {
        assert!(validate_url("ftp://example.com/spec.yaml").is_err());
        assert!(validate_url("/local/path/spec.yaml").is_err());
        assert!(validate_url("https://example.com/spec.yaml").is_ok());
        assert!(validate_url("http://example.com/spec.yaml").is_ok());
    }
}
