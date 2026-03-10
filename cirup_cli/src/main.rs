use std::error::Error;
use std::process::ExitCode;

use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use env_logger::{Builder, Env};
use log::error;

use cirup_core::{OutputEncoding, query};

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
enum CliOutputFormat {
    Table,
    Json,
    Jsonl,
}

impl From<CliOutputFormat> for query::QueryOutputFormat {
    fn from(value: CliOutputFormat) -> Self {
        match value {
            CliOutputFormat::Table => query::QueryOutputFormat::Table,
            CliOutputFormat::Json => query::QueryOutputFormat::Json,
            CliOutputFormat::Jsonl => query::QueryOutputFormat::Jsonl,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
enum CliLogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl CliLogLevel {
    fn as_filter(self) -> &'static str {
        match self {
            CliLogLevel::Error => "error",
            CliLogLevel::Warn => "warn",
            CliLogLevel::Info => "info",
            CliLogLevel::Debug => "debug",
            CliLogLevel::Trace => "trace",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
enum CliOutputEncoding {
    #[value(name = "utf8-no-bom")]
    Utf8NoBom,
    #[value(name = "utf8-bom")]
    Utf8Bom,
    #[value(name = "utf8")]
    Utf8,
}

impl From<CliOutputEncoding> for OutputEncoding {
    fn from(value: CliOutputEncoding) -> Self {
        match value {
            CliOutputEncoding::Utf8NoBom | CliOutputEncoding::Utf8 => OutputEncoding::Utf8NoBom,
            CliOutputEncoding::Utf8Bom => OutputEncoding::Utf8Bom,
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "cirup", author, version, about = "a translation continuous integration tool")]
struct Cli {
    #[arg(short = 'v', long = "verbose", global = true, action = ArgAction::Count, help = "Sets the level of verbosity")]
    verbose: u8,

    #[arg(long = "quiet", global = true, action = ArgAction::SetTrue, conflicts_with_all = ["verbose", "log_level"], help = "only print errors")]
    quiet: bool,

    #[arg(
        long = "log-level",
        global = true,
        value_enum,
        conflicts_with = "verbose",
        help = "set stderr logging level explicitly"
    )]
    log_level: Option<CliLogLevel>,

    #[arg(short = 'C', long = "show-changes", global = true, action = ArgAction::SetTrue, help = "additionally print keys that have values in [file2] but that do not match the values in [file1]")]
    show_changes: bool,

    #[arg(long = "touch", global = true, action = ArgAction::SetTrue, help = "force writing output files even when output content has not changed")]
    touch: bool,

    #[arg(
        long = "output-encoding",
        global = true,
        value_enum,
        default_value = "utf8-no-bom",
        help = "output file encoding: utf8-no-bom (default), utf8-bom, utf8"
    )]
    output_encoding: CliOutputEncoding,

    #[arg(
        long = "output-format",
        global = true,
        value_enum,
        default_value = "jsonl",
        help = "stdout output format: jsonl (default), json, table"
    )]
    output_format: CliOutputFormat,

    #[arg(long = "dry-run", global = true, action = ArgAction::SetTrue, help = "compute results without writing output files")]
    dry_run: bool,

    #[arg(long = "check", global = true, action = ArgAction::SetTrue, help = "exit with code 2 if the command would produce changes; implies --dry-run")]
    check: bool,

    #[arg(long = "summary", global = true, action = ArgAction::SetTrue, help = "print a structured execution summary instead of full result rows")]
    summary: bool,

    #[arg(long = "count-only", global = true, action = ArgAction::SetTrue, help = "print only the number of matching results to stdout")]
    count_only: bool,

    #[arg(long = "key-prefix", global = true, action = ArgAction::Append, help = "only keep results whose key starts with the given prefix")]
    key_prefix: Vec<String>,

    #[arg(long = "key-contains", global = true, action = ArgAction::Append, help = "only keep results whose key contains the given text")]
    key_contains: Vec<String>,

    #[arg(
        long = "limit",
        global = true,
        help = "limit the number of results written to stdout or output file"
    )]
    limit: Option<usize>,

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

fn query_options(cli: &Cli) -> query::QueryRunOptions {
    query::QueryRunOptions {
        output_format: cli.output_format.into(),
        count_only: cli.count_only,
        dry_run: cli.dry_run || cli.check,
        check: cli.check,
        summary: cli.summary,
        key_prefixes: cli.key_prefix.clone(),
        key_contains: cli.key_contains.clone(),
        limit: cli.limit,
        operation_name: None,
        input_files: Vec::new(),
        output_file: None,
    }
}

