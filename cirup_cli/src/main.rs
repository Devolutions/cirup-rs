use std::error::Error;
use std::path::Path;
use std::process::ExitCode;

use clap::{ArgAction, Parser, Subcommand};
use env_logger::{Builder, Env};
use log::error;

use cirup_core::config::Config;
use cirup_core::query;
use cirup_core::sync::Sync;

#[derive(Debug, Parser)]
#[command(name = "cirup", author, version, about = "a translation continuous integration tool")]
struct Cli {
    #[arg(short = 'v', long = "verbose", global = true, action = ArgAction::Count, help = "Sets the level of verbosity")]
    verbose: u8,

    #[arg(
        short = 'c',
        long = "config",
        global = true,
        help = "Sets the configuration file to use"
    )]
    config: Option<String>,

    #[arg(
        short = 'o',
        long = "old-commit",
        global = true,
        help = "a git hash specifying the old commit"
    )]
    old_commit: Option<String>,

    #[arg(
        short = 'n',
        long = "new-commit",
        global = true,
        requires = "old_commit",
        help = "a git hash specifying the new commit"
    )]
    new_commit: Option<String>,

    #[arg(short = 'C', long = "show-changes", global = true, action = ArgAction::SetTrue, help = "additionally print keys that have values in [file2] but that do not match the values in [file1]")]
    show_changes: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(about = "generate translations for all languages into working_dir. [config] is required.")]
    Pull,

    #[command(about = "merge translations from working_dir back into source control. [config] is required.")]
    Push,

    #[command(
        name = "vcs-log",
        about = "show the version control history of the source language, newest first. [config] is required."
    )]
    VcsLog {
        #[arg(
            short = 'l',
            long = "limit",
            default_value_t = 0,
            help = "limit the number of results returned"
        )]
        limit: u32,

        #[arg(long = "format", help = "optional log output format")]
        format: Option<String>,
    },

    #[command(
        name = "vcs-diff",
        about = "diff two commits of the source language. [config] is required. [old-commit] is required."
    )]
    VcsDiff,

    #[command(name = "file-print", about = "read [file] and output its contents")]
    FilePrint { file: String, output: Option<String> },

    #[command(
        name = "file-convert",
        about = "convert [file] to another type. possible extensions are .json, .resx and .restext"
    )]
    FileConvert { file: String, output: String },

    #[command(
        name = "file-sort",
        about = "sort [file] by key name. possible extensions are .json, .resx and .restext"
    )]
    FileSort { file: String, output: Option<String> },

    #[command(
        name = "file-diff",
        about = "output keys that have values in [file1] but not in [file2]. useful for finding missing translations."
    )]
    FileDiff {
        file1: String,
        file2: String,
        output: Option<String>,
    },

    #[command(name = "file-merge", about = "merges the values from [file2] into [file1]")]
    FileMerge {
        file1: String,
        file2: String,
        output: Option<String>,
    },

    #[command(
        name = "file-intersect",
        about = "output the intersection of values from [file1] and [file2]"
    )]
    FileIntersect {
        file1: String,
        file2: String,
        output: Option<String>,
    },

    #[command(
        name = "file-subtract",
        about = "outputs values from [file1] that do not exist in [file2]"
    )]
    FileSubtract {
        file1: String,
        file2: String,
        output: Option<String>,
    },

    #[command(
        name = "diff-with-base",
        about = "output keys that have values in [new] but not in [old] with the value in [base]"
    )]
    DiffWithBase { old: String, new: String, base: String },
}

fn print(input: &str, out_file: Option<&str>) {
    let query = query::query_print(input);
    query.run_interactive(out_file);
}

fn diff(file_one: &str, file_two: &str, out_file: Option<&str>) {
    let query = query::query_diff(file_one, file_two);
    query.run_interactive(out_file);
}

fn change(file_one: &str, file_two: &str, out_file: Option<&str>) {
    let query = query::query_change(file_one, file_two);
    query.run_interactive(out_file);
}

fn merge(file_one: &str, file_two: &str, out_file: Option<&str>) {
    let query = query::query_merge(file_one, file_two);
    query.run_interactive(out_file);
}

fn intersect(file_one: &str, file_two: &str, out_file: Option<&str>) {
    let query = query::query_intersect(file_one, file_two);
    query.run_interactive(out_file);
}

fn subtract(file_one: &str, file_two: &str, out_file: Option<&str>) {
    let query = query::query_subtract(file_one, file_two);
    query.run_interactive(out_file);
}

