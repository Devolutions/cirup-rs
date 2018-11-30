extern crate find_git;

use std::error::Error;
use std::path::Path;

use ::config;
use ::shell;

const DEFAULT_BRANCH: &str = "master";

// TODO Implement `push` (possibly setting git username and email)

pub struct Vcs {
    _name: String,
    executable: String,
    local_path: String,
    remote_path: String,
}

fn is_git_repo(path: &str) -> bool {
    let path = Path::new(path);
    path.is_dir() && path.join(".git").exists()
}

impl Vcs {
    pub fn new(config: &config::Config) -> Result<Self, Box<Error>> {
        match config.vcs.plugin.as_ref() {
            "git" => { 
                match find_git::git_path() {
                    Some(p) => {
                        Ok(Vcs {
                            _name: "git".to_string(),
                            executable: p.to_str().unwrap().to_string(),
                            local_path: config.vcs.local_path.to_string(),
                            remote_path: config.vcs.remote_path.to_string(),
                        })
                    },
                    None => { Err("cannot find git binary")? }
                }
            },
            _ => { Err("unknown vcs plugin")? }
        }
    }

    fn run(&self, args: &[&str]) -> Result<(), Box<Error>> {
        shell::status(&self.executable, Path::new(&self.local_path), args)?; 

        Ok(())
    }

    fn init_repo(&self) -> Result<(), Box<Error>> {
        if !is_git_repo(&self.local_path) {
            self.run(&["clone", &self.remote_path, "--branch", DEFAULT_BRANCH, &self.local_path])?;
        } else {
            let git_path = Path::new(&self.local_path).join("git");
            if git_path.join("rebase-merge").exists() || git_path.join("rebase-apply").exists() {
                self.run(&["rebase", "--abort"])?;
            }
        }

        Ok(())
    }

    pub fn pull(&self) -> Result<(), Box<Error>> {
        self.init_repo()?;

        let repo = format!("origin/{}", DEFAULT_BRANCH);

        // Abandon any local changes
        self.run(&["reset", "--hard", &repo])?;

        // Remove old conflicting branches
        self.run(&["remote", "prune", "origin"])?;

        // Pull changes from remote
        self.run(&["fetch"])?;

        // Reset to remote state
        self.run(&["reset", "--hard", &repo])?;

        Ok(())
    }

    pub fn log(
        &self, 
        filespec: &str, 
        format: Option<&str>, 
        old_commit: Option<&str>, 
        new_commit: Option<&str>, 
        inclusive: bool) 
        -> Result<(), Box<Error>> {
        let format = format!("--pretty={}", if format.is_none() { "oneline" } else { format.unwrap() });
        let commit : String;

        if old_commit.is_some() {
            commit = format!("{}{}..{}", 
                old_commit.unwrap(), 
                if inclusive { "^" } else { "" }, 
                if new_commit.is_some() { new_commit.unwrap() } else { "HEAD" });
        } else {
            commit = "HEAD".to_string();
        }

        return self.run(&["log", &format, &commit, filespec]);
    }

    pub fn diff(
        &self, 
        filespec: &str, 
        old_commit: &str, 
        new_commit: Option<&str>) 
        -> Result<(), Box<Error>> {
        let mut args: Vec<&str> = Vec::new();
        args.push("diff");
        args.push(old_commit);

        if new_commit.is_some() {
            args.push(new_commit.unwrap());
        }

        args.push(filespec);

        return self.run(&args);
    }

    pub fn show(&self, filespec: &str, commit: Option<&str>, output: &str) -> Result<(), Box<Error>> {
        let show = format!("{}:{}", if commit.is_none() { "HEAD" } else { commit.unwrap() }, filespec);
        shell::output_to_file(&self.executable, Path::new(&self.local_path), &["show", &show], Path::new(output))?;

        Ok(())
    }
}