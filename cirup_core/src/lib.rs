
extern crate uuid;
extern crate regex;
extern crate treexml;

#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate serde;
extern crate serde_json;
extern crate serde_xml_rs;
extern crate dot_json;
extern crate tempfile;
extern crate rusqlite;
#[macro_use]
extern crate log;

#[macro_use]
extern crate prettytable;

#[macro_use]
extern crate lazy_static;

pub mod resource;
use resource::Resource;

pub mod error;
pub mod shell;
pub mod config;

pub mod json;
pub mod resx;
pub mod restext;

pub mod file;
pub mod vtab;
pub mod query;

pub mod vcs;
pub mod sync;

mod utils;