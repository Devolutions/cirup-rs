
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

mod resource;
use resource::Resource;

mod error;
mod shell;
pub mod config;

mod json;
mod resx;
mod restext;

mod file;
mod vtab;

pub mod query;

mod revision;
mod utils;
pub mod vcs;
pub mod sync;