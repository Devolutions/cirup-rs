#[macro_use]
extern crate clap;
extern crate cirup_core;

use clap::App;
use cirup_core::engine::CirupEngine;

fn main() {
    let yaml = load_yaml!("cli.yml");
    let app = App::from_yaml(yaml);
    let matches = app.version(crate_version!()).get_matches();
    let input = matches.value_of("input").unwrap_or("").to_string();
    let table = matches.value_of("table").unwrap_or("").to_string();
    let query = matches.value_of("query").unwrap_or("").to_string();

    let engine = CirupEngine::new();
    engine.register_table_from_file("input", input.as_str());
    engine.query(query.as_str());
}
