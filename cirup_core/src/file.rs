
use std::fs;
use std::io::Read;
use std::path::Path;

use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

use Resource;
use FileFormat;
use json::JsonFileFormat;
use resx::ResxFileFormat;
use restext::RestextFileFormat;

pub fn load_string_from_file(filename: &str) -> String {
    if let Some(text) = vfile_get(filename) {
        return text;
    }
    let mut file = fs::File::open(filename).unwrap();
    let mut text = String::new();
    file.read_to_string(&mut text).unwrap();
    text
}

pub fn load_resource_file(filename: &str) -> Vec<Resource> {
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
            Vec::new()
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
