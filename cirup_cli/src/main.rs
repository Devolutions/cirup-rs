#[macro_use]
extern crate clap;
extern crate cirup_core;

use clap::App;
use cirup_core::vtab::query_file;

fn main() {
    let yaml = load_yaml!("cli.yml");
    let app = App::from_yaml(yaml);
    let matches = app.version(crate_version!()).get_matches();
    let input = matches.value_of("input").unwrap_or("").to_string();
    let table = matches.value_of("table").unwrap_or("").to_string();
    let query = matches.value_of("query").unwrap_or("").to_string();
    query_file(input.as_str(), table.as_str(), query.as_str());
}
