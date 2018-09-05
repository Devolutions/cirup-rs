#[macro_use]
extern crate clap;
extern crate cirup_core;

use clap::App;
use cirup_core::engine::CirupEngine;

fn main() {
    let yaml = load_yaml!("cli.yml");
    let app = App::from_yaml(yaml);
    let matches = app.version(crate_version!()).get_matches();

    let engine = CirupEngine::new();

    if let Some(files) = matches.values_of("diff") {
        let files: Vec<&str> = files.collect();
        let file_a = files[0];
        let file_b = files[1];
        engine.register_table_from_file("A", file_a);
        engine.register_table_from_file("B", file_b);
        let query = "SELECT * FROM A UNION SELECT * from B"; // FIXME: not the good query
        println!("diff {} {}", file_a, file_b);
        engine.query(query);
    }
    if let Some(files) = matches.values_of("merge") {
        let files: Vec<&str> = files.collect();
        let file_a = files[0];
        let file_b = files[1];
        engine.register_table_from_file("A", file_a);
        engine.register_table_from_file("B", file_b);
        let query = "SELECT * FROM A UNION SELECT * from B";
        println!("merge {} {}", file_a, file_b);
        engine.query(query);
    }
    if let Some(files) = matches.values_of("intersect") {
        let files: Vec<&str> = files.collect();
        let file_a = files[0];
        let file_b = files[1];
        engine.register_table_from_file("A", file_a);
        engine.register_table_from_file("B", file_b);
        let query = "SELECT * FROM A INTERSECT SELECT * from B";
        println!("intersect {} {}", file_a, file_b);
        engine.query(query);
    }
    if let Some(files) = matches.values_of("convert") {
        let files: Vec<&str> = files.collect();
        let file_a = files[0];
        let file_b = files[1];
        engine.register_table_from_file("A", file_a);
        engine.register_table_from_file("B", file_b);
        println!("convert {} {}", file_a, file_b);
        //engine.query(query.as_str());
    }
}
