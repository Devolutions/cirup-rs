
extern crate serde_json;
use serde_json::{Value};

use Resource;
use file::{FileFormat, FormatType};
use file::{load_string_from_file, save_string_to_file};

fn json_join_path(root_path: &str, child_path: &str) -> String {
    if root_path.is_empty() {
        child_path.to_string()
    } else {
        vec![root_path.to_string(), child_path.to_string()].join(".")
    }
}

fn json_parse_object(root_path: &str, root_value: &Value, resources: &mut Vec<Resource>) {
    let root_object = root_value.as_object().unwrap();
    for (key, value) in root_object.iter() {
        let path = json_join_path(root_path, key);
        if value.is_object() {
            json_parse_object(&path, value, resources);
        } else {
            let resource = Resource::new(path.as_str(), value.as_str().unwrap());
            resources.push(resource);
        }
    }
}

pub struct JsonFileFormat {

}

impl FileFormat for JsonFileFormat {

    const EXTENSION: &'static str = "json";
    const TYPE: FormatType = FormatType::Json;

    fn parse_from_str(&self, text: &str) -> Vec<Resource> {
        let mut resources: Vec<Resource> = Vec::new();
        let root_value: Value = serde_json::from_str(text).unwrap();
        json_parse_object("", &root_value, &mut resources);
        resources
    }

    fn parse_from_file(&self, filename: &str) -> Vec<Resource> {
        let text = load_string_from_file(filename);
        self.parse_from_str(text.as_ref())
    }

    fn write_to_str(&self, resources: Vec<Resource>) -> String {
        String::new()
    }

    fn write_to_file(&self, filename: &str, resources: Vec<Resource>) {
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

    let file_format = JsonFileFormat { };

    let resources = file_format.parse_from_str(&text);

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
