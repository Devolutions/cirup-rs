use std::error::Error;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::Path;
use std::process::Command;

pub fn status(exe: &str, dir: &Path, args: &[&str]) -> Result<i32, Box<Error>> {
    let status = Command::new(exe)
        .current_dir(dir)
        .args(args)
        .status()?;

    match status.code() {
        Some(c) => Ok(c),
        None => Err("process terminated by signal")?
    }
}

pub fn output(exe: &str, dir: &Path, args: &[&str]) -> Result<String, Box<Error>> {
    let output = Command::new(exe)
        .current_dir(dir)
        .args(args)
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn output_to_file(exe: &str, dir: &Path, args: &[&str], out: &Path) -> Result<(), Box<Error>> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(out)?;

    let output = output(exe, dir, args)?;

    file.write_all(output.as_bytes())?;

    Ok(())
}

#[test]
fn shell_status() {
    // TODO This test is not cross-platform
    let dir = Path::new(".");
    let status = status("ls", &dir, &[ "-l" ]);
    assert!(status.is_ok())
}