fn run(cli: &Cli) -> Result<query::QueryExecutionReport, Box<dyn Error>> {
    let output_encoding: OutputEncoding = cli.output_encoding.into();
    let options = query_options(cli);

    match &cli.command {
        Commands::FilePrint { file, output } => {
            let options = options.with_context("file-print", &[file], output.as_deref());
            let query = query::query_print(file);
            query
                .run_interactive_with_options(output.as_deref(), cli.touch, output_encoding, &options)
                .map_err(Into::into)
        }
        Commands::FileDiff { file1, file2, output } => {
            let operation_name = if cli.show_changes {
                "file-diff-changes"
            } else {
                "file-diff"
            };
            let options = options.with_context(operation_name, &[file1, file2], output.as_deref());
            if cli.show_changes {
                let query = query::query_change(file1, file2);
                query
                    .run_interactive_with_options(output.as_deref(), cli.touch, output_encoding, &options)
                    .map_err(Into::into)
            } else {
                let query = query::query_diff(file1, file2);
                query
                    .run_interactive_with_options(output.as_deref(), cli.touch, output_encoding, &options)
                    .map_err(Into::into)
            }
        }
        Commands::FileMerge { file1, file2, output } => {
            let options = options.with_context("file-merge", &[file1, file2], output.as_deref());
            let query = query::query_merge(file1, file2);
            query
                .run_interactive_with_options(output.as_deref(), cli.touch, output_encoding, &options)
                .map_err(Into::into)
        }
        Commands::FileIntersect { file1, file2, output } => {
            let options = options.with_context("file-intersect", &[file1, file2], output.as_deref());
            let query = query::query_intersect(file1, file2);
            query
                .run_interactive_with_options(output.as_deref(), cli.touch, output_encoding, &options)
                .map_err(Into::into)
        }
        Commands::FileSubtract { file1, file2, output } => {
            let options = options.with_context("file-subtract", &[file1, file2], output.as_deref());
            let query = query::query_subtract(file1, file2);
            query
                .run_interactive_with_options(output.as_deref(), cli.touch, output_encoding, &options)
                .map_err(Into::into)
        }
        Commands::FileConvert { file, output } => {
            let options = options.with_context("file-convert", &[file], Some(output));
            let query = query::query_convert(file);
            query
                .run_interactive_with_options(Some(output), cli.touch, output_encoding, &options)
                .map_err(Into::into)
        }
        Commands::FileSort { file, output } => {
            let target = output.as_deref().or(Some(file.as_str()));
            let options = options.with_context("file-sort", &[file], target);
            let query = query::query_sort(file);

            if output.is_some() {
                query
                    .run_interactive_with_options(output.as_deref(), cli.touch, output_encoding, &options)
                    .map_err(Into::into)
            } else {
                query
                    .run_interactive_with_options(Some(file), cli.touch, output_encoding, &options)
                    .map_err(Into::into)
            }
        }
        Commands::DiffWithBase { old, new, base } => {
            let options = options.with_context("diff-with-base", &[old, new, base], None);
            let query = query::query_diff_with_base(old, new, base);
            query.run_triple_interactive_with_options(&options).map_err(Into::into)
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let min_log_level = if cli.quiet {
        "error"
    } else if let Some(log_level) = cli.log_level {
        log_level.as_filter()
    } else {
        match cli.verbose {
            0 => "warn",
            1 => "info",
            2 => "debug",
            _ => "trace",
        }
    };

    let mut builder = Builder::from_env(Env::default().default_filter_or(min_log_level));
    builder.init();

    match run(&cli) {
        Ok(report) => {
            if cli.check && report.indicates_change() {
                ExitCode::from(2)
            } else {
                ExitCode::SUCCESS
            }
        }
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
        assert!(!cli.touch);
        assert_eq!(cli.output_format, CliOutputFormat::Jsonl);
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

    #[test]
    fn parse_file_sort_with_touch() {
        let cli = Cli::parse_from(["cirup", "--touch", "file-sort", "a.json"]);

        assert!(cli.touch);
        assert_eq!(cli.output_encoding, CliOutputEncoding::Utf8NoBom);
        assert_eq!(cli.output_format, CliOutputFormat::Jsonl);
        match cli.command {
            Commands::FileSort { file, output } => {
                assert_eq!(file, "a.json");
                assert_eq!(output, None);
            }
            _ => panic!("expected file-sort command"),
        }
    }

    #[test]
    fn parse_output_encoding_values() {
        let bom = Cli::parse_from([
            "cirup",
            "--output-encoding",
            "utf8-bom",
            "file-convert",
            "a.json",
            "b.restext",
        ]);
        assert_eq!(bom.output_encoding, CliOutputEncoding::Utf8Bom);

        let utf8 = Cli::parse_from([
            "cirup",
            "--output-encoding",
            "utf8",
            "file-convert",
            "a.json",
            "b.restext",
        ]);
        assert_eq!(utf8.output_encoding, CliOutputEncoding::Utf8);
    }

    #[test]
    fn parse_output_format_filters_and_limit() {
        let cli = Cli::parse_from([
            "cirup",
            "--output-format",
            "table",
            "--key-prefix",
            "lbl",
            "--key-prefix",
            "msg",
            "--key-contains",
            "Hello",
            "--limit",
            "10",
            "file-print",
            "a.json",
        ]);

        assert_eq!(cli.output_format, CliOutputFormat::Table);
        assert_eq!(cli.key_prefix, vec![String::from("lbl"), String::from("msg")]);
        assert_eq!(cli.key_contains, vec![String::from("Hello")]);
        assert_eq!(cli.limit, Some(10));
    }

    #[test]
    fn parse_quiet_and_count_only() {
        let cli = Cli::parse_from([
            "cirup",
            "--quiet",
            "--count-only",
            "diff-with-base",
            "old.json",
            "new.json",
            "base.json",
        ]);

        assert!(cli.quiet);
        assert!(cli.count_only);
    }

    #[test]
    fn parse_dry_run_check_and_summary() {
        let cli = Cli::parse_from(["cirup", "--dry-run", "--check", "--summary", "file-sort", "a.json"]);

        assert!(cli.dry_run);
        assert!(cli.check);
        assert!(cli.summary);
    }

    #[test]
    fn query_options_make_check_imply_dry_run() {
        let cli = Cli::parse_from(["cirup", "--check", "file-print", "a.json"]);
        let options = query_options(&cli);

        assert!(options.dry_run);
        assert!(options.check);
        assert!(!options.summary);
    }
}
