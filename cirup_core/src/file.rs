use std::fs;
use std::io::Read;
use std::io::prelude::*;
use std::path::Path;

use std::collections::HashMap;
use std::sync::Mutex;

use crate::Resource;
use crate::json::JsonFileFormat;
use crate::restext::RestextFileFormat;
use crate::resx::ResxFileFormat;
use std::error::Error;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum FormatType {
    Unknown,
    Json,
    Resx,
    Restext,
}

pub trait FileFormat {
    const EXTENSION: &'static str;
    fn parse_from_str(&self, text: &str) -> Result<Vec<Resource>, Box<dyn Error>>;
    fn parse_from_file(&self, filename: &str) -> Result<Vec<Resource>, Box<dyn Error>>;
    fn write_to_str(&self, resources: &Vec<Resource>) -> String;
    fn write_to_file(&self, filename: &str, resources: &Vec<Resource>);
}

pub fn get_format_type_from_extension(extension: &str) -> FormatType {
    match extension {
        JsonFileFormat::EXTENSION => FormatType::Json,
        ResxFileFormat::EXTENSION => FormatType::Resx,
        RestextFileFormat::EXTENSION => FormatType::Restext,
        _ => FormatType::Unknown,
    }
}

pub fn load_string_from_file(filename: &str) -> Result<String, Box<dyn Error>> {
    if let Some(text) = vfile_get(filename) {
        return Ok(text);
    }
    let mut file = fs::File::open(filename).unwrap();
    let mut text = String::new();
    file.read_to_string(&mut text).unwrap();
    Ok(text)
}

pub fn save_string_to_file(filename: &str, text: &str) {
    let mut file = fs::File::create(filename).unwrap();
    file.write_all(text.as_bytes()).unwrap();
}

#[cfg(test)]
pub fn load_resource_str(text: &str, extension: &str) -> Result<Vec<Resource>, Box<dyn Error>> {
    match extension {
        JsonFileFormat::EXTENSION => {
            let file_format = JsonFileFormat {};
            file_format.parse_from_str(text)
        }
        ResxFileFormat::EXTENSION => {
            let file_format = ResxFileFormat {};
            file_format.parse_from_str(text)
        }
        RestextFileFormat::EXTENSION => {
            let file_format = RestextFileFormat {};
            file_format.parse_from_str(text)
        }
        _ => Ok(Vec::new()),
    }
}

pub fn load_resource_file(filename: &str) -> Result<Vec<Resource>, Box<dyn Error>> {
    let path = Path::new(filename);
    let extension = path.extension().unwrap().to_str().unwrap();
    match get_format_type_from_extension(extension) {
        FormatType::Json => {
            let file_format = JsonFileFormat {};
            file_format.parse_from_file(filename)
        }
        FormatType::Resx => {
            let file_format = ResxFileFormat {};
            file_format.parse_from_file(filename)
        }
        FormatType::Restext => {
            let file_format = RestextFileFormat {};
            file_format.parse_from_file(filename)
        }
        FormatType::Unknown => Ok(Vec::new()),
    }
}

pub fn save_resource_file(filename: &str, resources: &Vec<Resource>) {
    let path = Path::new(filename);
    let extension = path.extension().unwrap().to_str().unwrap();
    match get_format_type_from_extension(extension) {
        FormatType::Json => {
            let file_format = JsonFileFormat {};
            file_format.write_to_file(filename, resources)
        }
        FormatType::Resx => {
            let file_format = ResxFileFormat {};
            file_format.write_to_file(filename, resources)
        }
        FormatType::Restext => {
            let file_format = RestextFileFormat {};
            file_format.write_to_file(filename, resources)
        }
        FormatType::Unknown => {}
    }
}

lazy_static! {
    static ref HASHMAP: Mutex<HashMap<String, String>> = {
        let map = HashMap::new();
        Mutex::new(map)
    };
}

pub fn vfile_set(id: &str, data: &str) {
    let mut map = HASHMAP.lock().unwrap();
    map.insert(id.to_string(), data.to_string());
}

pub fn vfile_get(id: &str) -> Option<String> {
    let map = HASHMAP.lock().unwrap();
    if let Some(val) = map.get(id) {
        Some(val.to_string())
    } else {
        None
    }
}

#[test]
fn test_vfile() {
    vfile_set("59398a3e-757b-4844-b103-047d32324a4e", "foo");
    vfile_set("48acadf4-4821-49df-a318-537db5000d2b", "bar");
    let foo = vfile_get("59398a3e-757b-4844-b103-047d32324a4e").unwrap();
    assert_eq!(foo, "foo");
    let bar = vfile_get("48acadf4-4821-49df-a318-537db5000d2b").unwrap();
    assert_eq!(bar, "bar");

    let test_json = include_str!("../test/test.json");
    vfile_set("test.json", test_json);
    let value = vfile_get("test.json").unwrap();
    assert_eq!(value, test_json);
}

#[test]
fn format_type_from_extension() {
    assert_eq!(get_format_type_from_extension("json"), FormatType::Json);
    assert_eq!(get_format_type_from_extension("resx"), FormatType::Resx);
    assert_eq!(get_format_type_from_extension("restext"), FormatType::Restext);
    assert_eq!(get_format_type_from_extension("txt"), FormatType::Unknown);
}
