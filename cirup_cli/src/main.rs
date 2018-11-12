#[macro_use]
extern crate clap;
extern crate cirup_core;

use clap::App;
use cirup_core::query::CirupEngine;
use cirup_core::file::save_resource_file;

fn get_file_args(files: Vec<&str>) -> (&str, &str, Option<&str>) {
    if files.len() > 2 {
        (files[0], files[1], Some(files[2]))
    } else {
        (files[0], files[1], None)
    }
}

fn main() {
    let yaml = load_yaml!("cli.yml");
    let app = App::from_yaml(yaml);
    let matches = app.version(crate_version!()).get_matches();

    let engine = CirupEngine::new();

    if let Some(files) = matches.values_of("different") {
        let (file_a, file_b, _file_c) = get_file_args(files.collect());
        engine.register_table_from_file("A", file_a);
        engine.register_table_from_file("B", file_b);
        let query = "SELECT A.key, A.val, B.val FROM A LEFT OUTER JOIN B ON A.key=B.key WHERE A.val <> B.val";
        engine.query(query);
    }
    if let Some(files) = matches.values_of("merge") {
        let (file_a, file_b, _file_c) = get_file_args(files.collect());
        engine.register_table_from_file("A", file_a);
        engine.register_table_from_file("B", file_b);
        let query = "SELECT * FROM A UNION SELECT * from B";
        engine.query(query);
    }
    if let Some(files) = matches.values_of("intersect") {
        let (file_a, file_b, _file_c) = get_file_args(files.collect());
        engine.register_table_from_file("A", file_a);
        engine.register_table_from_file("B", file_b);
        let query = "SELECT * FROM A INTERSECT SELECT * from B";
        engine.query(query);
    }
    if let Some(files) = matches.values_of("subtract") {
        let (file_a, file_b, file_c) = get_file_args(files.collect());
        engine.subtract_command(file_a, file_b, file_c);
    }
    if let Some(files) = matches.values_of("convert") {
        let files: Vec<&str> = files.collect();
        let input = files[0];
        let output = files[1];
        engine.register_table_from_file("A", input);
        let query = "SELECT * FROM A";
        let resources = engine.query_resource(query);
        save_resource_file(output, resources);
    }
    if let Some(files) = matches.values_of("print") {
        let files: Vec<&str> = files.collect();
        let file_a = files[0];
        engine.register_table_from_file("A", file_a);
        let query = "SELECT * FROM A";
        engine.query(query);
    }
}
