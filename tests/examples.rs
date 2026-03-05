#[allow(dead_code)]
mod common;

use assert_cmd::prelude::*;
use common::oav_command;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples")
}

/// Copy an example directory into a temp dir and run `oav config validate`.
/// This proves the .oavc parses correctly and all field values are valid.
fn validate_example(name: &str) {
    let src = examples_dir().join(name);
    assert!(src.is_dir(), "example dir not found: {}", src.display());

    let tmp = TempDir::new().unwrap();
    copy_dir_recursive(&src, tmp.path());

    oav_command()
        .current_dir(tmp.path())
        .args(["config", "validate"])
        .assert()
        .success();
}

fn copy_dir_recursive(src: &Path, dst: &Path) {
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            fs::create_dir_all(&dst_path).unwrap();
            copy_dir_recursive(&src_path, &dst_path);
        } else {
            fs::copy(&src_path, &dst_path).unwrap();
        }
    }
}

#[test]
fn example_client_only() {
    validate_example("client-only");
}

#[test]
fn example_custom_generators() {
    validate_example("custom-generators");
}

#[test]
fn example_custom_linter() {
    validate_example("custom-linter");
}

#[test]
fn example_server_and_client() {
    validate_example("server-and-client");
}

#[test]
fn example_lint_only() {
    validate_example("lint-only");
}
