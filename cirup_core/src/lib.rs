
extern crate uuid;
extern crate regex;
extern crate treexml;
extern crate serde_json;
extern crate rusqlite;

#[macro_use]
extern crate prettytable;

#[macro_use]
extern crate lazy_static;

pub mod resource;
use resource::Resource;

pub trait FileFormat {
    const EXTENSION: &'static str;
    fn parse_from_str(&self, text: &str) -> Vec<Resource>;
    fn parse_from_file(&self, filename: &str) -> Vec<Resource>;
    fn write_to_str(&self, resources: Vec<Resource>) -> String;
    fn write_to_file(&self, filename: &str, resources: Vec<Resource>);
}

pub mod json;
pub mod resx;
pub mod restext;

pub mod file;
pub mod vtab;
pub mod engine;
