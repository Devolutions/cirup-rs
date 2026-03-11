use std::fs;
use std::io::Read;
use std::path::Path;

use std::collections::HashMap;
use std::sync::Mutex;

use sha2::{Digest, Sha256};

use crate::Resource;
use crate::json::JsonFileFormat;
use crate::restext::RestextFileFormat;
use crate::resx::ResxFileFormat;
use std::error::Error;

const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

#[derive(Debug, PartialEq, Eq, Copy, Clone, Default)]
pub enum OutputEncoding {
    #[default]
    Utf8NoBom,
    Utf8Bom,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub(crate) enum FormatType {
    Unknown,
    Json,
    Resx,
    Restext,
}

pub(crate) trait FileFormat {
    const EXTENSION: &'static str;
    fn parse_from_str(&self, text: &str) -> Result<Vec<Resource>, Box<dyn Error>>;
    fn parse_from_file(&self, filename: &str) -> Result<Vec<Resource>, Box<dyn Error>>;
    fn write_to_str(&self, resources: &[Resource]) -> String;
}

pub(crate) fn get_format_type_from_extension(extension: &str) -> FormatType {
    match extension {
        JsonFileFormat::EXTENSION => FormatType::Json,
        ResxFileFormat::EXTENSION => FormatType::Resx,
        RestextFileFormat::EXTENSION => FormatType::Restext,
        _ => FormatType::Unknown,
    }
}

pub(crate) fn load_string_from_file(filename: &str) -> Result<String, Box<dyn Error>> {
    if let Some(text) = vfile_get(filename) {
        return Ok(text);
    }
    let mut file = fs::File::open(filename)?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    Ok(text)
}

fn sha256_hash(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

fn should_write_output(output_hash: [u8; 32], existing_bytes: Option<&[u8]>, touch: bool) -> bool {
    if touch {
        return true;
    }

    let Some(existing_bytes) = existing_bytes else {
        return true;
    };

    output_hash != sha256_hash(existing_bytes)
}

fn encode_utf8(text: &str, output_encoding: OutputEncoding) -> Vec<u8> {
    match output_encoding {
        OutputEncoding::Utf8NoBom => text.as_bytes().to_vec(),
        OutputEncoding::Utf8Bom => {
            let mut output = Vec::with_capacity(UTF8_BOM.len() + text.len());
            output.extend_from_slice(&UTF8_BOM);
            output.extend_from_slice(text.as_bytes());
            output
        }
    }
}

fn output_bytes_for_format(
    format_type: FormatType,
    resources: &[Resource],
    output_encoding: OutputEncoding,
) -> Vec<u8> {
    match format_type {
        FormatType::Json => {
            let file_format = JsonFileFormat {};
            let text = file_format.write_to_str(resources);
            encode_utf8(&text, output_encoding)
        }
        FormatType::Resx => {
            let file_format = ResxFileFormat {};
            let text = file_format.write_to_str(resources);
            encode_utf8(&text, output_encoding)
        }
        FormatType::Restext => {
            let file_format = RestextFileFormat {};
            let text = file_format.write_to_str(resources);
            encode_utf8(&text, output_encoding)
        }
        FormatType::Unknown => Vec::new(),
    }
}

fn output_bytes_for_file(filename: &str, resources: &[Resource], output_encoding: OutputEncoding) -> Option<Vec<u8>> {
    let path = Path::new(filename);
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    let format_type = get_format_type_from_extension(extension);

    if format_type == FormatType::Unknown {
        return None;
    }

    Some(output_bytes_for_format(format_type, resources, output_encoding))
}

#[cfg(test)]
pub(crate) fn load_resource_str(text: &str, extension: &str) -> Result<Vec<Resource>, Box<dyn Error>> {
    match extension {
        JsonFileFormat::EXTENSION => {
            let file_format = JsonFileFormat {};
            file_format.parse_from_str(text)
        }
        ResxFileFormat::EXTENSION => {
            let file_format = ResxFileFormat {};
            file_format.parse_from_str(text)
        }
        RestextFileFormat::EXTENSION => {
            let file_format = RestextFileFormat {};
            file_format.parse_from_str(text)
        }
        _ => Ok(Vec::new()),
    }
}

pub(crate) fn load_resource_file(filename: &str) -> Result<Vec<Resource>, Box<dyn Error>> {
    let path = Path::new(filename);
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .ok_or_else(|| format!("file '{}' has no valid extension", filename))?;
    match get_format_type_from_extension(extension) {
        FormatType::Json => {
            let file_format = JsonFileFormat {};
            file_format.parse_from_file(filename)
        }
        FormatType::Resx => {
            let file_format = ResxFileFormat {};
            file_format.parse_from_file(filename)
        }
        FormatType::Restext => {
            let file_format = RestextFileFormat {};
            file_format.parse_from_file(filename)
        }
        FormatType::Unknown => Ok(Vec::new()),
    }
}

pub(crate) fn save_resource_file(filename: &str, resources: &[Resource], touch: bool) {
    save_resource_file_with_encoding(filename, resources, touch, OutputEncoding::Utf8NoBom);
}

pub(crate) fn save_resource_file_with_encoding(
    filename: &str,
    resources: &[Resource],
    touch: bool,
    output_encoding: OutputEncoding,
) {
    let Some(output_bytes) = output_bytes_for_file(filename, resources, output_encoding) else {
        return;
    };
    let output_hash = sha256_hash(&output_bytes);
    let existing_bytes = fs::read(filename).ok();

    if should_write_output(output_hash, existing_bytes.as_deref(), touch) {
        fs::write(filename, output_bytes).expect("failed to write output file");
    }
}

pub(crate) fn would_save_resource_file_with_encoding(
    filename: &str,
    resources: &[Resource],
    touch: bool,
    output_encoding: OutputEncoding,
) -> bool {
    let Some(output_bytes) = output_bytes_for_file(filename, resources, output_encoding) else {
        return false;
    };

    let output_hash = sha256_hash(&output_bytes);
    let existing_bytes = fs::read(filename).ok();
    should_write_output(output_hash, existing_bytes.as_deref(), touch)
}

lazy_static! {
    static ref HASHMAP: Mutex<HashMap<String, String>> = {
        let map = HashMap::new();
        Mutex::new(map)
    };
}

#[cfg(test)]
pub(crate) fn vfile_set(id: &str, data: &str) {
    if let Ok(mut map) = HASHMAP.lock() {
        map.insert(id.to_owned(), data.to_owned());
    }
}

pub(crate) fn vfile_get(id: &str) -> Option<String> {
    let map = HASHMAP.lock().ok()?;
    map.get(id).cloned()
}

#[cfg(test)]
fn temp_output_file_path(extension: &str) -> String {
    let path = std::env::temp_dir().join(format!("cirup-touch-test-{}.{}", uuid::Uuid::new_v4(), extension));
    path.to_string_lossy().into_owned()
}

#[test]
fn test_vfile() {
    vfile_set("59398a3e-757b-4844-b103-047d32324a4e", "foo");
    vfile_set("48acadf4-4821-49df-a318-537db5000d2b", "bar");
    assert_eq!(
        vfile_get("59398a3e-757b-4844-b103-047d32324a4e").as_deref(),
        Some("foo")
    );
    assert_eq!(
        vfile_get("48acadf4-4821-49df-a318-537db5000d2b").as_deref(),
        Some("bar")
    );

    let test_json = include_str!("../test/test.json");
    vfile_set("test.json", test_json);
    assert_eq!(vfile_get("test.json").as_deref(), Some(test_json));
}

#[test]
fn format_type_from_extension() {
    assert_eq!(get_format_type_from_extension("json"), FormatType::Json);
    assert_eq!(get_format_type_from_extension("resx"), FormatType::Resx);
    assert_eq!(get_format_type_from_extension("restext"), FormatType::Restext);
    assert_eq!(get_format_type_from_extension("txt"), FormatType::Unknown);
}

#[test]
fn should_skip_write_when_hashes_match_and_touch_is_false() {
    let output = b"same-content";
    let output_hash = sha256_hash(output);
    assert!(!should_write_output(output_hash, Some(output), false));
}

#[test]
fn should_write_when_hashes_differ_and_touch_is_false() {
    let output = b"new-content";
    let existing = b"old-content";
    let output_hash = sha256_hash(output);
    assert!(should_write_output(output_hash, Some(existing), false));
}

#[test]
fn should_write_when_touch_is_true_even_if_hashes_match() {
    let output = b"same-content";
    let output_hash = sha256_hash(output);
    assert!(should_write_output(output_hash, Some(output), true));
}

#[test]
fn restext_output_bytes_do_not_include_utf8_bom() {
    let resources = vec![Resource::new("hello", "world")];
    let output = output_bytes_for_format(FormatType::Restext, &resources, OutputEncoding::Utf8NoBom);
    assert!(!output.starts_with(&UTF8_BOM));
}

#[test]
fn restext_output_bytes_include_utf8_bom_when_configured() {
    let resources = vec![Resource::new("hello", "world")];
    let output = output_bytes_for_format(FormatType::Restext, &resources, OutputEncoding::Utf8Bom);
    assert!(output.starts_with(&UTF8_BOM));
}

#[test]
fn save_resource_file_does_not_touch_unchanged_file_by_default() {
    let filename = temp_output_file_path("json");
    let resources = vec![Resource::new("hello", "world")];

    save_resource_file(&filename, &resources, false);
    let first_modified = fs::metadata(&filename)
        .and_then(|metadata| metadata.modified())
        .expect("failed to read first output file timestamp");

    std::thread::sleep(std::time::Duration::from_millis(1200));

    save_resource_file(&filename, &resources, false);
    let second_modified = fs::metadata(&filename)
        .and_then(|metadata| metadata.modified())
        .expect("failed to read second output file timestamp");

    let _ = fs::remove_file(&filename);
    assert_eq!(first_modified, second_modified);
}

#[test]
fn save_resource_file_touches_unchanged_file_when_touch_is_true() {
    let filename = temp_output_file_path("json");
    let resources = vec![Resource::new("hello", "world")];

    save_resource_file(&filename, &resources, false);
    let first_modified = fs::metadata(&filename)
        .and_then(|metadata| metadata.modified())
        .expect("failed to read first output file timestamp");

    std::thread::sleep(std::time::Duration::from_millis(1200));

    save_resource_file(&filename, &resources, true);
    let second_modified = fs::metadata(&filename)
        .and_then(|metadata| metadata.modified())
        .expect("failed to read second output file timestamp");

    let _ = fs::remove_file(&filename);
    assert!(second_modified > first_modified);
}

#[test]
fn would_save_resource_file_reports_false_for_unchanged_output() {
    let filename = temp_output_file_path("json");
    let resources = vec![Resource::new("hello", "world")];

    save_resource_file(&filename, &resources, false);

    let would_write = would_save_resource_file_with_encoding(&filename, &resources, false, OutputEncoding::Utf8NoBom);

    let _ = fs::remove_file(&filename);
    assert!(!would_write);
}

#[test]
fn would_save_resource_file_reports_true_for_missing_output() {
    let filename = temp_output_file_path("json");
    let resources = vec![Resource::new("hello", "world")];

    let would_write = would_save_resource_file_with_encoding(&filename, &resources, false, OutputEncoding::Utf8NoBom);

    assert!(would_write);
}
