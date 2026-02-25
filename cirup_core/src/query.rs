#![allow(clippy::self_named_module_files)]

use prettytable::{Cell, Row, Table};

use crate::config::{QueryBackendKind, QueryConfig};
use crate::file::save_resource_file;
use crate::query_backend::{QueryBackend, build_backend};

use crate::{Resource, Triple};

#[allow(clippy::print_stdout)]
pub fn print_resources_pretty(resources: &[Resource]) {
    let mut table: Table = Table::new();

    table.add_row(row!["name", "value"]); // table header

    for resource in resources {
        let mut row = Row::empty();
        row.add_cell(Cell::new(resource.name.as_str()));
        row.add_cell(Cell::new(resource.value.as_str()));
        table.add_row(row);
    }

    println!("{}", table);
}

#[allow(clippy::print_stdout)]
pub fn print_triples_pretty(triples: &[Triple]) {
    for triple in triples {
        println!("name: {}", triple.name);
        println!("base: {}", triple.base);
        println!("value: {}", triple.value);
        println!();
    }
}

fn default_query_backend() -> QueryBackendKind {
    std::env::var("CIRUP_QUERY_BACKEND")
        .ok()
        .and_then(|value| QueryBackendKind::parse(&value))
        .unwrap_or_default()
}

fn default_query_config() -> QueryConfig {
    let mut query_config = QueryConfig::default();
    query_config.backend = default_query_backend();

    query_config.turso.url = std::env::var("CIRUP_TURSO_URL")
        .ok()
        .or_else(|| std::env::var("LIBSQL_URL").ok())
        .or_else(|| std::env::var("LIBSQL_HRANA_URL").ok());
    query_config.turso.auth_token = std::env::var("CIRUP_TURSO_AUTH_TOKEN")
        .ok()
        .or_else(|| std::env::var("LIBSQL_AUTH_TOKEN").ok())
        .or_else(|| std::env::var("TURSO_AUTH_TOKEN").ok());

    query_config
}

pub fn query_file(input: &str, table: &str, query: &str) {
    let mut engine = CirupEngine::new();
    engine.register_table_from_file(table, input);
    let resources = engine.query_resource(query);
    print_resources_pretty(&resources);
}

pub struct CirupEngine {
    backend: Box<dyn QueryBackend>,
}

impl CirupEngine {
    pub fn new() -> Self {
        Self::with_query_config(&default_query_config())
    }

    pub fn with_backend(kind: QueryBackendKind) -> Self {
        let mut query_config = default_query_config();
        query_config.backend = kind;
        Self::with_query_config(&query_config)
    }

    pub fn with_query_config(query_config: &QueryConfig) -> Self {
        Self {
            backend: build_backend(query_config),
        }
    }

    #[cfg(test)]
    fn register_table_from_str(&mut self, table: &str, filename: &str, data: &str) {
        self.backend.register_table_from_str(table, filename, data);
    }

    pub fn register_table_from_file(&mut self, table: &str, filename: &str) {
        self.backend.register_table_from_file(table, filename);
    }

    pub fn query_resource(&self, query: &str) -> Vec<Resource> {
        self.backend.query_resource(query)
    }

    pub fn query_triple(&self, query: &str) -> Vec<Triple> {
        self.backend.query_triple(query)
    }
}

