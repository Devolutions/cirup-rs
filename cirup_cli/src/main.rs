use std::error::Error;
use std::process::ExitCode;

use clap::{ArgAction, Parser, Subcommand};
use env_logger::{Builder, Env};
use log::error;

use cirup_core::query;

#[derive(Debug, Parser)]
#[command(name = "cirup", author, version, about = "a translation continuous integration tool")]
struct Cli {
    #[arg(short = 'v', long = "verbose", global = true, action = ArgAction::Count, help = "Sets the level of verbosity")]
    verbose: u8,

    #[arg(short = 'C', long = "show-changes", global = true, action = ArgAction::SetTrue, help = "additionally print keys that have values in [file2] but that do not match the values in [file1]")]
    show_changes: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
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

fn run(cli: &Cli) -> Result<(), Box<dyn Error>> {
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

    match run(&cli) {
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
    fn parse_diff_with_base() {
        let cli = Cli::parse_from(["cirup", "diff-with-base", "old.json", "new.json", "base.json"]);

        match cli.command {
            Commands::DiffWithBase { old, new, base } => {
                assert_eq!(old, "old.json");
                assert_eq!(new, "new.json");
                assert_eq!(base, "base.json");
            }
            _ => panic!("expected diff-with-base command"),
        }
    }
}
