extern crate find_git;

use std::boxed::Box;
use std::error::Error;
use std::path::Path;

use chrono::*;
use regex::Regex;

use config;
use shell;

const DEFAULT_BRANCH: &str = "master";

pub trait Vcs {
    fn init_repo(&self) -> Result<(), Box<Error>>;
    fn current_revision(&self) -> Result<String, Box<Error>>;
    fn pull(&self) -> Result<(), Box<Error>>;
    fn push(&self) -> Result<(), Box<Error>>;
    fn log(
        &self,
        filespec: &str,
        format: Option<&str>,
        old_commit: Option<&str>,
        new_commit: Option<&str>,
        inclusive: bool,
        limit: u32,
    ) -> Result<(), Box<Error>>;
    fn diff(
        &self,
        filespec: &str,
        old_commit: &str,
        new_commit: Option<&str>,
    ) -> Result<(), Box<Error>>;
    fn show(&self, filespec: &str, commit: Option<&str>, output: &str) -> Result<(), Box<Error>>;
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

    fn output(&self, args: &[&str]) -> Result<String, Box<Error>> {
        shell::output(&self.executable, Path::new(&self.local_path), args)
    }

    fn is_repo(&self) -> bool {
        let path = Path::new(&self.local_path);
        path.is_dir() && path.join(format!(".{}", self.name)).exists()
    }
}

pub fn new(config: &config::Config) -> Result<Box<Vcs>, Box<Error>> {
    let mut meta = VcsMetadata {
        local_path: config.vcs.local_path.to_string(),
        remote_path: config.vcs.remote_path.to_string(),
        ..Default::default()
    };

    match config.vcs.plugin.as_ref() {
        vcs_type::GIT => match find_git::git_path() {
            Some(p) => {
                meta.name = vcs_type::GIT.to_string();
                meta.executable = p.to_str().unwrap().to_string();
                Ok(Box::new(Git { meta: meta }))
            }
            None => Err("cannot find git binary")?,
        },
        vcs_type::SVN => match shell::find_binary("svn") {
            Some(p) => {
                meta.name = vcs_type::SVN.to_string();
                meta.executable = p.to_str().unwrap().to_string();
                Ok(Box::new(Svn { meta: meta }))
            }
            None => Err("cannot find svn binary")?,
        },
        _ => Err("unknown vcs plugin")?,
    }
}

pub struct Git {
    meta: VcsMetadata,
}

impl Vcs for Git {
    fn init_repo(&self) -> Result<(), Box<Error>> {
        if !self.meta.is_repo() {
            info!(
                "{} does not appear to be a git repository. Cloning...",
                self.meta.local_path
            );
            self.meta.run(&[
                "clone",
                &self.meta.remote_path,
                "--branch",
                DEFAULT_BRANCH,
                &self.meta.local_path,
            ])?;
        } else {
            let git_path = Path::new(&self.meta.local_path).join(".git");
            if git_path.join("rebase-merge").exists() || git_path.join("rebase-apply").exists() {
                Err(format!(
                    "{} appears to have a pending rebase",
                    self.meta.local_path
                ))?;
            }
        }

        Ok(())
    }

    fn current_revision(&self) -> Result<String, Box<Error>> {
        self.meta.output(&["rev-parse", "--short", "HEAD"])
    }

