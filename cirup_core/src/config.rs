use std::error::Error;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use toml;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub vcs: Vcs,
    #[serde(alias = "job")]
    pub sync: Sync,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Vcs {
    pub plugin: String,
    pub local_path: String,
    pub remote_path: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Sync {
    pub source_language: String,
    #[serde(default)]
    pub target_languages: Vec<String>,
    pub match_language_file: String,
    pub match_language_name: String,
    pub source_dir: String,
    #[serde(alias = "export_dir")]
    pub working_dir: String,
}

impl Config {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let contents = fs::read_to_string(path)?;
        let config = Config::new_from_string(&contents)?;

        info!("source language: {}", config.sync.source_language);
        info!("target language(s): {}", config.sync.target_languages.join(" "));

        Ok(config)
    }

    pub fn new_from_string(contents: &str) -> Result<Self, Box<dyn Error>> {
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }
}

#[test]
fn config_read() {
    let config = Config::new_from_string(include_str!("../test/config.cirup")).unwrap();
    assert_eq!(config.vcs.plugin, "git");
    assert_eq!(config.sync.source_language, "en");
    assert_eq!(config.sync.source_dir, "resources/i18n");
    assert_eq!(config.sync.working_dir, "/tmp");
    assert!(config.sync.target_languages.is_empty());
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

#[test]
fn config_read_modern_sync() {
    let config = Config::new_from_string(
        r#"
[vcs]
plugin = "git"
local_path = "/repo"
remote_path = "git@example/repo.git"

[sync]
source_language = "en"
target_languages = ["fr", "de"]
match_language_file = "\\.json$"
match_language_name = "(.+?)(\\.[^.]*$|$)"
source_dir = "resources/i18n"
working_dir = "/tmp/exports"
"#,
    )
    .unwrap();

    assert_eq!(config.sync.target_languages, vec!["fr", "de"]);
    assert_eq!(config.sync.working_dir, "/tmp/exports");
}
