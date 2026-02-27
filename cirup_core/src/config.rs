use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Default)]
#[serde(rename_all = "kebab-case")]
pub enum QueryBackendKind {
    Rusqlite,
    #[default]
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

#[test]
fn query_backend_kind_parse_aliases() {
    assert_eq!(QueryBackendKind::parse("rusqlite"), Some(QueryBackendKind::Rusqlite));
    assert_eq!(QueryBackendKind::parse("turso"), Some(QueryBackendKind::TursoLocal));
    assert_eq!(
        QueryBackendKind::parse("turso_remote"),
        Some(QueryBackendKind::TursoRemote)
    );
    assert_eq!(QueryBackendKind::parse("unknown"), None);
}
