use std::error::Error;
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::str;

#[cfg(windows)]
const LOCATE_COMMAND: &'static str = "where";
#[cfg(not(windows))]
const LOCATE_COMMAND: &'static str = "which";

pub fn status(exe: &str, dir: &Path, args: &[&str]) -> Result<i32, Box<dyn Error>> {
    trace!("{} {:?}", exe, args);
    let status = Command::new(exe)
        .current_dir(dir)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    match status.code() {
        Some(c) => Ok(c),
        None => Err("process terminated by signal")?,
    }
}

pub fn output(exe: &str, dir: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    trace!("{} {:?}", exe, args);
    let output = Command::new(exe).current_dir(dir).args(args).output()?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn output_to_file(exe: &str, dir: &Path, args: &[&str], out: &Path) -> Result<(), Box<dyn Error>> {
    let mut file = OpenOptions::new().write(true).create(true).open(out)?;

    let output = output(exe, dir, args)?;

    file.write_all(output.as_bytes())?;

    Ok(())
}

pub fn find_binary(binary: &str) -> Option<::std::path::PathBuf> {
    let output = Command::new(LOCATE_COMMAND).arg(binary).output().ok()?;
    let output_text = str::from_utf8(&output.stdout).ok()?;
    let bin = output_text.trim().lines().next()?;

    if binary_ran_ok(bin) {
        return Some(PathBuf::from(bin));
    }

    None
}

fn binary_ran_ok<S: AsRef<OsStr>>(path: S) -> bool {
    Command::new(path)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

#[test]
fn shell_status_success() {
    let dir = Path::new(".");
    #[cfg(windows)]
    let status = status("cmd", dir, &["/C", "exit", "0"]);
    #[cfg(not(windows))]
    let status = status("sh", dir, &["-c", "exit 0"]);

    assert_eq!(status.unwrap(), 0);
}

#[test]
fn shell_status_nonzero_exit() {
    let dir = Path::new(".");
    #[cfg(windows)]
    let status = status("cmd", dir, &["/C", "exit", "7"]);
    #[cfg(not(windows))]
    let status = status("sh", dir, &["-c", "exit 7"]);

    assert_eq!(status.unwrap(), 7);
}

#[test]
fn find_existing_binary() {
    let bin = find_binary("cargo");
    assert!(bin.is_some())
}

#[test]
fn find_missing_binary() {
    let bin = find_binary("cirup_binary_that_should_not_exist_49a85f");
    assert!(bin.is_none())
}
