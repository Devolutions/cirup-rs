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

    #[arg(
        long = "key-filter",
        global = true,
        action = ArgAction::Append,
        help = "repeatable simple regex-style key filter supporting literals, ^, $, . and .*"
    )]
    key_filter: Vec<String>,

    #[arg(
        long = "value-filter",
        global = true,
        action = ArgAction::Append,
        help = "repeatable simple regex-style value filter supporting literals, ^, $, . and .*"
    )]
    value_filter: Vec<String>,

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
        key_filters: cli.key_filter.clone(),
        value_filters: cli.value_filter.clone(),
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
    fn parse_log_level_value() {
        let cli = Cli::parse_from(["cirup", "--log-level", "debug", "file-print", "a.json"]);

        assert_eq!(cli.log_level, Some(CliLogLevel::Debug));
        assert_eq!(cli.verbose, 0);
        assert!(!cli.quiet);
    }

    #[test]
    fn parse_output_format_filters_and_limit() {
        let cli = Cli::parse_from([
            "cirup",
            "--output-format",
            "table",
            "--key-filter",
            "^lbl",
            "--value-filter",
            ".*Hello$",
            "--limit",
            "10",
            "file-print",
            "a.json",
        ]);

        assert_eq!(cli.output_format, CliOutputFormat::Table);
        assert_eq!(cli.key_filter, vec![String::from("^lbl")]);
        assert_eq!(cli.value_filter, vec![String::from(".*Hello$")]);
        assert_eq!(cli.limit, Some(10));
    }

    #[test]
    fn parse_repeated_key_and_value_filters() {
        let cli = Cli::parse_from([
            "cirup",
            "--key-filter",
            "^lbl",
            "--key-filter",
            ".*Title$",
            "--value-filter",
            "^English$",
            "--value-filter",
            ".*French.*",
            "file-print",
            "a.json",
        ]);

        assert_eq!(cli.key_filter, vec![String::from("^lbl"), String::from(".*Title$")]);
        assert_eq!(
            cli.value_filter,
            vec![String::from("^English$"), String::from(".*French.*")]
        );
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
    fn parse_file_commands_with_outputs() {
        let convert = Cli::parse_from(["cirup", "file-convert", "a.json", "b.restext"]);
        match convert.command {
            Commands::FileConvert { file, output } => {
                assert_eq!(file, "a.json");
                assert_eq!(output, "b.restext");
            }
            _ => panic!("expected file-convert command"),
        }

        let merge = Cli::parse_from(["cirup", "file-merge", "a.json", "b.json", "out.json"]);
        match merge.command {
            Commands::FileMerge { file1, file2, output } => {
                assert_eq!(file1, "a.json");
                assert_eq!(file2, "b.json");
                assert_eq!(output.as_deref(), Some("out.json"));
            }
            _ => panic!("expected file-merge command"),
        }

        let intersect = Cli::parse_from(["cirup", "file-intersect", "a.json", "b.json", "out.json"]);
        match intersect.command {
            Commands::FileIntersect { file1, file2, output } => {
                assert_eq!(file1, "a.json");
                assert_eq!(file2, "b.json");
                assert_eq!(output.as_deref(), Some("out.json"));
            }
            _ => panic!("expected file-intersect command"),
        }

        let subtract = Cli::parse_from(["cirup", "file-subtract", "a.json", "b.json", "out.json"]);
        match subtract.command {
            Commands::FileSubtract { file1, file2, output } => {
                assert_eq!(file1, "a.json");
                assert_eq!(file2, "b.json");
                assert_eq!(output.as_deref(), Some("out.json"));
            }
            _ => panic!("expected file-subtract command"),
        }
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

    #[test]
    fn quiet_conflicts_with_verbose_and_log_level() {
        let verbose_error = Cli::try_parse_from(["cirup", "--quiet", "--verbose", "file-print", "a.json"])
            .expect_err("expected quiet/verbose conflict");
        assert!(verbose_error.to_string().contains("cannot be used with"));

        let log_level_error = Cli::try_parse_from(["cirup", "--quiet", "--log-level", "info", "file-print", "a.json"])
            .expect_err("expected quiet/log-level conflict");
        assert!(log_level_error.to_string().contains("cannot be used with"));
    }

    #[test]
    fn log_level_conflicts_with_verbose() {
        let error = Cli::try_parse_from(["cirup", "--log-level", "trace", "-v", "file-print", "a.json"])
            .expect_err("expected log-level/verbose conflict");

        assert!(error.to_string().contains("cannot be used with"));
    }

    #[test]
    fn removed_key_prefix_and_contains_flags_are_rejected() {
        let prefix_error = Cli::try_parse_from(["cirup", "--key-prefix", "lbl", "file-print", "a.json"])
            .expect_err("expected removed key-prefix flag to fail");
        assert!(prefix_error.to_string().contains("unexpected argument '--key-prefix'"));

        let pattern_error = Cli::try_parse_from(["cirup", "--key-pattern", "^lbl", "file-print", "a.json"])
            .expect_err("expected removed key-pattern flag to fail");
        assert!(
            pattern_error
                .to_string()
                .contains("unexpected argument '--key-pattern'")
        );

        let contains_error = Cli::try_parse_from(["cirup", "--key-contains", "Hello", "file-print", "a.json"])
            .expect_err("expected removed key-contains flag to fail");
        assert!(
            contains_error
                .to_string()
                .contains("unexpected argument '--key-contains'")
        );
    }
}
