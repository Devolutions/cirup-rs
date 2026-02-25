use regex::Regex;
use std::fmt;
use std::fs;
use std::io::prelude::*;

use crate::file::load_string_from_file;
use crate::file::FileFormat;
use std::error::Error;
use crate::Resource;

/*
 * .restext file format:
 * https://docs.microsoft.com/en-us/dotnet/framework/tools/resgen-exe-resource-file-generator
 * https://docs.microsoft.com/en-us/dotnet/framework/resources/creating-resource-files-for-desktop-apps
 */

lazy_static! {
    static ref REGEX_RESTEXT: Regex = Regex::new(r"^\s*(\w+)=(.*)$").unwrap();
}

pub struct RestextFileFormat {}

/* https://lise-henry.github.io/articles/optimising_strings.html */

pub fn escape_newlines(input: &str) -> String {
    let mut output = String::new();
    for c in input.chars() {
        match c {
            '\\' => output.push_str("\\\\"),
            '\r' => output.push_str("\\r"),
            '\n' => output.push_str("\\n"),
            _ => output.push(c),
        }
    }
    output
}

impl FileFormat for RestextFileFormat {
    const EXTENSION: &'static str = "restext";

    fn parse_from_str(&self, text: &str) -> Result<Vec<Resource>, Box<dyn Error>> {
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

        Ok(resources)
    }

    fn parse_from_file(&self, filename: &str) -> Result<Vec<Resource>, Box<dyn Error>> {
        let text = load_string_from_file(filename)?;
        self.parse_from_str(text.as_ref())
    }

    fn write_to_str(&self, resources: &Vec<Resource>) -> String {
        let mut output = String::new();

        for resource in resources {
            let escaped_value = escape_newlines(resource.value.as_str());
            fmt::write(
                &mut output,
                format_args!("{}={}\r\n", resource.name, escaped_value),
            )
            .unwrap();
        }

        output
    }

    fn write_to_file(&self, filename: &str, resources: &Vec<Resource>) {
        let bom: [u8; 3] = [0xEF, 0xBB, 0xBF];
        let text = self.write_to_str(resources);
        let mut file = fs::File::create(filename).unwrap();
        file.write_all(&bom).unwrap();
        file.write_all(text.as_bytes()).unwrap();
    }
}

#[test]
fn test_restext_parse() {
    let text = "lblBoat=I'm on a boat.\r\n\
                lblYolo=You only live once\r\n\
                lblDogs=Who let the dogs out?\r\n";

    let file_format = RestextFileFormat {};

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
fn test_restext_write() {
    let file_format = RestextFileFormat {};

    let resources = vec![
        Resource::new("lblBoat", "I'm on a boat."),
        Resource::new("lblYolo", "You only live once"),
        Resource::new("lblDogs", "Who let the dogs out?"),
    ];

    let expected_text = "lblBoat=I'm on a boat.\r\n\
                         lblYolo=You only live once\r\n\
                         lblDogs=Who let the dogs out?\r\n";

    let actual_text = file_format.write_to_str(&resources);
    println!("{}", actual_text);
    println!("{}", expected_text);
    assert_eq!(actual_text, expected_text);
}

#[test]
fn test_escape_newlines() {
    let text = "line1\\line2\r\nline3";
    let escaped = escape_newlines(text);
    assert_eq!(escaped, "line1\\\\line2\\r\\nline3");
}