impl Default for CirupEngine {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CirupQuery {
    engine: CirupEngine,
    query: String,
}

const PRINT_QUERY: &str = "SELECT * FROM A";
const DIFF_QUERY: &str = r"
        SELECT A.key, A.val, B.val 
        FROM A 
        LEFT OUTER JOIN B ON A.key = B.key 
        WHERE (B.val IS NULL)";
const DIFF_WITH_BASE_QUERY: &str = r"
        SELECT B.key, B.val, C.val 
        FROM B 
        LEFT OUTER JOIN A ON B.key = A.key 
        INNER JOIN C ON B.key = C.key 
        WHERE (A.val IS NULL)";
const CHANGE_QUERY: &str = r"
        SELECT A.key, A.val, B.val 
        FROM A 
        LEFT OUTER JOIN B ON A.key = B.key 
        WHERE (B.val IS NULL) OR (A.val <> B.val)";
const MERGE_QUERY: &str = r"
        SELECT A.key, CASE WHEN B.val IS NOT NULL THEN B.val ELSE A.val END
        FROM A
        LEFT OUTER JOIN B on A.key = B.key
        UNION
        SELECT B.key, B.val
        FROM B
        LEFT OUTER JOIN A on A.key = B.key
        WHERE (A.key IS NULL)";
const INTERSECT_QUERY: &str = r"
        SELECT * FROM A 
        INTERSECT 
        SELECT * from B";
const SUBTRACT_QUERY: &str = r"
        SELECT * FROM A 
        WHERE A.key NOT IN 
            (SELECT B.key FROM B)";
const CONVERT_QUERY: &str = "SELECT * FROM A";
const SORT_QUERY: &str = "SELECT * FROM A ORDER BY A.key";

pub fn query_print(file: &str) -> CirupQuery {
    query_print_with_backend(file, default_query_backend())
}

pub fn query_print_with_backend(file: &str, backend: QueryBackendKind) -> CirupQuery {
    CirupQuery::new_with_backend(PRINT_QUERY, file, None, None, backend)
}

pub fn query_convert(file: &str) -> CirupQuery {
    query_convert_with_backend(file, default_query_backend())
}

pub fn query_convert_with_backend(file: &str, backend: QueryBackendKind) -> CirupQuery {
    CirupQuery::new_with_backend(CONVERT_QUERY, file, None, None, backend)
}

pub fn query_sort(file: &str) -> CirupQuery {
    query_sort_with_backend(file, default_query_backend())
}

pub fn query_sort_with_backend(file: &str, backend: QueryBackendKind) -> CirupQuery {
    CirupQuery::new_with_backend(SORT_QUERY, file, None, None, backend)
}

pub fn query_diff(file_one: &str, file_two: &str) -> CirupQuery {
    query_diff_with_backend(file_one, file_two, default_query_backend())
}

pub fn query_diff_with_backend(file_one: &str, file_two: &str, backend: QueryBackendKind) -> CirupQuery {
    CirupQuery::new_with_backend(DIFF_QUERY, file_one, Some(file_two), None, backend)
}

pub fn query_diff_with_config(file_one: &str, file_two: &str, query_config: &QueryConfig) -> CirupQuery {
    CirupQuery::new_with_query_config(DIFF_QUERY, file_one, Some(file_two), None, query_config)
}

pub fn query_diff_with_base(old: &str, new: &str, base: &str) -> CirupQuery {
    query_diff_with_base_with_backend(old, new, base, default_query_backend())
}

pub fn query_diff_with_base_with_backend(old: &str, new: &str, base: &str, backend: QueryBackendKind) -> CirupQuery {
    CirupQuery::new_with_backend(DIFF_WITH_BASE_QUERY, old, Some(new), Some(base), backend)
}

pub fn query_change(file_one: &str, file_two: &str) -> CirupQuery {
    query_change_with_backend(file_one, file_two, default_query_backend())
}

pub fn query_change_with_backend(file_one: &str, file_two: &str, backend: QueryBackendKind) -> CirupQuery {
    CirupQuery::new_with_backend(CHANGE_QUERY, file_one, Some(file_two), None, backend)
}

pub fn query_change_with_config(file_one: &str, file_two: &str, query_config: &QueryConfig) -> CirupQuery {
    CirupQuery::new_with_query_config(CHANGE_QUERY, file_one, Some(file_two), None, query_config)
}

pub fn query_merge(file_one: &str, file_two: &str) -> CirupQuery {
    query_merge_with_backend(file_one, file_two, default_query_backend())
}

pub fn query_merge_with_backend(file_one: &str, file_two: &str, backend: QueryBackendKind) -> CirupQuery {
    CirupQuery::new_with_backend(MERGE_QUERY, file_one, Some(file_two), None, backend)
}

pub fn query_merge_with_config(file_one: &str, file_two: &str, query_config: &QueryConfig) -> CirupQuery {
    CirupQuery::new_with_query_config(MERGE_QUERY, file_one, Some(file_two), None, query_config)
}

pub fn query_intersect(file_one: &str, file_two: &str) -> CirupQuery {
    query_intersect_with_backend(file_one, file_two, default_query_backend())
}

pub fn query_intersect_with_backend(file_one: &str, file_two: &str, backend: QueryBackendKind) -> CirupQuery {
    CirupQuery::new_with_backend(INTERSECT_QUERY, file_one, Some(file_two), None, backend)
}

pub fn query_subtract(file_one: &str, file_two: &str) -> CirupQuery {
    query_subtract_with_backend(file_one, file_two, default_query_backend())
}

pub fn query_subtract_with_backend(file_one: &str, file_two: &str, backend: QueryBackendKind) -> CirupQuery {
    CirupQuery::new_with_backend(SUBTRACT_QUERY, file_one, Some(file_two), None, backend)
}

impl CirupQuery {
    pub fn new(query: &str, file_one: &str, file_two: Option<&str>, file_three: Option<&str>) -> Self {
        Self::new_with_query_config(query, file_one, file_two, file_three, &default_query_config())
    }

    pub fn new_with_backend(
        query: &str,
        file_one: &str,
        file_two: Option<&str>,
        file_three: Option<&str>,
        backend: QueryBackendKind,
    ) -> Self {
        let mut query_config = default_query_config();
        query_config.backend = backend;
        Self::new_with_query_config(query, file_one, file_two, file_three, &query_config)
    }

