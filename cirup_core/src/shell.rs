use std::process::Command;
use std::path::Path;

// TODO Redirect output

pub fn run(exe: &str, dir: &Path, args: &[&str]) -> i32 {
    let command = Command::new(exe)
        .current_dir(dir)
        .args(args)
        .status()
        .expect("failed to run command");

    // println!("stdout: {}", String::from_utf8_lossy(&command.stdout));
    // println!("stderr: {}", String::from_utf8_lossy(&command.stderr));

    match command.code() {
        Some(_n) => 1,
        None => -1 // terminated by signal (unix)
    }
}

#[test]
fn shell_run() {
    let dir = Path::new(".");
    let status = run("ls", &dir, &[ "-l" ]);
    assert_eq!(status, 1)
}