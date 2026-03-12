extern crate serde;
extern crate serde_json;

use serde::Serialize;
use serde_json::{Map, Value};
#[cfg(test)]
use std::time::Instant;

use crate::Resource;
use crate::file::FileFormat;
use crate::file::load_string_from_file;
use std::error::Error;

pub(crate) struct JsonFileFormat {}

fn json_dot_insert(root_map: &mut Map<String, Value>, name: &str, value: &str) {
    if let Some((root_path, child_path)) = name.split_once('.') {
        let child_value = root_map
            .entry(root_path.to_owned())
            .or_insert_with(|| Value::Object(Map::new()));

        if let Value::Object(child_map) = child_value {
            json_dot_insert(child_map, child_path, value);
        }
    } else {
        root_map.insert(name.to_owned(), Value::String(value.to_owned()));
    }
}

fn flatten_json_value(value: &Value, path: &mut String, resources: &mut Vec<Resource>) {
    match value {
        Value::Object(object) => {
            for (key, child_value) in object {
                let prefix_len = path.len();
                if prefix_len > 0 {
                    path.push('.');
                }
                path.push_str(key);
                flatten_json_value(child_value, path, resources);
                path.truncate(prefix_len);
            }
        }
        Value::String(text) => resources.push(Resource::new(path, text)),
        _ => {}
    }
}

fn json_to_string_pretty(value: &Map<String, Value>) -> String {
    let writer = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser = serde_json::Serializer::with_formatter(writer, formatter);
    if value.serialize(&mut ser).is_err() {
        return "{}".to_owned();
    }
    String::from_utf8(ser.into_inner()).unwrap_or_default()
}

impl FileFormat for JsonFileFormat {
    const EXTENSION: &'static str = "json";

    fn parse_from_str(&self, text: &str) -> Result<Vec<Resource>, Box<dyn Error>> {
        let mut resources: Vec<Resource> = Vec::new();
        let root_value: Value = serde_json::from_str(text)?;
        let root_object = match root_value.as_object() {
            Some(object) => object,
            None => Err("json value is not an object")?,
        };

        let mut path = String::new();
        for (key, value) in root_object {
            path.clear();
            path.push_str(key);
            flatten_json_value(value, &mut path, &mut resources);
        }

        Ok(resources)
    }

    fn parse_from_file(&self, filename: &str) -> Result<Vec<Resource>, Box<dyn Error>> {
        let text = load_string_from_file(filename)?;
        self.parse_from_str(text.as_ref())
    }

    fn write_to_str(&self, resources: &[Resource]) -> String {
        let mut root_map: Map<String, Value> = Map::new();

        for resource in resources {
            json_dot_insert(&mut root_map, &resource.name, &resource.value);
        }

        json_to_string_pretty(&root_map)
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

    let resources = match file_format.parse_from_str(text) {
        Ok(resources) => resources,
        Err(e) => panic!("json parse failed: {}", e),
    };

    let resource = &resources[0];
    assert_eq!(resource.name, "lblBoat");
    assert_eq!(resource.value, "I'm on a boat.");

    let resource = &resources[1];
    assert_eq!(resource.name, "lblYolo");
    assert_eq!(resource.value, "You only live once");

    let resource = &resources[2];
    assert_eq!(resource.name, "lblDogs");
    assert_eq!(resource.value, "Who let the dogs out?");

    let resource = &resources[3];
    assert_eq!(resource.name, "language.en");
    assert_eq!(resource.value, "English");

    let resource = &resources[4];
    assert_eq!(resource.name, "language.fr");
    assert_eq!(resource.value, "French");

    let resource = &resources[5];
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

#[test]
#[ignore = "benchmark: run manually with --ignored --nocapture"]
#[allow(clippy::print_stdout)]
fn benchmark_json_parse_and_write_large_input() {
    let file_format = JsonFileFormat {};
    let repetitions = 5_000usize;
    let mut resources = Vec::with_capacity(repetitions * 6);

    for index in 0..repetitions {
        let prefix = format!("group{index}");
        resources.push(Resource::new(&format!("{prefix}.lblBoat"), "I'm on a boat."));
        resources.push(Resource::new(&format!("{prefix}.lblYolo"), "You only live once"));
        resources.push(Resource::new(&format!("{prefix}.lblDogs"), "Who let the dogs out?"));
        resources.push(Resource::new(&format!("{prefix}.language.en"), "English"));
        resources.push(Resource::new(&format!("{prefix}.language.fr"), "French"));
        resources.push(Resource::new(&format!("{prefix}.very.deep.object"), "value"));
    }

    let started = Instant::now();
    let written = file_format.write_to_str(&resources);
    let write_elapsed = started.elapsed();

    let started = Instant::now();
    let reparsed = file_format
        .parse_from_str(&written)
        .unwrap_or_else(|e| panic!("json benchmark parse failed: {}", e));
    let parse_elapsed = started.elapsed();

    assert_eq!(reparsed.len(), resources.len());

    println!(
        "json benchmark: resources={} bytes={} write={:?} parse={:?}",
        resources.len(),
        written.len(),
        write_elapsed,
        parse_elapsed
    );
}
