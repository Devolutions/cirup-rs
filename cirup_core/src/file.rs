
use std::fs;
use std::io::Read;
use std::path::Path;

use Resource;
use json::json_parse_from_file;
use resx::resx_parse_from_file;

pub fn load_string_from_file(filename: &str) -> String {
    let mut file = fs::File::open(filename).unwrap();
    let mut text = String::new();
    file.read_to_string(&mut text).unwrap();
    text
}

pub fn load_resource_file(filename: &str) -> Vec<Resource> {
    let path = Path::new(filename);
    let extension = path.extension().unwrap().to_str().unwrap();
    match extension {
        "json" => {
            json_parse_from_file(filename)
        },
        "resx" => {
            resx_parse_from_file(filename)
        },
        _ => {
            Vec::new()
        }
    }
}
