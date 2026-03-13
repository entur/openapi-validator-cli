#[allow(dead_code)]
mod common;

use assert_cmd::prelude::*;
use common::{fixture_path, oav_command};
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

// --- Root-level quick check ---

#[test]
fn discover_root_yaml() {
    let temp = TempDir::new().unwrap();
    fs::copy(fixture_path("valid.yml"), temp.path().join("openapi.yaml")).unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["init", "--quiet"])
        .assert()
        .success();
}

#[test]
fn discover_root_yml() {
    let temp = TempDir::new().unwrap();
    fs::copy(fixture_path("valid.yml"), temp.path().join("openapi.yml")).unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["init", "--quiet"])
        .assert()
        .success();
}

#[test]
fn discover_root_json() {
    let temp = TempDir::new().unwrap();
    fs::copy(fixture_path("valid.json"), temp.path().join("openapi.json")).unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["init", "--quiet"])
        .assert()
        .success();
}

// --- Deep discovery via walkdir ---

#[test]
fn discover_nested_yaml() {
    let temp = TempDir::new().unwrap();
    let nested = temp.path().join("api").join("v1");
    fs::create_dir_all(&nested).unwrap();
    fs::copy(fixture_path("valid.yml"), nested.join("spec.yaml")).unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["init", "--quiet"])
        .assert()
        .success();
}

#[test]
fn discover_nested_json() {
    let temp = TempDir::new().unwrap();
    let nested = temp.path().join("api").join("v1");
    fs::create_dir_all(&nested).unwrap();
    fs::copy(fixture_path("valid.json"), nested.join("spec.json")).unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["init", "--quiet"])
        .assert()
        .success();
}

// --- Non-spec files are ignored ---

#[test]
fn json_without_openapi_key_ignored() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("config.json"),
        r#"{"name": "not a spec", "version": "1.0"}"#,
    )
    .unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["init", "--quiet"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No OpenAPI spec found"));
}

#[test]
fn yaml_without_openapi_key_ignored() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("config.yaml"),
        "name: not a spec\nversion: 1.0\n",
    )
    .unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["init", "--quiet"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No OpenAPI spec found"));
}

#[test]
fn json_with_openapi_as_value_ignored() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("meta.json"),
        r#"{"type": "openapi", "name": "some tool"}"#,
    )
    .unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["init", "--quiet"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No OpenAPI spec found"));
}

// --- Explicit --spec with JSON ---

#[test]
fn explicit_spec_json() {
    let temp = TempDir::new().unwrap();
    fs::copy(fixture_path("valid.json"), temp.path().join("my-api.json")).unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["init", "--spec", "my-api.json"])
        .assert()
        .success();
}
