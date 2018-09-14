
extern crate uuid;
extern crate regex;
extern crate treexml;

extern crate serde;
extern crate serde_json;
extern crate dot_json;

extern crate rusqlite;

#[macro_use]
extern crate prettytable;

#[macro_use]
extern crate lazy_static;

pub mod resource;
use resource::Resource;

pub mod json;
pub mod resx;
pub mod restext;

pub mod file;
pub mod vtab;
pub mod query;
