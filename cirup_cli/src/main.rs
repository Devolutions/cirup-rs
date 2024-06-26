#[macro_use]
extern crate clap;
extern crate cirup_core;
#[macro_use]
extern crate log;
extern crate env_logger;

use std::error::Error;
use std::path::Path;

use clap::App;
use env_logger::{Builder, Env};

use cirup_core::config::Config;
use cirup_core::query;
use cirup_core::sync::Sync;

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

fn run(matches: &clap::ArgMatches, config: Option<Config>) -> Result<(), Box<dyn Error>> {
    match matches.subcommand() {
        ("file-print", Some(args)) => {
            print(args.value_of("file").unwrap(), args.value_of("output"));
            Ok(())
        }
        ("file-diff", Some(args)) => {
            if args.is_present("show_changes") {
                change(
                    args.value_of("file1").unwrap(),
                    args.value_of("file2").unwrap(),
                    args.value_of("output"),
                );
            } else {
                diff(
                    args.value_of("file1").unwrap(),
                    args.value_of("file2").unwrap(),
                    args.value_of("output"),
                );
            }
            Ok(())
        }
        ("file-merge", Some(args)) => {
            merge(
                args.value_of("file1").unwrap(),
                args.value_of("file2").unwrap(),
                args.value_of("output"),
            );
            Ok(())
        }
        ("file-intersect", Some(args)) => {
            intersect(
                args.value_of("file1").unwrap(),
                args.value_of("file2").unwrap(),
                args.value_of("output"),
            );
            Ok(())
        }
        ("file-subtract", Some(args)) => {
            subtract(
                args.value_of("file1").unwrap(),
                args.value_of("file2").unwrap(),
                args.value_of("output"),
            );
            Ok(())
        }
        ("file-convert", Some(args)) => {
            convert(
                args.value_of("file").unwrap(),
                args.value_of("output").unwrap(),
            );
            Ok(())
        }
        ("file-sort", Some(args)) => {
            sort(
                args.value_of("file").unwrap(),
                args.value_of("output"),
            );
            Ok(())
        }
        ("vcs-log", Some(args)) => match config {
            Some(c) => {
                let sync = Sync::new(&c)?;

                sync.vcs.log(
                    &sync.source_language_path().to_string_lossy(),
                    args.value_of("format"),
                    args.value_of("old_commit"),
                    args.value_of("new_commit"),
                    true,
                    value_t!(args, "limit", u32).unwrap_or(0),
                )?;

                Ok(())
            }
            None => Err("configuration file required")?,
        },
        ("vcs-diff", Some(args)) => match config {
            Some(c) => {
                let sync = Sync::new(&c)?;

                sync.vcs.diff(
                    &sync.source_language_path().to_string_lossy(),
                    args.value_of("old_commit").unwrap(),
                    args.value_of("new_commit"),
                )?;

                Ok(())
            }
            None => Err("configuration file required")?,
        },
        ("pull", Some(args)) => match config {
            Some(c) => {
                let sync = Sync::new(&c)?;
                sync.pull(
                    args.value_of("old_commit"),
                    args.value_of("new_commit"),
                    args.is_present("show_changes"),
                )?;

                Ok(())
            }
            None => Err("configuration file required")?,
        },
        ("push", Some(args)) => match config {
            Some(c) => {
                let sync = Sync::new(&c)?;
                sync.push(args.value_of("old_commit"), args.value_of("new_commit"))?;

                Ok(())
            }
            None => Err("configuration file required")?,
        },
        ("diff-with-base", Some(args)) => {
            diff_with_base(
                args.value_of("old").unwrap(),
                args.value_of("new").unwrap(),
                args.value_of("base").unwrap(),
            );
            Ok(())
        }        
        _ => Err("unrecognised subcommand")?,
    }
}

fn main() {
    let yaml = load_yaml!("cli.yml");
    let app = App::from_yaml(yaml);
    let matches = app.version(crate_version!()).get_matches();

    let min_log_level = match matches.occurrences_of("verbose") {
        0 => "info",
        1 => "debug",
        2 | _ => "trace",
    };

    let mut builder = Builder::from_env(Env::default().default_filter_or(min_log_level));
    builder.init();

    let mut config: Option<Config> = None;

    if let Some(config_file) = matches.value_of("config") {
        match Config::new(Path::new(config_file)) {
            Ok(c) => config = Some(c),
            Err(e) => error!("unable to read the config file ({})", e),
        }
    }

    match run(&matches, config) {
        Ok(()) => return,
        Err(e) => error!("an unexpected error occured ({})", e),
    }
}
