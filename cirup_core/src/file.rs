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
pub(crate) enum FormatType {
    Unknown,
    Json,
    Resx,
    Restext,
}

pub(crate) trait FileFormat {
    const EXTENSION: &'static str;
    fn parse_from_str(&self, text: &str) -> Result<Vec<Resource>, Box<dyn Error>>;
    fn parse_from_file(&self, filename: &str) -> Result<Vec<Resource>, Box<dyn Error>>;
    fn write_to_str(&self, resources: &[Resource]) -> String;
    fn write_to_file(&self, filename: &str, resources: &[Resource]);
}

pub(crate) fn get_format_type_from_extension(extension: &str) -> FormatType {
    match extension {
        JsonFileFormat::EXTENSION => FormatType::Json,
        ResxFileFormat::EXTENSION => FormatType::Resx,
        RestextFileFormat::EXTENSION => FormatType::Restext,
        _ => FormatType::Unknown,
    }
}

pub(crate) fn load_string_from_file(filename: &str) -> Result<String, Box<dyn Error>> {
    if let Some(text) = vfile_get(filename) {
        return Ok(text);
    }
    let mut file = fs::File::open(filename)?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    Ok(text)
}

pub(crate) fn save_string_to_file(filename: &str, text: &str) {
    let mut file = fs::File::create(filename).expect("failed to create output file");
    file.write_all(text.as_bytes()).expect("failed to write output file");
}

#[cfg(test)]
pub(crate) fn load_resource_str(text: &str, extension: &str) -> Result<Vec<Resource>, Box<dyn Error>> {
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

pub(crate) fn load_resource_file(filename: &str) -> Result<Vec<Resource>, Box<dyn Error>> {
    let path = Path::new(filename);
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .ok_or_else(|| format!("file '{}' has no valid extension", filename))?;
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

pub(crate) fn save_resource_file(filename: &str, resources: &[Resource]) {
    let path = Path::new(filename);
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
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

#[cfg(test)]
pub(crate) fn vfile_set(id: &str, data: &str) {
    if let Ok(mut map) = HASHMAP.lock() {
        map.insert(id.to_owned(), data.to_owned());
    }
}

pub(crate) fn vfile_get(id: &str) -> Option<String> {
    let map = HASHMAP.lock().ok()?;
    map.get(id).cloned()
}

#[test]
fn test_vfile() {
    vfile_set("59398a3e-757b-4844-b103-047d32324a4e", "foo");
    vfile_set("48acadf4-4821-49df-a318-537db5000d2b", "bar");
    assert_eq!(
        vfile_get("59398a3e-757b-4844-b103-047d32324a4e").as_deref(),
        Some("foo")
    );
    assert_eq!(
        vfile_get("48acadf4-4821-49df-a318-537db5000d2b").as_deref(),
        Some("bar")
    );

    let test_json = include_str!("../test/test.json");
    vfile_set("test.json", test_json);
    assert_eq!(vfile_get("test.json").as_deref(), Some(test_json));
}

#[test]
fn format_type_from_extension() {
    assert_eq!(get_format_type_from_extension("json"), FormatType::Json);
    assert_eq!(get_format_type_from_extension("resx"), FormatType::Resx);
    assert_eq!(get_format_type_from_extension("restext"), FormatType::Restext);
    assert_eq!(get_format_type_from_extension("txt"), FormatType::Unknown);
}
