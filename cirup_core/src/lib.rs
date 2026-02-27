extern crate regex;
extern crate treexml;
extern crate uuid;

extern crate dot_json;
#[cfg(feature = "rusqlite-c")]
extern crate rusqlite;
extern crate serde;
extern crate serde_json;
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

mod json;
mod restext;
mod resx;

mod file;
mod query_backend;

pub mod query;

