use std::fs;
use std::path::Path;
use std::error::Error;

use toml;

#[derive(Serialize,Deserialize)]
pub struct Config {
    pub vcs: Vcs,
    pub sync: Sync,
}

#[derive(Serialize,Deserialize)]
pub struct Vcs {
    pub plugin: String,
    pub local_path: String,
    pub remote_path: String,
}

#[derive(Serialize,Deserialize)]
pub struct Sync {
    pub source_language: String,
    pub target_languages: Vec<String>,
    pub match_language_file: String,
    pub match_language_name: String,
    pub source_dir: String,
    pub working_dir: String,
}

impl Config {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<Error>> {
        let contents = fs::read_to_string(path)?;
        let config = Config::new_from_string(&contents)?;

        info!("source language: {}", config.sync.source_language);
        info!("target language(s): {}", config.sync.target_languages.join(" "));

        Ok(config)
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
        sync: Sync {
            source_language: "en".to_string(),
            target_languages: vec!["fr".to_string(), "de".to_string()],
            match_language_file: "\\.json$".to_string(),
            match_language_name: "(.+?)(\\.[^.]*$|$)".to_string(),
            source_dir: "xxx".to_string(),
            working_dir: "xxx".to_string(),
        },
    };

    let toml = toml::to_string(&config).unwrap();
    println!("{}", toml);
}
