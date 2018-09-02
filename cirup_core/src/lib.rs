
extern crate treexml;
extern crate serde_json;

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

pub mod resx;
pub mod json;
