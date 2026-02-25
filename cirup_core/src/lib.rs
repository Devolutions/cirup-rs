extern crate chrono;
extern crate regex;
extern crate treexml;
extern crate uuid;

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
pub use crate::resource::Resource;

mod triple;
pub use crate::triple::Triple;

pub mod config;
mod shell;

mod json;
mod restext;
mod resx;

mod file;
mod query_backend;

pub mod query;

mod revision;
pub mod sync;
mod utils;
pub mod vcs;
