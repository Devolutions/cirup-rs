
use regex::Regex;
use std::fmt;

use Resource;
use file::{FileFormat, FormatType};
use file::{load_string_from_file, save_string_to_file};

/**
 * .restext file format:
 * https://docs.microsoft.com/en-us/dotnet/framework/tools/resgen-exe-resource-file-generator
 * https://docs.microsoft.com/en-us/dotnet/framework/resources/creating-resource-files-for-desktop-apps
 */

/**
 * TODO:
 * - escape '\' as "\\"
 * - replace newline with \r\n
 * - read/write UTF-8 BOM
 * - use \r\n line ending
 */

lazy_static! {
    static ref REGEX_RESTEXT: Regex = Regex::new(r"^\s*(\w+)=(.*)$").unwrap();
}

pub struct RestextFileFormat {

}

impl FileFormat for RestextFileFormat {

    const EXTENSION: &'static str = "restext";
    const TYPE: FormatType = FormatType::Restext;

    fn parse_from_str(&self, text: &str) -> Vec<Resource> {
        let mut resources: Vec<Resource> = Vec::new();

        for line in text.lines() {
            if REGEX_RESTEXT.is_match(line) {
                let captures = REGEX_RESTEXT.captures(line).unwrap();
                let name = &captures[1];
                let value = &captures[2];
                let resource = Resource::new(name, value);
                resources.push(resource);
            }
        }

        resources
    }

    fn parse_from_file(&self, filename: &str) -> Vec<Resource> {
        let text = load_string_from_file(filename);
        self.parse_from_str(text.as_ref())
    }

    fn write_to_str(&self, resources: Vec<Resource>) -> String {
        let mut output = String::new();

        for resource in resources {
            fmt::write(&mut output, format_args!("{}={}\n",
                resource.name, resource.value)).unwrap();
        }

        output
    }

    fn write_to_file(&self, filename: &str, resources: Vec<Resource>) {
        let text = self.write_to_str(resources);
        save_string_to_file(filename, text.as_str());
    }
}

#[test]
fn test_restext_parse() {
    let text = r#"
lblBoat=I'm on a boat.
lblYolo=You only live once
lblDogs=Who let the dogs out?
"#;

    let file_format = RestextFileFormat { };

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
}

#[test]
fn test_restext_write() {

    let file_format = RestextFileFormat { };

    let resources = vec![
        Resource::new("lblBoat", "I'm on a boat."),
        Resource::new("lblYolo", "You only live once"),
        Resource::new("lblDogs", "Who let the dogs out?"),
    ];

    let expected_text =
r#"lblBoat=I'm on a boat.
lblYolo=You only live once
lblDogs=Who let the dogs out?
"#;

    let actual_text = file_format.write_to_str(resources);
    println!("{}", actual_text);
    println!("{}", expected_text);
    assert_eq!(actual_text, expected_text);
}
