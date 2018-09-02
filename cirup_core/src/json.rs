
extern crate serde_json;
use serde_json::{Value, Error};

use Resource;

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

fn json_parse_text(text: &str) -> Vec<Resource> {
    let mut resources: Vec<Resource> = Vec::new();
    let root_value: Value = serde_json::from_str(text).unwrap();
    let root_object = root_value.as_object().unwrap();
    json_parse_object("", &root_value, &mut resources);
    resources
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

    let resources = json_parse_text(&text);

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
