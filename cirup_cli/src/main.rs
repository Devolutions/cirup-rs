#[macro_use]
extern crate clap;
extern crate cirup_core;

// use cirup_core::config;
// use cirup_core::vcs;
// use cirup_core::job;

use clap::App;

// use std::fs::File;
// use std::io::prelude::*;
// use std::path::Path;
use cirup_core::query::CirupQuery;

fn print(input: &str) {
    let query = CirupQuery::new();
    query.register_table("A", input);
    query.query("SELECT * FROM A", None);
}

fn diff(file_one: &str, file_two: &str, out_file: Option<&str>) {
    let query = CirupQuery::new();
    query.register_table("A", file_one);
    query.register_table("B", file_two);
    query.query(r"
        SELECT A.key, A.val, B.val 
        FROM A 
        LEFT OUTER JOIN B ON A.key = B.key 
        WHERE (B.val IS NULL) OR (A.val <> B.val)", 
        out_file);
}

fn merge(file_one: &str, file_two: &str, out_file: Option<&str>) {
    let query = CirupQuery::new();
    query.register_table("A", file_one);
    query.register_table("B", file_two);
    query.query(r"
        SELECT 
            A.key,
            (CASE WHEN B.val IS NOT NULL
                  THEN B.val
                  ELSE A.val END) as val
        FROM A
        LEFT OUTER JOIN B on A.key = B.key",
        out_file);
}

fn intersect(file_one: &str, file_two: &str, out_file: Option<&str>) {
    let query = CirupQuery::new();
    query.register_table("A", file_one);
    query.register_table("B", file_two);
    query.query(r"
        SELECT * FROM A 
        INTERSECT 
        SELECT * from B",
        out_file);
}

fn subtract(file_one: &str, file_two: &str, out_file: Option<&str>) {
    let query = CirupQuery::new();
    query.register_table("A", file_one);
    query.register_table("B", file_two);
    query.query(r"
        SELECT * FROM A 
        WHERE A.key NOT IN 
            (SELECT B.key FROM B)",
        out_file);
}

fn convert(file_one: &str, out_file: &str) {
    let query = CirupQuery::new();
    query.register_table("A", file_one);
    query.query("SELECT * FROM A", Some(out_file));
}

fn main() {
    let yaml = load_yaml!("cli.yml");
    let app = App::from_yaml(yaml);
    let matches = app.version(crate_version!()).get_matches();

    match matches.subcommand() {
        ("print", Some(args)) => { 
            print(args.value_of("file").unwrap()) 
        },
        ("diff", Some(args)) => { 
            diff(args.value_of("file1").unwrap(), args.value_of("file2").unwrap(), args.value_of("output")) 
        },
        ("merge", Some(args)) => {
            merge(args.value_of("file1").unwrap(), args.value_of("file2").unwrap(), args.value_of("output")) 
        },
        ("intersect", Some(args)) => {
            intersect(args.value_of("file1").unwrap(), args.value_of("file2").unwrap(), args.value_of("output")) 
        },
        ("subtract", Some(args)) => {
            subtract(args.value_of("file1").unwrap(), args.value_of("file2").unwrap(), args.value_of("output")) 
        },
        ("convert", Some(args)) => {
            convert(args.value_of("file").unwrap(), args.value_of("output").unwrap()) 
        },
        ("", None) => { /* no subcommand */ },
        _ => { /* unreachable */ },
    }

    
    if let Some(config_file) = matches.value_of("config") {
        println!("using config file: {}", config_file);
    }

    // if let Some(config) = matches.value_of("config") {
    //         let cfg = cirup_core::config::Config::config_from_file(config);
    //         println!(">> {}", cfg.vcs.remote_path);

    //         let vcs = vcs::Vcs::new(&cfg);
    //         //vcs.pull();

    //         let job = job::Job::new(&cfg);

    //         println!("source language: {}", job.source_language);

    //         let p = Path::new(&cfg.job.source_dir);
    //         let f = Path::new(&job.source_language);
    //         let x = p.join(f.file_name().unwrap());

    //         vcs.show(&x.to_string_lossy().to_string(), "b5caeb2", "/Users/rdevolutions/Desktop/test")

            //vcs.log(&job.source_language);
            //vcs.log_pretty(&job.source_language, "%h - %an, %ar : %s");
            //vcs.diff(&job.source_language, "dd3a4a71");
            //vcs.log_pretty_since(&job.source_language, "%h - %an, %ar : %s", "c92bdbb76026bc5124a3728b1ed058dcad65472b", true);
    // }
    // else look for .cirup config in the current directory?
}
