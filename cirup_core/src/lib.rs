
extern crate uuid;
extern crate regex;
extern crate treexml;
extern crate serde_json;
extern crate rusqlite;
extern crate prettytable;

#[macro_use]
extern crate lazy_static;

use std::fmt;

#[derive(Clone)]
pub struct Resource {
    name: String,
    value: String,
}

impl fmt::Display for Resource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} = {}", self.name, self.value)
    }
}

impl Resource {
    pub fn new(name: &str, value: &str) -> Self {
        Resource {
            name: name.to_owned(),
            value: value.to_owned(),
        }
    }
}

pub trait FileFormat {
    const EXTENSION: &'static str;
    fn parse_from_str(&self, text: &str) -> Vec<Resource>;
    fn parse_from_file(&self, filename: &str) -> Vec<Resource>;
}

pub mod json;
pub mod resx;
pub mod restext;

pub mod file;
pub mod vtab;
pub mod engine;
