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

pub fn status(exe: &str, dir: &Path, args: &[&str]) -> Result<i32, Box<Error>> {
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

pub fn output(exe: &str, dir: &Path, args: &[&str]) -> Result<String, Box<Error>> {
    let output = Command::new(exe).current_dir(dir).args(args).output()?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn output_to_file(exe: &str, dir: &Path, args: &[&str], out: &Path) -> Result<(), Box<Error>> {
    let mut file = OpenOptions::new().write(true).create(true).open(out)?;

    let output = output(exe, dir, args)?;

    file.write_all(output.as_bytes())?;

    Ok(())
}

pub fn find_binary(binary: &str) -> Option<::std::path::PathBuf> {
    if let Ok(output) = Command::new(LOCATE_COMMAND).arg(binary).output() {
        let bin = str::from_utf8(&output.stdout)
            .expect(&format!(
                "non-UTF8 output when running `{} {}`",
                LOCATE_COMMAND, binary
            ))
            .trim()
            .lines()
            .next()
            .expect(&format!(
                "should have had at least one line of text when running `{} {}`",
                LOCATE_COMMAND, binary
            ));
        if binary_ran_ok(&bin) {
            return Some(PathBuf::from(bin));
        }
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
fn shell_status() {
    // TODO This test is not cross-platform
    let dir = Path::new(".");
    let status = status("ls", &dir, &["-l"]);
    assert!(status.is_ok())
}

#[test]
fn find_svn() {
    // TODO This test is not cross-platform
    let bin = find_binary("svn");
    assert!(bin.is_some())
}
