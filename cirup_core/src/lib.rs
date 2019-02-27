extern crate regex;
extern crate treexml;
extern crate uuid;

#[macro_use]
extern crate serde_derive;
extern crate dot_json;
extern crate rusqlite;
extern crate serde;
extern crate serde_json;
extern crate serde_xml_rs;
extern crate tempfile;
extern crate toml;
#[macro_use]
extern crate log;

#[macro_use]
extern crate prettytable;

#[macro_use]
extern crate lazy_static;

mod resource;
use resource::Resource;

pub mod config;
mod error;
mod shell;

mod json;
mod restext;
mod resx;

mod file;
mod vtab;

pub mod query;

mod revision;
pub mod sync;
mod utils;
pub mod vcs;
