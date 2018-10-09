
use std::fs;
use std::io::Read;
use std::path::Path;
use std::io::prelude::*;

use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

use Resource;
use error::CirupError;
use json::JsonFileFormat;
use resx::ResxFileFormat;
use restext::RestextFileFormat;

pub enum FormatType {
    Unknown,
    Json,
    Resx,
    Restext,
}

pub trait FileFormat {
    const EXTENSION: &'static str;
    const TYPE: FormatType;
    fn parse_from_str(&self, text: &str) -> Vec<Resource>;
    fn parse_from_file(&self, filename: &str) -> Result<Vec<Resource>, CirupError>;
    fn write_to_str(&self, resources: Vec<Resource>) -> String;
    fn write_to_file(&self, filename: &str, resources: Vec<Resource>);
}

pub fn get_format_type_from_extension(extension: &str) -> FormatType {
    match extension {
        JsonFileFormat::EXTENSION => {
            FormatType::Json
        },
        ResxFileFormat::EXTENSION => {
            FormatType::Resx
        },
        RestextFileFormat::EXTENSION => {
            FormatType::Restext
        },
        _ => {
            FormatType::Unknown
        }
    }
}

pub fn load_string_from_file(filename: &str) -> Result<String, CirupError> {
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

pub fn load_resource_str(text: &str, extension: &str) -> Vec<Resource> {
    match extension {
        JsonFileFormat::EXTENSION => {
            let file_format = JsonFileFormat { };
            file_format.parse_from_str(text)
        },
        ResxFileFormat::EXTENSION => {
            let file_format = ResxFileFormat { };
            file_format.parse_from_str(text)
        },
        RestextFileFormat::EXTENSION => {
            let file_format = RestextFileFormat { };
            file_format.parse_from_str(text)
        },
        _ => {
            Vec::new()
        }
    }
}

pub fn load_resource_file(filename: &str) -> Result<Vec<Resource>, CirupError> {
    let path = Path::new(filename);
    let extension = path.extension().unwrap().to_str().unwrap();
    match extension {
        JsonFileFormat::EXTENSION => {
            let file_format = JsonFileFormat { };
            file_format.parse_from_file(filename)
        },
        ResxFileFormat::EXTENSION => {
            let file_format = ResxFileFormat { };
            file_format.parse_from_file(filename)
        },
        RestextFileFormat::EXTENSION => {
            let file_format = RestextFileFormat { };
            file_format.parse_from_file(filename)
        },
        _ => {
            Ok(Vec::new())
        }
    }
}

pub fn save_resource_file(filename: &str, resources: Vec<Resource>) {
    let path = Path::new(filename);
    let extension = path.extension().unwrap().to_str().unwrap();
    match extension {
        JsonFileFormat::EXTENSION => {
            let file_format = JsonFileFormat { };
            file_format.write_to_file(filename, resources)
        },
        ResxFileFormat::EXTENSION => {
            let file_format = ResxFileFormat { };
            file_format.write_to_file(filename, resources)
        },
        RestextFileFormat::EXTENSION => {
            let file_format = RestextFileFormat { };
            file_format.write_to_file(filename, resources)
        },
        _ => {

        }
    }
}

lazy_static! {
    static ref HASHMAP: Mutex<HashMap<String, String>> = {
        let map = HashMap::new();
        Mutex::new(map)
    };
}

pub fn vfile_id() -> String {
    let id = Uuid::new_v4();
    id.to_string()
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
