
extern crate serde_json;
use serde_json::{Value, Error};

use Resource;

fn json_parse_text(text: &str) -> Vec<Resource> {
    let mut resources: Vec<Resource> = Vec::new();
    let root_value: Value = serde_json::from_str(text).unwrap();
    let root_object = root_value.as_object().unwrap();
    for (key, value) in root_object.iter() {
        println!("{} = {}", key, value);
        let resource = Resource {
            name: key.clone(),
            value: value.as_str().unwrap().to_string(),
        };
        resources.push(resource);
    }

    resources
}

#[test]
fn test_json_parse() {
    let text = r#"
    {
        "lblBoat": "I'm on a boat.",
        "lblYolo": "You only live once",
        "lblDogs": "Who let the dogs out?"
    }
    "#;

    let resources = json_parse_text(&text);

    assert_eq!(resources.get(0).unwrap().name, "lblBoat");
    assert_eq!(resources.get(0).unwrap().value, "I'm on a boat.");

    assert_eq!(resources.get(1).unwrap().name, "lblYolo");
    assert_eq!(resources.get(1).unwrap().value, "You only live once");

    assert_eq!(resources.get(2).unwrap().name, "lblDogs");
    assert_eq!(resources.get(2).unwrap().value, "Who let the dogs out?");
}
