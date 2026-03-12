extern crate treexml;
use treexml::Document;

use crate::Resource;
use crate::file::FileFormat;
use crate::file::load_string_from_file;
use std::error::Error;

pub(crate) struct ResxFileFormat {}

fn without_bom(text: &str) -> &[u8] {
    if text.starts_with("\u{feff}") {
        return &text.as_bytes()[3..];
    }

    text.as_bytes()
}

fn push_escaped_xml_text(output: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            _ => output.push(ch),
        }
    }
}

fn push_escaped_xml_attr(output: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(ch),
        }
    }
}

impl FileFormat for ResxFileFormat {
    const EXTENSION: &'static str = "resx";

    fn parse_from_str(&self, text: &str) -> Result<Vec<Resource>, Box<dyn Error>> {
        let mut resources: Vec<Resource> = Vec::new();
        let bytes = without_bom(text);

        if !bytes.is_empty() {
            let doc = Document::parse(bytes).map_err(|e| format!("resx parse error: {:?}", e))?;
            let root = doc.root.ok_or("resx root not found")?;

            for data in root.filter_children(|t| t.name == "data") {
                if let Some(data_name) = data.attributes.get("name")
                    && let Some(value) = data.find_child(|tag| tag.name == "value")
                {
                    let data_value = value.text.as_deref().unwrap_or_default();
                    let resource = Resource::new(data_name, data_value);
                    resources.push(resource);
                }
            }
        }

        Ok(resources)
    }

    fn parse_from_file(&self, filename: &str) -> Result<Vec<Resource>, Box<dyn Error>> {
        let text = load_string_from_file(filename)?;
        self.parse_from_str(text.as_ref())
    }

    fn write_to_str(&self, resources: &[Resource]) -> String {
        let estimated_body_len = resources
            .iter()
            .map(|resource| resource.name.len() + resource.value.len() + 64)
            .sum::<usize>();
        let mut output = String::with_capacity(48 + estimated_body_len);
        output.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<root>");

        for resource in resources {
            output.push_str("\n  <data name=\"");
            push_escaped_xml_attr(&mut output, resource.name.as_str());
            output.push_str("\" xml:space=\"preserve\">\n    <value>");
            push_escaped_xml_text(&mut output, resource.value.as_str());
            output.push_str("</value>\n  </data>");
        }

        output.push_str("\n</root>");
        output
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

    let resources = match file_format.parse_from_str(text) {
        Ok(resources) => resources,
        Err(e) => panic!("resx parse failed: {}", e),
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
