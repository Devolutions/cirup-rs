
use regex::Regex;

use Resource;
use FileFormat;
use file::load_string_from_file;

/**
 * .restext file format:
 * https://docs.microsoft.com/en-us/dotnet/framework/tools/resgen-exe-resource-file-generator
 * https://docs.microsoft.com/en-us/dotnet/framework/resources/creating-resource-files-for-desktop-apps
 */

lazy_static! {
    static ref REGEX_RESTEXT: Regex = Regex::new(r"^\s*(\w+)=(.*)$").unwrap();
}

pub struct RestextFileFormat {

}

impl FileFormat for RestextFileFormat {

    const EXTENSION: &'static str = "restext";

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
