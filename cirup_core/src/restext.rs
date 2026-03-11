#[cfg(test)]
use std::time::Instant;

use crate::Resource;
use crate::file::FileFormat;
use crate::file::load_string_from_file;
use std::error::Error;

/*
 * .restext file format:
 * https://docs.microsoft.com/en-us/dotnet/framework/tools/resgen-exe-resource-file-generator
 * https://docs.microsoft.com/en-us/dotnet/framework/resources/creating-resource-files-for-desktop-apps
 */

pub(crate) struct RestextFileFormat {}

/* https://lise-henry.github.io/articles/optimising_strings.html */

fn push_escaped_newlines(output: &mut String, input: &str) {
    for c in input.chars() {
        match c {
            '\\' => output.push_str("\\\\"),
            '\r' => output.push_str("\\r"),
            '\n' => output.push_str("\\n"),
            _ => output.push(c),
        }
    }
}

fn parse_restext_line(line: &str) -> Option<(&str, &str)> {
    let (name_part, value) = line.split_once('=')?;
    let name = name_part.trim_start_matches(char::is_whitespace);

    if name.is_empty() || !name.chars().all(|ch| ch == '_' || ch.is_alphanumeric()) {
        return None;
    }

    Some((name, value))
}

#[cfg(test)]
pub(crate) fn escape_newlines(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    push_escaped_newlines(&mut output, input);
    output
}

impl FileFormat for RestextFileFormat {
    const EXTENSION: &'static str = "restext";

    fn parse_from_str(&self, text: &str) -> Result<Vec<Resource>, Box<dyn Error>> {
        let mut resources: Vec<Resource> = Vec::new();
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);

        for line in text.lines() {
            if let Some((name, value)) = parse_restext_line(line) {
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

    fn write_to_str(&self, resources: &[Resource]) -> String {
        let estimated_len = resources
            .iter()
            .map(|resource| resource.name.len() + resource.value.len() + 3)
            .sum::<usize>();
        let mut output = String::with_capacity(estimated_len);

        for resource in resources {
            output.push_str(&resource.name);
            output.push('=');
            push_escaped_newlines(&mut output, resource.value.as_str());
            output.push_str("\r\n");
        }

        output
    }
}

#[test]
fn test_restext_parse() {
    let text = "lblBoat=I'm on a boat.\r\n\
                lblYolo=You only live once\r\n\
                lblDogs=Who let the dogs out?\r\n";

    let file_format = RestextFileFormat {};

    let resources = match file_format.parse_from_str(text) {
        Ok(resources) => resources,
        Err(e) => panic!("restext parse failed: {}", e),
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
fn test_restext_parse_with_utf8_bom() {
    let text = "\u{feff}lblBoat=I'm on a boat.\r\n";

    let file_format = RestextFileFormat {};

    let resources = match file_format.parse_from_str(text) {
        Ok(resources) => resources,
        Err(e) => panic!("restext parse with bom failed: {}", e),
    };

    let resource = &resources[0];
    assert_eq!(resource.name, "lblBoat");
    assert_eq!(resource.value, "I'm on a boat.");
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
    assert_eq!(actual_text, expected_text);
}

#[test]
fn test_escape_newlines() {
    let text = "line1\\line2\r\nline3";
    let escaped = escape_newlines(text);
    assert_eq!(escaped, "line1\\\\line2\\r\\nline3");
}

#[test]
#[ignore = "benchmark: run manually with --ignored --nocapture"]
#[allow(clippy::print_stdout)]
fn benchmark_restext_parse_and_write_large_input() {
    let file_format = RestextFileFormat {};
    let base = include_str!("../test/test.restext");
    let repetitions = 20_000usize;
    let mut text = String::with_capacity(base.len() * repetitions);

    for _ in 0..repetitions {
        text.push_str(base);
    }

    let started = Instant::now();
    let resources = file_format
        .parse_from_str(&text)
        .unwrap_or_else(|e| panic!("restext parse benchmark failed: {}", e));
    let parse_elapsed = started.elapsed();

    let started = Instant::now();
    let written = file_format.write_to_str(&resources);
    let write_elapsed = started.elapsed();

    assert_eq!(resources.len(), 3 * repetitions);
    assert!(!written.is_empty());

    println!(
        "restext benchmark: lines={} bytes={} parse={:?} write={:?}",
        resources.len(),
        text.len(),
        parse_elapsed,
        write_elapsed
    );
}
