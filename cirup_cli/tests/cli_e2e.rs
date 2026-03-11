use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn fixture_path(name: &str) -> PathBuf {
    repo_root().join("cirup_core").join("test").join(name)
}

fn cirup_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cirup"))
}

fn run_cirup(args: &[&str]) -> Output {
    cirup_command().args(args).output().expect("run cirup")
}

fn stdout_string(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be valid utf-8")
}

fn stderr_string(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be valid utf-8")
}

#[test]
fn check_returns_exit_code_two_for_detected_changes() {
    let new_file = fixture_path("test_new.resx");
    let old_file = fixture_path("test_old.resx");

    let output = run_cirup(&[
        "--check",
        "file-diff",
        &new_file.to_string_lossy(),
        &old_file.to_string_lossy(),
    ]);

    assert_eq!(output.status.code(), Some(2));
    assert!(stdout_string(&output).is_empty());
    assert!(stderr_string(&output).is_empty());
}

#[test]
fn check_summary_reports_change_detection() {
    let new_file = fixture_path("test_new.resx");
    let old_file = fixture_path("test_old.resx");

    let output = run_cirup(&[
        "--check",
        "--summary",
        "--output-format",
        "json",
        "file-diff",
        &new_file.to_string_lossy(),
        &old_file.to_string_lossy(),
    ]);

    assert_eq!(output.status.code(), Some(2));

    let report: Value = serde_json::from_str(&stdout_string(&output)).expect("summary json");
    assert_eq!(report["operation"], "file-diff");
    assert_eq!(report["result_kind"], "resource");
    assert_eq!(report["output_count"], 3);
    assert_eq!(report["check"], true);
    assert_eq!(report["dry_run"], true);
    assert_eq!(report["change_detected"], true);
}

#[test]
fn count_only_prints_diff_count() {
    let new_file = fixture_path("test_new.resx");
    let old_file = fixture_path("test_old.resx");

    let output = run_cirup(&[
        "--count-only",
        "file-diff",
        &new_file.to_string_lossy(),
        &old_file.to_string_lossy(),
    ]);

    assert!(output.status.success());
    assert_eq!(stdout_string(&output), "3\n");
    assert!(stderr_string(&output).is_empty());
}

#[test]
fn output_format_json_prints_filtered_resource_array() {
    let input = fixture_path("test.json");

    let output = run_cirup(&[
        "--output-format",
        "json",
        "--key-filter",
        "^lblBoat$",
        "file-print",
        &input.to_string_lossy(),
    ]);

    assert!(output.status.success());

    let rows: Value = serde_json::from_str(&stdout_string(&output)).expect("resource json");
    let array = rows.as_array().expect("json array output");

    assert_eq!(array.len(), 1);
    assert_eq!(array[0]["name"], "lblBoat");
    assert_eq!(array[0]["value"], "I'm on a boat.");
}

#[test]
fn dry_run_summary_reports_in_place_write_without_modifying_file() {
    let temp = tempdir().expect("tempdir");
    let file = temp.path().join("strings.json");
    fs::write(&file, "{\n  \"z\": \"last\",\n  \"a\": \"first\"\n}\n").expect("write temp file");
    let original = fs::read_to_string(&file).expect("read original file");

    let output = run_cirup(&[
        "--dry-run",
        "--summary",
        "--output-format",
        "json",
        "file-sort",
        &file.to_string_lossy(),
    ]);

    assert!(output.status.success());

    let report: Value = serde_json::from_str(&stdout_string(&output)).expect("summary json");
    assert_eq!(report["operation"], "file-sort");
    assert_eq!(report["dry_run"], true);
    assert_eq!(report["would_write"], true);
    assert_eq!(report["wrote_output"], false);
    assert_eq!(report["change_detected"], true);
    assert_eq!(report["output_file"], file.to_string_lossy().as_ref());

    let after = fs::read_to_string(&file).expect("read file after dry-run");
    assert_eq!(after, original);
}
