
extern crate treexml;
use treexml::Document;

use Resource;

fn resx_parse_text(text: &str) -> Vec<Resource> {
    let doc = Document::parse(text.as_bytes()).unwrap();
    let root = doc.root.unwrap();

    let mut resources: Vec<Resource> = Vec::new();
    let children: Vec<&treexml::Element> = root.filter_children(|t| t.name == "data").collect();

    for data in children {
        let data_name = data.attributes.get(&"name".to_owned()).unwrap();
        let value = data.find_child(|tag| tag.name == "value").unwrap().clone();
        let data_value = value.text.unwrap().clone();
        let resource = Resource {
            name: data_name.clone(),
            value: data_value,
        };
        resources.push(resource);
    }

    resources
}

#[test]
fn test_resx_parse() {
    let text = r#"
    <?xml version="1.0" encoding="utf-8"?>
    <root>
      <data name="lblBoat" xml:space="preserve">
        <value>I'm on a boat.</value>
      </data>
      <data name="lblYolo" xml:space="preserve">
        <value>You only live once</value>
      </data>
      <data name="lblDogs" xml:space="preserve">
        <value>Who let the dogs out?</value>
      </data>
    </root>
    "#;

    let resources = resx_parse_text(&text);

    assert_eq!(resources.get(0).unwrap().name, "lblBoat");
    assert_eq!(resources.get(0).unwrap().value, "I'm on a boat.");

    assert_eq!(resources.get(1).unwrap().name, "lblYolo");
    assert_eq!(resources.get(1).unwrap().value, "You only live once");

    assert_eq!(resources.get(2).unwrap().name, "lblDogs");
    assert_eq!(resources.get(2).unwrap().value, "Who let the dogs out?");
}
