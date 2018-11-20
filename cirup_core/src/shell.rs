use std::process::Command;
use std::path::Path;
use std::fs::OpenOptions;
use std::io::prelude::*;

pub fn status(exe: &str, dir: &Path, args: &[&str]) -> i32 {
    let status = Command::new(exe)
        .current_dir(dir)
        .args(args)
        .status()
        .expect("failed to run command");

    match status.code() {
        Some(_n) => _n,
        None => -1 // Terminated by signal (unix)
    }
}

pub fn output(exe: &str, dir: &Path, args: &[&str]) -> String {
    let output = Command::new(exe)
        .current_dir(dir)
        .args(args)
        .output()
        .expect("failed to run command");

    println!(">>> {:?}", output);

    // todo GIT success == 0

    String::from_utf8_lossy(&output.stdout).to_string()
}

pub fn output_to_file(exe: &str, dir: &Path, args: &[&str], out: &Path) -> i32 {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(out)
        .expect("couldn't open output file for writing");

    let output = output(exe, dir, args);

    println!(">>> {}", output);

    file.write_all(output.as_bytes()).expect("couldn't write to output file");
    
    1
}

#[test]
fn shell_status() {
    let dir = Path::new(".");
    let status = status("ls", &dir, &[ "-l" ]);
    assert_eq!(status, 0)
}