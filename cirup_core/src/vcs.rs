extern crate find_git;

use std::path::Path;

use ::config;
use ::shell;

const DEFAULT_BRANCH: &str = "master";

pub struct Vcs {
    name: String,
    executable: String,
    local_path: String,
    remote_path: String,
}

fn is_git_repo(path: &str) -> bool {
    let path = Path::new(path);
    path.is_dir() && path.join(".git").exists()
}

impl Vcs {
    pub fn new(config: &config::Config) -> Vcs {
        match config.vcs.plugin.as_ref() {
            "git" => { 
                let git_path = find_git::git_path().expect("cannot find git");
                Vcs { 
                    name: "git".to_string(),
                    executable: git_path.to_string_lossy().to_string(),
                    local_path: config.vcs.local_path.to_string(),
                    remote_path: config.vcs.remote_path.to_string(),
                }
            },
            _ => { panic!("unknown vcs plugin") }
        }
    }

    fn run(&self, args: &[&str]) {
        shell::status(&self.executable, Path::new(&self.local_path), args);
    }

    fn init_repo(&self) {
        if !is_git_repo(&self.local_path) {
            self.run(&["clone", &self.remote_path, "--branch", DEFAULT_BRANCH, &self.local_path]);
        } else {
            let git_path = Path::new(&self.local_path).join("git");
            if git_path.join("rebase-merge").exists() || git_path.join("rebase-apply").exists() {
                self.run(&["rebase", "--abort"]);
            }
        }
    }

    pub fn pull(&self) {
        self.init_repo();

        let repo = format!("origin/{}", DEFAULT_BRANCH);

        // Abandon any local changes
        self.run(&["reset", "--hard", &repo]);

        // Remove old conflicting branches
        self.run(&["remote", "prune", "origin"]);

        // Pull changes from remote
        self.run(&["fetch"]);

        // Reset to remote state
        self.run(&["reset", "--hard", &repo]);
    }

    pub fn log(&self, filespec: &str) {
        self.log_pretty(filespec, "oneline");
    }

    pub fn log_pretty(&self, filespec: &str, format: &str) {
        let pretty = format!("--pretty={}", format);
        self.run(&["log", &pretty, filespec]);
    }

    pub fn log_pretty_since(&self, filespec: &str, format: &str, since_commit: &str, inclusive: bool) {
        let new = format!("{}{}..HEAD", since_commit, if inclusive { "^" } else { "" });
        let pretty = format!("--pretty={}", format);
        self.run(&["log", &pretty, &new, filespec]);
    }

    pub fn diff(&self, filespec: &str, old_commit: &str) {
        self.run(&["diff", old_commit, filespec]);
    }

    pub fn diff_commits(&self, filespec: &str, old_commit: &str, new_commit: &str) {
        self.run(&["diff", old_commit, new_commit, filespec]);
    }

    pub fn show(&self, filespec: &str, commit: &str, output: &str) {
        let show = format!("{}:{}", commit, filespec);
        shell::output_to_file(&self.executable, Path::new(&self.local_path), &["show", &show], Path::new(output));
    }
}