#![allow(clippy::self_named_module_files)]

use std::io;

use prettytable::{Cell, Row, Table};

use crate::config::{QueryBackendKind, QueryConfig};
use crate::file::{
    OutputEncoding, save_resource_file, save_resource_file_with_encoding, would_save_resource_file_with_encoding,
};
use crate::query_backend::{QueryBackend, build_backend};

use crate::{Resource, Triple};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QueryOutputFormat {
    Table,
    Json,
    #[default]
    Jsonl,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QueryRunOptions {
    pub output_format: QueryOutputFormat,
    pub count_only: bool,
    pub dry_run: bool,
    pub check: bool,
    pub summary: bool,
    pub key_prefixes: Vec<String>,
    pub key_contains: Vec<String>,
    pub limit: Option<usize>,
    pub operation_name: Option<String>,
    pub input_files: Vec<String>,
    pub output_file: Option<String>,
}

impl QueryRunOptions {
    #[must_use]
    pub fn with_context(mut self, operation_name: &str, input_files: &[&str], output_file: Option<&str>) -> Self {
        self.operation_name = Some(operation_name.to_owned());
        self.input_files = input_files.iter().map(|value| (*value).to_owned()).collect();
        self.output_file = output_file.map(str::to_owned);
        self
    }

    fn matches_name(&self, name: &str) -> bool {
        let prefix_match =
            self.key_prefixes.is_empty() || self.key_prefixes.iter().any(|prefix| name.starts_with(prefix));
        let contains_match =
            self.key_contains.is_empty() || self.key_contains.iter().any(|needle| name.contains(needle));

        prefix_match && contains_match
    }

    fn validate_for_output(&self, out_file: Option<&str>) -> Result<(), io::Error> {
        if self.count_only && out_file.is_some() {
            return Err(io::Error::other("--count-only cannot be combined with an output file"));
        }

        if self.count_only && self.summary {
            return Err(io::Error::other("--count-only cannot be combined with --summary"));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QueryExecutionCounts {
    matched_count: usize,
    filtered_count: usize,
    output_count: usize,
    truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct QueryExecutionReport {
    pub operation: Option<String>,
    pub result_kind: String,
    pub input_files: Vec<String>,
    pub output_file: Option<String>,
    pub matched_count: usize,
    pub filtered_count: usize,
    pub output_count: usize,
    pub truncated: bool,
    pub dry_run: bool,
    pub check: bool,
    pub would_write: bool,
    pub wrote_output: bool,
    pub change_detected: bool,
}

impl QueryExecutionReport {
    fn from_options(
        options: &QueryRunOptions,
        result_kind: &str,
        counts: QueryExecutionCounts,
        would_write: bool,
        wrote_output: bool,
    ) -> Self {
        let change_detected = if options.output_file.is_some() {
            would_write
        } else {
            counts.output_count > 0
        };

        Self {
            operation: options.operation_name.clone(),
            result_kind: result_kind.to_owned(),
            input_files: options.input_files.clone(),
            output_file: options.output_file.clone(),
            matched_count: counts.matched_count,
            filtered_count: counts.filtered_count,
            output_count: counts.output_count,
            truncated: counts.truncated,
            dry_run: options.dry_run,
            check: options.check,
            would_write,
            wrote_output,
            change_detected,
        }
    }

    pub fn indicates_change(&self) -> bool {
        self.change_detected
    }
}

fn ensure_trailing_newline(mut text: String) -> String {
    if !text.ends_with('\n') {
        text.push('\n');
    }

    text
}

fn resources_to_table(resources: &[Resource]) -> String {
    let mut table: Table = Table::new();

    table.add_row(row!["name", "value"]);

    for resource in resources {
        let mut row = Row::empty();
        row.add_cell(Cell::new(resource.name.as_str()));
        row.add_cell(Cell::new(resource.value.as_str()));
        table.add_row(row);
    }

    ensure_trailing_newline(table.to_string())
}

fn triples_to_table(triples: &[Triple]) -> String {
    let mut table: Table = Table::new();

    table.add_row(row!["name", "value", "base"]);

    for triple in triples {
        let mut row = Row::empty();
        row.add_cell(Cell::new(triple.name.as_str()));
        row.add_cell(Cell::new(triple.value.as_str()));
        row.add_cell(Cell::new(triple.base.as_str()));
        table.add_row(row);
    }

    ensure_trailing_newline(table.to_string())
}

fn render_jsonl<T: serde::Serialize>(values: &[T]) -> String {
    let mut output = String::new();

    for value in values {
        output.push_str(&serde_json::to_string(value).expect("failed to serialize JSONL row"));
        output.push('\n');
    }

    output
}

fn render_resources(resources: &[Resource], output_format: QueryOutputFormat) -> String {
    match output_format {
        QueryOutputFormat::Table => resources_to_table(resources),
        QueryOutputFormat::Json => ensure_trailing_newline(
            serde_json::to_string(resources).expect("failed to serialize resource list to JSON"),
        ),
        QueryOutputFormat::Jsonl => render_jsonl(resources),
    }
}

fn render_triples(triples: &[Triple], output_format: QueryOutputFormat) -> String {
    match output_format {
        QueryOutputFormat::Table => triples_to_table(triples),
        QueryOutputFormat::Json => {
            ensure_trailing_newline(serde_json::to_string(triples).expect("failed to serialize triple list to JSON"))
        }
        QueryOutputFormat::Jsonl => render_jsonl(triples),
    }
}

fn render_count(count: usize) -> String {
    format!("{count}\n")
}

fn report_to_table(report: &QueryExecutionReport) -> String {
    let mut table: Table = Table::new();

    table.add_row(row!["field", "value"]);

    let rows = [
        ("operation", report.operation.as_deref().unwrap_or_default().to_owned()),
        ("result_kind", report.result_kind.clone()),
        ("input_files", report.input_files.join(",")),
        ("output_file", report.output_file.clone().unwrap_or_default()),
        ("matched_count", report.matched_count.to_string()),
        ("filtered_count", report.filtered_count.to_string()),
        ("output_count", report.output_count.to_string()),
        ("truncated", report.truncated.to_string()),
        ("dry_run", report.dry_run.to_string()),
        ("check", report.check.to_string()),
        ("would_write", report.would_write.to_string()),
        ("wrote_output", report.wrote_output.to_string()),
        ("change_detected", report.change_detected.to_string()),
    ];

    for (field, value) in rows {
        let mut row = Row::empty();
        row.add_cell(Cell::new(field));
        row.add_cell(Cell::new(value.as_str()));
        table.add_row(row);
    }

    ensure_trailing_newline(table.to_string())
}

fn render_report(report: &QueryExecutionReport, output_format: QueryOutputFormat) -> String {
    match output_format {
        QueryOutputFormat::Table => report_to_table(report),
        QueryOutputFormat::Json => ensure_trailing_newline(
            serde_json::to_string(report).expect("failed to serialize execution report to JSON"),
        ),
        QueryOutputFormat::Jsonl => render_jsonl(std::slice::from_ref(report)),
    }
}

fn filter_resources(mut resources: Vec<Resource>, options: &QueryRunOptions) -> (QueryExecutionCounts, Vec<Resource>) {
    let matched_count = resources.len();
    resources.retain(|resource| options.matches_name(&resource.name));
    let filtered_count = resources.len();
    let mut truncated = false;

    if let Some(limit) = options.limit {
        truncated = filtered_count > limit;
        resources.truncate(limit);
    }

    (
        QueryExecutionCounts {
            matched_count,
            filtered_count,
            output_count: resources.len(),
            truncated,
        },
        resources,
    )
}

fn filter_triples(mut triples: Vec<Triple>, options: &QueryRunOptions) -> (QueryExecutionCounts, Vec<Triple>) {
    let matched_count = triples.len();
    triples.retain(|triple| options.matches_name(&triple.name));
    let filtered_count = triples.len();
    let mut truncated = false;

    if let Some(limit) = options.limit {
        truncated = filtered_count > limit;
        triples.truncate(limit);
    }

    (
        QueryExecutionCounts {
            matched_count,
            filtered_count,
            output_count: triples.len(),
            truncated,
        },
        triples,
    )
}

#[allow(clippy::print_stdout)]
pub fn print_resources_pretty(resources: &[Resource]) {
    print!("{}", resources_to_table(resources));
}

#[allow(clippy::print_stdout)]
pub fn print_triples_pretty(triples: &[Triple]) {
    print!("{}", triples_to_table(triples));
}

fn default_query_backend() -> QueryBackendKind {
    std::env::var("CIRUP_QUERY_BACKEND")
        .ok()
        .and_then(|value| QueryBackendKind::parse(&value))
        .unwrap_or_default()
}

fn default_query_config() -> QueryConfig {
    let mut query_config = QueryConfig {
        backend: default_query_backend(),
        ..QueryConfig::default()
    };

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

    pub fn run_with_options(&self, options: &QueryRunOptions) -> Vec<Resource> {
        let (_, resources) = filter_resources(self.run(), options);
        resources
    }

    pub fn run_triple_with_options(&self, options: &QueryRunOptions) -> Vec<Triple> {
        let (_, triples) = filter_triples(self.run_triple(), options);
        triples
    }

    pub fn run_interactive(&self, out_file: Option<&str>, touch: bool) {
        let resources = self.run();

        if let Some(out_file) = out_file {
            save_resource_file(out_file, &resources, touch);
        } else {
            print_resources_pretty(&resources);
        }
    }

    pub fn run_interactive_with_encoding(&self, out_file: Option<&str>, touch: bool, output_encoding: OutputEncoding) {
        let resources = self.run();

        if let Some(out_file) = out_file {
            save_resource_file_with_encoding(out_file, &resources, touch, output_encoding);
        } else {
            print_resources_pretty(&resources);
        }
    }

    #[allow(clippy::print_stdout)]
    pub fn run_interactive_with_options(
        &self,
        out_file: Option<&str>,
        touch: bool,
        output_encoding: OutputEncoding,
        options: &QueryRunOptions,
    ) -> Result<QueryExecutionReport, io::Error> {
        options.validate_for_output(out_file)?;

        let (counts, resources) = filter_resources(self.run(), options);
        let would_write = out_file
            .map(|path| would_save_resource_file_with_encoding(path, &resources, touch, output_encoding))
            .unwrap_or(false);
        let mut wrote_output = false;
        let report = QueryExecutionReport::from_options(options, "resource", counts, would_write, false);

        if options.count_only {
            print!("{}", render_count(counts.output_count));
            return Ok(report);
        }

        if options.check {
            if options.summary {
                print!("{}", render_report(&report, options.output_format));
            }
            return Ok(report);
        }

        if let Some(out_file) = out_file {
            if options.dry_run {
                if !options.summary {
                    print!("{}", render_resources(&resources, options.output_format));
                }
            } else {
                save_resource_file_with_encoding(out_file, &resources, touch, output_encoding);
                wrote_output = would_write;
            }
        } else if !options.summary {
            print!("{}", render_resources(&resources, options.output_format));
        }

        let report = QueryExecutionReport::from_options(options, "resource", counts, would_write, wrote_output);

        if options.summary {
            print!("{}", render_report(&report, options.output_format));
        }

        Ok(report)
    }

    pub fn run_triple_interactive(&self) {
        let triples = self.run_triple();
        print_triples_pretty(&triples);
    }

    #[allow(clippy::print_stdout)]
    pub fn run_triple_interactive_with_options(
        &self,
        options: &QueryRunOptions,
    ) -> Result<QueryExecutionReport, io::Error> {
        options.validate_for_output(None)?;

        let (counts, triples) = filter_triples(self.run_triple(), options);
        let report = QueryExecutionReport::from_options(options, "triple", counts, false, false);

        if options.count_only {
            print!("{}", render_count(counts.output_count));
            return Ok(report);
        }

        if options.check {
            if options.summary {
                print!("{}", render_report(&report, options.output_format));
            }
            return Ok(report);
        }

        if !options.summary {
            print!("{}", render_triples(&triples, options.output_format));
        }

        if options.summary {
            print!("{}", render_report(&report, options.output_format));
        }

        Ok(report)
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

    let mut query_config = QueryConfig {
        backend: QueryBackendKind::TursoRemote,
        ..QueryConfig::default()
    };
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

#[test]
fn test_query_run_options_filter_and_limit_resources() {
    let query = query_print_with_backend("test.json", QueryBackendKind::Rusqlite);
    let options = QueryRunOptions {
        key_prefixes: vec![String::from("lbl")],
        key_contains: vec![String::from("Yolo")],
        limit: Some(1),
        ..QueryRunOptions::default()
    };

    let resources = query.run_with_options(&options);

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].name, String::from("lblYolo"));
}

#[test]
fn test_render_resources_jsonl() {
    let resources = vec![Resource::new("hello", "world"), Resource::new("goodbye", "moon")];
    let output = render_resources(&resources, QueryOutputFormat::Jsonl);

    assert_eq!(
        output,
        "{\"name\":\"hello\",\"value\":\"world\"}\n{\"name\":\"goodbye\",\"value\":\"moon\"}\n"
    );
}

#[test]
fn test_render_triples_json() {
    let triples = vec![Triple::new("hello", "world", "base")];
    let output = render_triples(&triples, QueryOutputFormat::Json);

    assert_eq!(output, "[{\"name\":\"hello\",\"value\":\"world\",\"base\":\"base\"}]\n");
}

#[test]
fn test_count_only_rejects_output_file() {
    let options = QueryRunOptions {
        count_only: true,
        ..QueryRunOptions::default()
    };

    let error = options
        .validate_for_output(Some("out.json"))
        .expect_err("expected validation error");
    assert_eq!(error.to_string(), "--count-only cannot be combined with an output file");
}

#[test]
fn test_summary_rejects_count_only() {
    let options = QueryRunOptions {
        count_only: true,
        summary: true,
        ..QueryRunOptions::default()
    };

    let error = options
        .validate_for_output(None)
        .expect_err("expected validation error");
    assert_eq!(error.to_string(), "--count-only cannot be combined with --summary");
}

#[test]
fn test_report_detects_change_for_stdout_results() {
    let report = QueryExecutionReport::from_options(
        &QueryRunOptions::default().with_context("file-diff", &["a.json", "b.json"], None),
        "resource",
        QueryExecutionCounts {
            matched_count: 3,
            filtered_count: 2,
            output_count: 2,
            truncated: false,
        },
        false,
        false,
    );

    assert!(report.indicates_change());
}

#[test]
fn test_report_renders_as_json_summary() {
    let report = QueryExecutionReport::from_options(
        &QueryRunOptions::default().with_context("file-sort", &["a.json"], Some("a.json")),
        "resource",
        QueryExecutionCounts {
            matched_count: 4,
            filtered_count: 4,
            output_count: 4,
            truncated: false,
        },
        true,
        false,
    );

    let output = render_report(&report, QueryOutputFormat::Json);

    assert!(output.contains("\"operation\":\"file-sort\""));
    assert!(output.contains("\"would_write\":true"));
    assert!(output.ends_with('\n'));
}