fn convert(file_one: &str, out_file: &str) {
    let query = query::query_convert(file_one);
    query.run_interactive(Some(out_file));
}

fn sort(file_one: &str, out_file: Option<&str>) {
    let query = query::query_sort(file_one);

    if out_file.is_some() {
        query.run_interactive(out_file);
    } else {
        query.run_interactive(Some(file_one));
    }
}

fn diff_with_base(old: &str, new: &str, base: &str) {
    let query = query::query_diff_with_base(old, new, base);
    query.run_triple_interactive();
}

fn run(cli: &Cli, config: Option<Config>) -> Result<(), Box<dyn Error>> {
    match &cli.command {
        Commands::FilePrint { file, output } => {
            print(file, output.as_deref());
            Ok(())
        }
        Commands::FileDiff { file1, file2, output } => {
            if cli.show_changes {
                change(file1, file2, output.as_deref());
            } else {
                diff(file1, file2, output.as_deref());
            }
            Ok(())
        }
        Commands::FileMerge { file1, file2, output } => {
            merge(file1, file2, output.as_deref());
            Ok(())
        }
        Commands::FileIntersect { file1, file2, output } => {
            intersect(file1, file2, output.as_deref());
            Ok(())
        }
        Commands::FileSubtract { file1, file2, output } => {
            subtract(file1, file2, output.as_deref());
            Ok(())
        }
        Commands::FileConvert { file, output } => {
            convert(file, output);
            Ok(())
        }
        Commands::FileSort { file, output } => {
            sort(file, output.as_deref());
            Ok(())
        }
        Commands::VcsLog { limit, format } => match config {
            Some(c) => {
                let sync = Sync::new(&c)?;

                sync.vcs.log(
                    &sync.source_language_path().to_string_lossy(),
                    format.as_deref(),
                    cli.old_commit.as_deref(),
                    cli.new_commit.as_deref(),
                    true,
                    *limit,
                )?;

                Ok(())
            }
            None => Err("configuration file required")?,
        },
        Commands::VcsDiff => match config {
            Some(c) => {
                let sync = Sync::new(&c)?;
                let old_commit = match cli.old_commit.as_deref() {
                    Some(value) => value,
                    None => Err("old commit required")?,
                };

                sync.vcs.diff(
                    &sync.source_language_path().to_string_lossy(),
                    old_commit,
                    cli.new_commit.as_deref(),
                )?;

                Ok(())
            }
            None => Err("configuration file required")?,
        },
        Commands::Pull => match config {
            Some(c) => {
                let sync = Sync::new(&c)?;
                sync.pull(cli.old_commit.as_deref(), cli.new_commit.as_deref(), cli.show_changes)?;

                Ok(())
            }
            None => Err("configuration file required")?,
        },
        Commands::Push => match config {
            Some(c) => {
                let sync = Sync::new(&c)?;
                sync.push(cli.old_commit.as_deref(), cli.new_commit.as_deref())?;

                Ok(())
            }
            None => Err("configuration file required")?,
        },
        Commands::DiffWithBase { old, new, base } => {
            diff_with_base(old, new, base);
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let min_log_level = match cli.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    let mut builder = Builder::from_env(Env::default().default_filter_or(min_log_level));
    builder.init();

    let mut config: Option<Config> = None;

    if let Some(config_file) = cli.config.as_deref() {
        match Config::new(Path::new(config_file)) {
            Ok(c) => config = Some(c),
            Err(e) => {
                error!("unable to read the config file ({})", e);
                return ExitCode::FAILURE;
            }
        }
    }

    match run(&cli, config) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("an unexpected error occured ({})", e);
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_file_diff_with_show_changes() {
        let cli = Cli::parse_from(["cirup", "--show-changes", "file-diff", "a.json", "b.json", "out.json"]);

        assert!(cli.show_changes);
        match cli.command {
            Commands::FileDiff { file1, file2, output } => {
                assert_eq!(file1, "a.json");
                assert_eq!(file2, "b.json");
                assert_eq!(output.as_deref(), Some("out.json"));
            }
            _ => panic!("expected file-diff command"),
        }
    }

    #[test]
    fn parse_vcs_log_limit() {
        let cli = Cli::parse_from(["cirup", "--old-commit", "abc", "vcs-log", "--limit", "12"]);

        assert_eq!(cli.old_commit.as_deref(), Some("abc"));
        match cli.command {
            Commands::VcsLog { limit, format } => {
                assert_eq!(limit, 12);
                assert!(format.is_none());
            }
            _ => panic!("expected vcs-log command"),
        }
    }
}
