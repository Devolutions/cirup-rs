extern crate treexml;
use treexml::{Document, Element};

use file::{load_string_from_file, save_string_to_file};
use file::{FileFormat, FormatType};
use std::error::Error;
use Resource;

pub struct ResxFileFormat {}

fn without_bom(text: &str) -> &[u8] {
    if text.starts_with("\u{feff}") {
        return &text.as_bytes()[3..];
    };

    return text.as_bytes();
}

impl FileFormat for ResxFileFormat {
    const EXTENSION: &'static str = "resx";
    const TYPE: FormatType = FormatType::Resx;

    fn parse_from_str(&self, text: &str) -> Result<Vec<Resource>, Box<dyn Error>> {
        let mut resources: Vec<Resource> = Vec::new();
        let bytes = without_bom(text);

        if bytes.len() > 0 {
            let doc = Document::parse(bytes).unwrap();
            let root = doc.root.unwrap();

            let children: Vec<&treexml::Element> =
                root.filter_children(|t| t.name == "data").collect();

            for data in children {
                let data_name = data.attributes.get(&"name".to_owned()).unwrap();
                let value = data.find_child(|tag| tag.name == "value").unwrap().clone();
                let data_value = value.text.unwrap_or_default().clone();
                let resource = Resource::new(data_name, data_value.as_ref());
                resources.push(resource);
            }
        }

        Ok(resources)
    }

    fn parse_from_file(&self, filename: &str) -> Result<Vec<Resource>, Box<dyn Error>> {
        let text = load_string_from_file(filename)?;
        self.parse_from_str(text.as_ref())
    }

    fn write_to_str(&self, resources: &Vec<Resource>) -> String {
        let mut root = Element::new("root");

        for resource in resources {
            let mut data = Element::new("data");
            data.attributes
                .insert("name".to_string(), resource.name.to_string());
            data.attributes
                .insert("xml:space".to_string(), "preserve".to_string());
            let mut value = Element::new("value");
            value.text = Some(resource.value.to_string());
            data.children.push(value);
            root.children.push(data);
        }

        let doc = Document {
            root: Some(root),
            encoding: "utf-8".to_string(),
            ..Document::default()
        };

        doc.to_string()
    }

    fn write_to_file(&self, filename: &str, resources: &Vec<Resource>) {
        let text = self.write_to_str(resources);
        save_string_to_file(filename, text.as_str());
    }
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

    let file_format = ResxFileFormat {};

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
}

#[test]
fn test_resx_write() {
    let file_format = ResxFileFormat {};

    let resources = vec![
        Resource::new("lblBoat", "I'm on a boat."),
        Resource::new("lblYolo", "You only live once"),
        Resource::new("lblDogs", "Who let the dogs out?"),
    ];

    let expected_text = r#"<?xml version="1.0" encoding="utf-8"?>
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
</root>"#;

    let actual_text = file_format.write_to_str(&resources);
    //println!("{}", actual_text);
    //println!("{}", expected_text);
    assert_eq!(actual_text, expected_text);
}