    fn pull(&self) -> Result<(), Box<Error>> {
        self.init_repo()?;

        debug!("vcs pull start");

        // Pull changes from remote
        self.meta.run(&["fetch"])?;

        // Error out if there are conflicts
        match self.meta.run(&["merge", "--ff-only"]) {
            Ok(_) => (),
            Err(_) => Err(format!(
                "{} appears to have conflicts",
                self.meta.local_path
            ))?,
        }

        debug!("vcs pull end");

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
        inclusive: bool,
        limit: u32
    ) -> Result<(), Box<Error>> {
        let format = format!("--pretty=format:%h - %aI - %an - %s");
        let commit: String;

        if old_commit.is_some() {
            commit = format!(
                "{}{}..{}",
                old_commit.unwrap(),
                if inclusive { "^" } else { "" },
                if new_commit.is_some() {
                    new_commit.unwrap()
                } else {
                    "HEAD"
                }
            );
        } else {
            commit = "HEAD".to_string();
        }

        let limit_arg = limit.to_string();
        let mut args = vec!["log", &format];

        if limit > 0 {
            args.push("--max-count");
            args.push(&limit_arg);
        };

        args.push(&commit);
        args.push(filespec);

        match self.meta.output(&args) {
            Ok(output) => {
                println!("{}", output);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn diff(
        &self,
        filespec: &str,
        old_commit: &str,
        new_commit: Option<&str>,
    ) -> Result<(), Box<Error>> {
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
        let show = format!(
            "{}:{}",
            if commit.is_none() {
                "HEAD"
            } else {
                commit.unwrap()
            },
            filespec
        );
        shell::output_to_file(
            &self.meta.executable,
            Path::new(&self.meta.local_path),
            &["show", &show],
            Path::new(output),
        )?;

        Ok(())
    }
}

pub struct Svn {
    meta: VcsMetadata,
}

#[derive(Debug, Deserialize, PartialEq)]
struct LogEntry {
    revision: i32,
    author: String,
    date: String,
    msg: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Log {
    #[serde(rename = "logentry")]
    entries: Vec<LogEntry>,
}

impl LogEntry {
    fn iso_formatted_date(&self) -> Result<String, Box<Error>> {
        let dt = Utc.datetime_from_str(self.date.trim_end_matches("Z"), "%Y-%m-%dT%H:%M:%S%.6f")?;
        return Ok(dt.to_rfc3339_opts(SecondsFormat::Secs, false))
    }
}

#[test]
fn iso_formatted_date_test() {
    let log = LogEntry {
        revision: 0,
        author: "0".to_string(),
        msg: "0".to_string(),
        date: "2017-12-23T15:51:26.982890Z".to_string()
    };
    assert_eq!(log.iso_formatted_date().unwrap(), "2017-12-23T15:51:26+00:00".to_string());
}

impl Svn {
    fn status_has_conflicts(status: &str) -> Result<bool, Box<Error>> {
        Ok(Regex::new(r"(?m)^C")?.is_match(status))
    }
}

#[test]
fn test_status_conflicts() {
    assert_eq!(
        false,
        Svn::status_has_conflicts(
            "\
--- Merging r6429 through r6736 into '.':
U    UIResources.it.resx
U    MsgResources.pl.resx"
        )
        .unwrap()
    );
    assert_eq!(
        true,
        Svn::status_has_conflicts(
            "\
--- Merging r6429 through r6736 into '.':
U    UIResources.it.resx
C    MsgResources.pl.resx"
        )
        .unwrap()
    );
}

impl Vcs for Svn {
    fn init_repo(&self) -> Result<(), Box<Error>> {
        if !self.meta.is_repo() {
            info!(
                "{} does not appear to be a svn repository. Checking out...",
                self.meta.local_path
            );
            self.meta.run(&[
                "co",
                &self.meta.remote_path,
                &self.meta.local_path,
                "--non-interactive",
            ])?;
        }

        Ok(())
    }

    fn current_revision(&self) -> Result<String, Box<Error>> {
        self.meta.output(&["info", "--show-item", "revision"])
    }

    fn pull(&self) -> Result<(), Box<Error>> {
        self.init_repo()?;

        debug!("vcs pull start");

        match shell::output(
            &self.meta.executable,
            Path::new(&self.meta.local_path),
            &["merge", "--dry-run", "-r", "BASE:HEAD", "."],
        ) {
            Ok(status) => match Svn::status_has_conflicts(&status) {
                Ok(false) => {
                    self.meta.run(&["update"])?;
                }
                _ => warn!(
                    "updating repository {} may result in conflicts, skipping update",
                    self.meta.local_path
                ),
            },
            Err(_) => warn!("failed to update repository {}", self.meta.local_path),
        }

        debug!("vcs pull end");

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
        _inclusive: bool,
        limit: u32
    ) -> Result<(), Box<Error>> {
        let commit = format!(
            "{}:{}",
            if new_commit.is_some() {
                new_commit.unwrap()
            } else {
                "HEAD"
            },
            if old_commit.is_some() {
                old_commit.unwrap()
            } else {
                "1"
            },
        );

        let limit_arg = limit.to_string();
        let mut args = vec!["log", "--revision", &commit, "--xml", filespec];

        if limit > 0 {
            args.push("--limit");
            args.push(&limit_arg);
        };

        let xml = self
            .meta
            .output(&args)?;
        let log: Log = serde_xml_rs::de::from_str(&xml)?;

        for entry in &log.entries {
            println!(
                "{} - {} - {} - {}",
                entry.revision,
                entry.iso_formatted_date()?,
                entry.author,
                entry.msg.lines().nth(0).unwrap()
            );
        }

        Ok(())
    }

    fn diff(
        &self,
        filespec: &str,
        old_commit: &str,
        new_commit: Option<&str>,
    ) -> Result<(), Box<Error>> {
        let commit = format!(
            "{}:{}",
            old_commit,
            if new_commit.is_some() {
                new_commit.unwrap()
            } else {
                "HEAD"
            }
        );

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

        shell::output_to_file(
            &self.meta.executable,
            Path::new(&self.meta.local_path),
            &args,
            Path::new(output),
        )?;

        Ok(())
    }
}
