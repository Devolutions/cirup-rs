#[macro_use]
extern crate clap;
extern crate cirup_core;

use std::error::Error;
use std::path::Path;

use clap::App;

use cirup_core::config::Config;
use cirup_core::query;
use cirup_core::sync::Sync;
use cirup_core::vcs;

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

fn run(matches: &clap::ArgMatches, config: Option<Config>) -> Result<(), Box<Error>> {
    match matches.subcommand() {
        ("file-print", Some(args)) => { 
            print(args.value_of("file").unwrap(), args.value_of("output"));
            Ok(())
        },
        ("file-diff", Some(args)) => { 
            if args.is_present("show_changes") {
                change(args.value_of("file1").unwrap(), args.value_of("file2").unwrap(), args.value_of("output"));
            } else {
                diff(args.value_of("file1").unwrap(), args.value_of("file2").unwrap(), args.value_of("output"));
            }
            Ok(())
        },
        ("file-merge", Some(args)) => {
            merge(args.value_of("file1").unwrap(), args.value_of("file2").unwrap(), args.value_of("output"));
            Ok(())
        },
        ("file-intersect", Some(args)) => {
            intersect(args.value_of("file1").unwrap(), args.value_of("file2").unwrap(), args.value_of("output"));
            Ok(())
        },
        ("file-subtract", Some(args)) => {
            subtract(args.value_of("file1").unwrap(), args.value_of("file2").unwrap(), args.value_of("output"));
            Ok(())
        },
        ("file-convert", Some(args)) => {
            convert(args.value_of("file").unwrap(), args.value_of("output").unwrap());
            Ok(())
        },
        ("vcs-log", Some(args)) => {
            match config {
                Some(c) => {
                    let sync = Sync::new(&c)?;
                    let vcs = vcs::new(&c)?;

                    println!("source language is {:?}", sync.source_language_path());

                    vcs.pull()?;  
                    vcs.log(
                        &sync.source_language_path().to_string_lossy(), 
                        args.value_of("format"), 
                        args.value_of("old_commit"), 
                        args.value_of("new_commit"), 
                        true)?;

                    Ok(())
                },
                None => { Err("configuration file required")? }
            }
        },
        ("vcs-diff", Some(args)) => {
            match config {
                Some(c) => {
                    let sync = Sync::new(&c)?;
                    let vcs = vcs::new(&c)?;

                    println!("source language is {:?}", sync.source_language_path());

                    vcs.pull()?;  
                    vcs.diff(
                        &sync.source_language_path().to_string_lossy(), 
                        args.value_of("old_commit").unwrap(), 
                        args.value_of("new_commit"), )?;

                    Ok(())
                },
                None => { Err("configuration file required")? }
            }
        },
        ("pull", Some(args)) => {
            match config {
                Some(c) => {
                    let sync = Sync::new(&c)?;
                    sync.pull(args.value_of("old_commit"), args.value_of("new_commit"))?;

                    Ok(())
                },
                None => { Err("configuration file required")? }
            }
        },
        ("push", Some(args)) => {
            match config {
                Some(c) => {
                    let sync = Sync::new(&c)?;
                    sync.push(args.is_present("force"))?;

                    Ok(())
                },
                None => { Err("configuration file required")? }
            }
        }
        _ => { Err("unrecognised subcommand")? },
    }
}

fn main() {
    let yaml = load_yaml!("cli.yml");
    let app = App::from_yaml(yaml);
    let matches = app.version(crate_version!()).get_matches();

    let mut config : Option<Config> = None;

    if let Some(config_file) = matches.value_of("config") {
        match Config::new(Path::new(config_file)) {
            Ok(c) => { config = Some(c) },
            Err(e) => { println!("failed to read config file: {:?}", e) }
        }
    }

    match run(&matches, config) {
        Ok(()) => { return }
        Err(e) => { println!("{}", e)}
    }
}
