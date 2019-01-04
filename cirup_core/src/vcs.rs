extern crate find_git;

use std::boxed::Box;
use std::error::Error;
use std::path::Path;

use ::config;
use ::shell;

const DEFAULT_BRANCH: &str = "master";

pub trait Vcs {
    fn pull(&self) -> Result<(), Box<Error>>;
    fn push(&self) -> Result<(), Box<Error>>;
    fn log(
        &self, 
        filespec: &str, 
        format: Option<&str>, 
        old_commit: Option<&str>, 
        new_commit: Option<&str>, 
        inclusive: bool) 
        -> Result<(), Box<Error>>;
    fn diff(
        &self, 
        filespec: &str, 
        old_commit: &str, 
        new_commit: Option<&str>) 
        -> Result<(), Box<Error>>;
    fn show(
        &self, 
        filespec: &str, 
        commit: Option<&str>, 
        output: &str) 
        -> Result<(), Box<Error>>;
}

pub mod vcs_type {
    pub const GIT: &'static str = "git";
    pub const SVN: &'static str = "svn";
}

struct VcsMetadata {
    name: String,
    executable: String,
    local_path: String,
    remote_path: String,
}

impl Default for VcsMetadata {
    fn default() -> VcsMetadata {
        VcsMetadata {
            name: String::new(),
            executable: String::new(),
            local_path: String::new(),
            remote_path: String::new(),
        }
    }
}

impl VcsMetadata {
    fn run(&self, args: &[&str]) -> Result<(), Box<Error>> {
        shell::status(&self.executable, Path::new(&self.local_path), args)?; 

        Ok(())
    }
}

pub fn new(config: &config::Config) -> Result<Box<Vcs>, Box<Error>> {
    let mut meta = VcsMetadata { 
        local_path: config.vcs.local_path.to_string(), 
        remote_path: config.vcs.remote_path.to_string(),
        ..Default::default() };

    match config.vcs.plugin.as_ref() {
                vcs_type::GIT => { 
                    match find_git::git_path() {
                        Some(p) => {
                            meta.name = vcs_type::GIT.to_string();
                            meta.executable = p.to_str().unwrap().to_string();
                            Ok(Box::new(Git { meta: meta }))
                        },
                        None => { Err("cannot find git binary")? }
                    }
                },
                vcs_type::SVN => { 
                    match shell::find_binary("svn") {
                        Some(p) => {
                            meta.name = vcs_type::SVN.to_string();
                            meta.executable = p.to_str().unwrap().to_string();
                            Ok(Box::new(Svn { meta: meta }))
                        },
                        None => { Err("cannot find svn binary")? }
                    }
                },
                _ => { Err("unknown vcs plugin")? }
            }
}

pub struct Git {
    meta: VcsMetadata,
}

fn is_git_repo(path: &str) -> bool {
    let path = Path::new(path);
    path.is_dir() && path.join(".git").exists()
}

impl Git {
    fn init_repo(&self) -> Result<(), Box<Error>> {
        if !is_git_repo(&self.meta.local_path) {
            self.meta.run(&["clone", &self.meta.remote_path, "--branch", DEFAULT_BRANCH, &self.meta.local_path])?;
        } else {
            let git_path = Path::new(&self.meta.local_path).join("git");
            if git_path.join("rebase-merge").exists() || git_path.join("rebase-apply").exists() {
                self.meta.run(&["rebase", "--abort"])?;
            }
        }

        Ok(())
    }
}

impl Vcs for Git {
    fn pull(&self) -> Result<(), Box<Error>> {
        self.init_repo()?;

        let repo = format!("origin/{}", DEFAULT_BRANCH);

        // Abandon any local changes
        self.meta.run(&["reset", "--hard", &repo])?;

        // Remove old conflicting branches
        self.meta.run(&["remote", "prune", "origin"])?;

        // Pull changes from remote
        self.meta.run(&["fetch"])?;

        // Reset to remote state
        self.meta.run(&["reset", "--hard", &repo])?;

        Ok(())
    }

    fn push(&self) -> Result<(), Box<Error>> {
        unimplemented!();
    }

    fn log(
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

        return self.meta.run(&["log", &format, &commit, filespec]);
    }

    fn diff(
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

        return self.meta.run(&args);
    }

    fn show(&self, filespec: &str, commit: Option<&str>, output: &str) -> Result<(), Box<Error>> {
        let show = format!("{}:{}", if commit.is_none() { "HEAD" } else { commit.unwrap() }, filespec);
        shell::output_to_file(&self.meta.executable, Path::new(&self.meta.local_path), &["show", &show], Path::new(output))?;

        Ok(())
    }
}

pub struct Svn {
    meta: VcsMetadata,
}

impl Svn {
    fn resolve(&self, mine: bool) -> Result<(), Box<Error>> {
        let strategy = if mine { "mine-full" } else { "theirs-full" };
        self.meta.run(&["resolve", &self.meta.local_path, "--accept", strategy, "--recursive", "--non-interactive"])?;

        Ok(())
    }
}

impl Vcs for Svn {
    fn pull(&self) -> Result<(), Box<Error>> {
        self.meta.run(&["co", &self.meta.remote_path, &self.meta.local_path, "--force", "--non-interactive"])?;
        self.resolve(false)?;

        Ok(())
    }

    fn push(&self) -> Result<(), Box<Error>> {
        unimplemented!();
    }

    fn log(
        &self, 
        filespec: &str, 
        _format: Option<&str>, 
        old_commit: Option<&str>, 
        new_commit: Option<&str>, 
        _inclusive: bool) 
        -> Result<(), Box<Error>> {

        let commit = format!("{}:{}", 
                if old_commit.is_some() { old_commit.unwrap() } else { "1" },
                if new_commit.is_some() { new_commit.unwrap() } else { "HEAD" });

        return self.meta.run(&["log", "--revision", &commit, filespec]);
    }

    fn diff(
        &self, 
        filespec: &str, 
        old_commit: &str, 
        new_commit: Option<&str>) 
        -> Result<(), Box<Error>> {
        let commit = format!("{}:{}", 
            old_commit, 
            if new_commit.is_some() { new_commit.unwrap() } else { "HEAD" });

        return self.meta.run(&["diff", "--revision", &commit, filespec]);
    }

    fn show(&self, filespec: &str, commit: Option<&str>, output: &str) -> Result<(), Box<Error>> {
        let mut args: Vec<&str> = Vec::new();
        args.push("cat");
        args.push(filespec);

        if commit.is_some() {
            args.push("--revision");
            args.push(commit.unwrap());
        }
        
        shell::output_to_file(&self.meta.executable, Path::new(&self.meta.local_path), &args, Path::new(output))?;

        Ok(())
    }
}