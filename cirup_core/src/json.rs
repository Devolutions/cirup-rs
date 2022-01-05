extern crate dot_json;
extern crate serde;
extern crate serde_json;

use dot_json::value_to_dot;
use serde::Serialize;
use serde_json::{Map, Value};

use file::{load_string_from_file, save_string_to_file};
use file::{FileFormat, FormatType};
use std::error::Error;
use Resource;

pub struct JsonFileFormat {}

fn json_dot_insert(root_map: &mut Map<String, Value>, name: &str, value: &str) {
    if let Some(dot_index) = name.find('.') {
        let root_path = &name[0..dot_index];
        let child_path = &name[dot_index + 1..name.len()];

        if !root_map.contains_key(root_path) {
            let child_map: Map<String, Value> = Map::new();
            root_map.insert(root_path.to_string(), Value::Object(child_map));
        }

        let mut child_map = root_map
            .get_mut(root_path)
            .unwrap()
            .as_object_mut()
            .unwrap();
        json_dot_insert(&mut child_map, child_path, value);
    } else {
        root_map.insert(name.to_string(), Value::String(value.to_string()));
    }
}

fn json_to_string_pretty(value: &Map<String, Value>) -> String {
    let writer = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser = serde_json::Serializer::with_formatter(writer, formatter);
    value.serialize(&mut ser).unwrap();
    String::from_utf8(ser.into_inner()).unwrap()
}

impl FileFormat for JsonFileFormat {
    const EXTENSION: &'static str = "json";
    const TYPE: FormatType = FormatType::Json;

    fn parse_from_str(&self, text: &str) -> Result<Vec<Resource>, Box<dyn Error>> {
        let mut resources: Vec<Resource> = Vec::new();
        let root_value: Value = serde_json::from_str(text)?;
        let root_value_dot = value_to_dot(&root_value);
        let root_object_dot = root_value_dot.as_object().unwrap();
        for (key, value) in root_object_dot.iter() {
            let resource = Resource::new(key.as_str(), value.as_str().unwrap());
            resources.push(resource);
        }
        Ok(resources)
    }

    fn parse_from_file(&self, filename: &str) -> Result<Vec<Resource>, Box<dyn Error>> {
        let text = load_string_from_file(filename)?;
        self.parse_from_str(text.as_ref())
    }

    fn write_to_str(&self, resources: &Vec<Resource>) -> String {
        let mut root_map: Map<String, Value> = Map::new();

        for resource in resources {
            json_dot_insert(&mut root_map, &resource.name, &resource.value);
        }

        json_to_string_pretty(&root_map)
    }

    fn write_to_file(&self, filename: &str, resources: &Vec<Resource>) {
        let text = self.write_to_str(resources);
        save_string_to_file(filename, text.as_str());
    }
}

#[test]
fn test_json_parse() {
    let text = r#"
{
    "lblBoat": "I'm on a boat.",
    "lblYolo": "You only live once",
    "lblDogs": "Who let the dogs out?",
    "language": {
        "en": "English",
        "fr": "French"
    },
    "very": {
        "deep": {
            "object": "value"
        }
    }
}
    "#;

    let file_format = JsonFileFormat {};

    let resources = file_format.parse_from_str(&text).unwrap();

    let resource = resources.get(0).unwrap();
    assert_eq!(resource.name, "lblBoat");
    assert_eq!(resource.value, "I'm on a boat.");

    let resource = resources.get(1).unwrap();
    assert_eq!(resource.name, "lblYolo");
    assert_eq!(resource.value, "You only live once");

    let resource = resources.get(2).unwrap();
    assert_eq!(resource.name, "lblDogs");
    assert_eq!(resource.value, "Who let the dogs out?");

    let resource = resources.get(3).unwrap();
    assert_eq!(resource.name, "language.en");
    assert_eq!(resource.value, "English");

    let resource = resources.get(4).unwrap();
    assert_eq!(resource.name, "language.fr");
    assert_eq!(resource.value, "French");

    let resource = resources.get(5).unwrap();
    assert_eq!(resource.name, "very.deep.object");
    assert_eq!(resource.value, "value");
}

#[test]
fn test_json_write() {
    let file_format = JsonFileFormat {};

    let resources = vec![
        Resource::new("lblBoat", "I'm on a boat."),
        Resource::new("lblYolo", "You only live once"),
        Resource::new("lblDogs", "Who let the dogs out?"),
        Resource::new("language.en", "English"),
        Resource::new("language.fr", "French"),
        Resource::new("very.deep.object", "value"),
    ];

    let expected_text = r#"{
    "lblBoat": "I'm on a boat.",
    "lblYolo": "You only live once",
    "lblDogs": "Who let the dogs out?",
    "language": {
        "en": "English",
        "fr": "French"
    },
    "very": {
        "deep": {
            "object": "value"
        }
    }
}"#;

    let actual_text = file_format.write_to_str(&resources);
    //println!("{}", actual_text);
    //println!("{}", expected_text);
    assert_eq!(actual_text, expected_text);
}
