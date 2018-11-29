use std::fs;
use std::path::Path;
use std::error::Error;

use toml;

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
    pub source_match: String,
    pub source_name_match: String,
    pub source_dir: String,
    pub export_dir: String,
    pub import_dir: String
}

impl Config {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<Error>> {
        let contents = fs::read_to_string(path)?;
        Config::new_from_string(&contents)
    }

    pub fn new_from_string(contents: &str) -> Result<Self, Box<Error>> {
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }
}

#[test]
fn config_read() {
    let config = Config::new_from_string(include_str!("../test/config.cirup")).unwrap();
    let toml = toml::to_string(&config).unwrap();
    println!("{}", toml);
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
            source_match: "\\.json$".to_string(),
            source_name_match: "(.+?)(\\.[^.]*$|$)".to_string(),
            source_dir: "xxx".to_string(),
            export_dir: "xxx".to_string(),
            import_dir: "xxx".to_string(),
        },
    };

    let toml = toml::to_string(&config).unwrap();
    println!("{}", toml);
}
