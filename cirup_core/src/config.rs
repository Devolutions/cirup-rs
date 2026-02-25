use std::error::Error;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use toml;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Default)]
#[serde(rename_all = "kebab-case")]
pub enum QueryBackendKind {
    #[default]
    Rusqlite,
    TursoLocal,
    TursoRemote,
}

impl QueryBackendKind {
    pub fn parse(value: &str) -> Option<Self> {
        Self::from_str(value).ok()
    }
}

impl FromStr for QueryBackendKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "rusqlite" => Ok(QueryBackendKind::Rusqlite),
            "turso-local" | "turso_local" | "turso" => Ok(QueryBackendKind::TursoLocal),
            "turso-remote" | "turso_remote" | "libsql-remote" | "libsql_remote" => Ok(QueryBackendKind::TursoRemote),
            _ => Err(format!(
                "unsupported query backend '{}': expected one of rusqlite, turso-local, turso-remote",
                value
            )),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct TursoConfig {
    pub url: Option<String>,
    pub auth_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct QueryConfig {
    #[serde(default)]
    pub backend: QueryBackendKind,
    #[serde(default)]
    pub turso: TursoConfig,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub vcs: Vcs,
    #[serde(alias = "job")]
    pub sync: Sync,
    #[serde(default)]
    pub query: QueryConfig,
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
        let config: Config = toml::from_str(contents)?;
        Ok(config)
    }
}

#[test]
fn config_read() {
    let config = match Config::new_from_string(include_str!("../test/config.cirup")) {
        Ok(config) => config,
        Err(e) => panic!("failed to parse config fixture: {}", e),
    };
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
            plugin: "git".to_owned(),
            local_path: "xxx".to_owned(),
            remote_path: "yyy".to_owned(),
        },
        sync: Sync {
            source_language: "en".to_owned(),
            target_languages: vec!["fr".to_owned(), "de".to_owned()],
            match_language_file: "\\.json$".to_owned(),
            match_language_name: "(.+?)(\\.[^.]*$|$)".to_owned(),
            source_dir: "xxx".to_owned(),
            working_dir: "xxx".to_owned(),
        },
        query: QueryConfig::default(),
    };

    let toml = match toml::to_string(&config) {
        Ok(toml) => toml,
        Err(e) => panic!("failed to serialize config: {}", e),
    };
    assert!(!toml.is_empty());
}

#[test]
fn config_read_modern_sync() {
    let config = match Config::new_from_string(
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
    ) {
        Ok(config) => config,
        Err(e) => panic!("failed to parse modern sync config: {}", e),
    };

    assert_eq!(config.sync.target_languages, vec!["fr", "de"]);
    assert_eq!(config.sync.working_dir, "/tmp/exports");
}

#[test]
fn config_read_query_backend() {
    let config = match Config::new_from_string(
        r#"
[vcs]
plugin = "git"
local_path = "/repo"
remote_path = "git@example/repo.git"

[sync]
source_language = "en"
match_language_file = "\\.json$"
match_language_name = "(.+?)(\\.[^.]*$|$)"
source_dir = "resources/i18n"
working_dir = "/tmp/exports"

[query]
backend = "turso-local"

[query.turso]
url = "libsql://acme.turso.io"
auth_token = "token"
"#,
    ) {
        Ok(config) => config,
        Err(e) => panic!("failed to parse query backend config: {}", e),
    };

    assert_eq!(config.query.backend, QueryBackendKind::TursoLocal);
    assert_eq!(config.query.turso.url.as_deref(), Some("libsql://acme.turso.io"));
    assert_eq!(config.query.turso.auth_token.as_deref(), Some("token"));
}
