#[allow(dead_code)]
mod common;

use assert_cmd::prelude::*;
use common::{fixture_path, oav_command};
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn blank_spec_rejected() {
    let temp = TempDir::new().unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["init", "--spec", "  "])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--spec cannot be blank"));
}

#[test]
fn invalid_server_generators_rejected() {
    let temp = TempDir::new().unwrap();
    fs::copy(fixture_path("valid.yml"), temp.path().join("openapi.yaml")).unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["init", "--server-generators", "fake"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Unsupported server generator: 'fake'",
        ));
}

#[test]
fn invalid_client_generators_rejected() {
    let temp = TempDir::new().unwrap();
    fs::copy(fixture_path("valid.yml"), temp.path().join("openapi.yaml")).unwrap();
    oav_command()
        .current_dir(temp.path())
        .args(["init", "--client-generators", "fake"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Unsupported client generator: 'fake'",
        ));
}
