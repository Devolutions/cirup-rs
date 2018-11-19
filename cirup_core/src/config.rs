extern crate toml;

#[cfg(test)]
//use toml;

use std::fs::File;
use std::path::Path;
use std::io::prelude::*;

#[derive(Serialize,Deserialize)]
pub struct Config {
    pub vcs: Vcs,
    pub job: Job,
}

#[derive(Serialize,Deserialize)]
pub struct Vcs {
    pub plugin: String,
    pub local_path: String,
    pub remote_path: String,
}

#[derive(Serialize,Deserialize)]
pub struct Job {
    pub source_language: String,
    pub source_dir: String,
    pub source_match: String,
    pub source_name_match: String,
}

impl Config {
    pub fn config_from_file(path: &str) -> Config {
        let path = Path::new(&path);
        let mut file = ::std::fs::File::open(path).expect("file not found or cannot be opened");

        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("error reading config file");

        let config: Config = toml::from_str(&contents).expect("unable to parse config file");

        config
    }
}

#[test]
fn config_write() {
    let config = Config {
        vcs: Vcs {
            plugin: "git".to_string(),
            local_path: "xxx".to_string(),
            remote_path: "yyy".to_string(),
        },
        job: Job {
            source_language: "en".to_string(),
            source_dir: "xxx".to_string(),
            source_match: "\\.json$".to_string(),
            source_name_match: "(.+?)(\\.[^.]*$|$)".to_string(),
        },
    };

    let toml = toml::to_string(&config).unwrap();
    println!("{}", toml);
}