    pub fn new_with_query_config(
        query: &str,
        file_one: &str,
        file_two: Option<&str>,
        file_three: Option<&str>,
        query_config: &QueryConfig,
    ) -> Self {
        let mut engine = CirupEngine::with_query_config(query_config);
        engine.register_table_from_file("A", file_one);

        if let Some(file_two) = file_two {
            engine.register_table_from_file("B", file_two);
        }

        if let Some(file_three) = file_three {
            engine.register_table_from_file("C", file_three);
        }

        CirupQuery {
            engine,
            query: query.to_owned(),
        }
    }

    pub fn run(&self) -> Vec<Resource> {
        self.engine.query_resource(&self.query)
    }

    pub fn run_triple(&self) -> Vec<Triple> {
        self.engine.query_triple(&self.query)
    }

    pub fn run_interactive(&self, out_file: Option<&str>) {
        let resources = self.run();

        if let Some(out_file) = out_file {
            save_resource_file(out_file, &resources);
        } else {
            print_resources_pretty(&resources);
        }
    }

    pub fn run_triple_interactive(&self) {
        let triples = self.run_triple();
        print_triples_pretty(&triples);
    }
}

#[cfg(test)]
use crate::file::load_resource_str;

#[test]
#[allow(clippy::self_named_module_files)]
fn test_query() {
    let mut engine = CirupEngine::new();
    engine.register_table_from_str("A", "test.json", include_str!("../test/test.json"));
    engine.register_table_from_str("B", "test.resx", include_str!("../test/test.resx"));

    // find the union of the two tables (merge strings)
    let resources = engine.query_resource("SELECT * FROM A UNION SELECT * from B");
    print_resources_pretty(&resources);

    assert_eq!(resources.len(), 6);

    // find the intersection of the two tables (common strings)
    let resources = engine.query_resource("SELECT * FROM A INTERSECT SELECT * from B");
    print_resources_pretty(&resources);

    assert_eq!(resources.len(), 3);
}

#[test]
fn test_query_subtract() {
    let mut engine = CirupEngine::new();

    engine.register_table_from_str("A", "test1A.restext", include_str!("../test/subtract/test1A.restext"));
    engine.register_table_from_str("B", "test1B.restext", include_str!("../test/subtract/test1B.restext"));
    let expected = match load_resource_str(include_str!("../test/subtract/test1C.restext"), "restext") {
        Ok(resources) => resources,
        Err(e) => panic!("failed to parse expected restext fixture: {}", e),
    };

    let actual = engine.query_resource("SELECT * FROM A WHERE A.key NOT IN (SELECT B.key FROM B)");
    assert_eq!(actual, expected);
}

#[test]
#[allow(clippy::self_named_module_files)]
fn test_query_diff_with_base() {
    let mut engine = CirupEngine::new();
    engine.register_table_from_str("A", "test_old.resx", include_str!("../test/test_old.resx"));
    engine.register_table_from_str("B", "test_new.resx", include_str!("../test/test_new.resx"));
    engine.register_table_from_str("C", "test.resx", include_str!("../test/test.resx"));

    let triples = engine.query_triple(DIFF_WITH_BASE_QUERY);

    assert_eq!(triples.len(), 2);
    assert_eq!(triples[0].name, String::from("lblYolo"));
    assert_eq!(triples[0].base, String::from("You only live once"));
    assert_eq!(triples[0].value, String::from("Juste une vie a vivre"));
}

#[test]
#[cfg(feature = "turso-rust")]
fn test_query_turso_remote_env_gated() {
    let remote_url = std::env::var("CIRUP_TURSO_URL")
        .ok()
        .or_else(|| std::env::var("LIBSQL_URL").ok())
        .or_else(|| std::env::var("LIBSQL_HRANA_URL").ok());

    let Some(remote_url) = remote_url else {
        return;
    };

    let remote_auth_token = std::env::var("CIRUP_TURSO_AUTH_TOKEN")
        .ok()
        .or_else(|| std::env::var("LIBSQL_AUTH_TOKEN").ok())
        .or_else(|| std::env::var("TURSO_AUTH_TOKEN").ok())
        .unwrap_or_default();

    let mut query_config = QueryConfig::default();
    query_config.backend = QueryBackendKind::TursoRemote;
    query_config.turso.url = Some(remote_url);
    if !remote_auth_token.is_empty() {
        query_config.turso.auth_token = Some(remote_auth_token);
    }

    let mut engine = CirupEngine::with_query_config(&query_config);
    engine.register_table_from_str("A", "test.json", include_str!("../test/test.json"));

    let mut actual = engine.query_resource("SELECT * FROM A ORDER BY A.key");
    let mut expected = match load_resource_str(include_str!("../test/test.json"), "json") {
        Ok(resources) => resources,
        Err(e) => panic!("failed to parse expected json fixture: {}", e),
    };

    actual.sort_by(|a, b| a.name.cmp(&b.name).then(a.value.cmp(&b.value)));
    expected.sort_by(|a, b| a.name.cmp(&b.name).then(a.value.cmp(&b.value)));

    assert_eq!(actual, expected);
}
