#[cfg(feature = "turso-rust")]
use crate::config::TursoConfig;
use crate::config::{QueryBackendKind, QueryConfig};
use crate::file::load_resource_file;
#[cfg(test)]
use crate::file::vfile_set;
use crate::{Resource, Triple};

use rusqlite::{Connection, Error as SqlError, Statement};

#[cfg(feature = "turso-rust")]
use std::cell::RefCell;
#[cfg(feature = "turso-rust")]
use std::collections::{HashMap, HashSet};

#[cfg(feature = "turso-rust")]
use libsql::{
    Builder as LibsqlBuilder, Connection as LibsqlConnection, Database as LibsqlDatabase, Error as LibsqlError,
};
#[cfg(feature = "turso-rust")]
use tokio::runtime::{Builder as RuntimeBuilder, Runtime};
#[cfg(feature = "turso-rust")]
use turso::{Builder as TursoBuilder, Connection as TursoConnection, Database as TursoDatabase, Error as TursoError};

pub(crate) trait QueryBackend {
    #[cfg(test)]
    fn register_table_from_str(&mut self, table: &str, filename: &str, data: &str);
    fn register_table_from_file(&mut self, table: &str, filename: &str);
    fn query_resource(&self, query: &str) -> Vec<Resource>;
    fn query_triple(&self, query: &str) -> Vec<Triple>;
}

fn valid_table_name(table: &str) -> bool {
    let mut chars = table.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }

    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn load_resources(filename: &str) -> Vec<Resource> {
    match load_resource_file(filename) {
        Ok(resources) => resources,
        Err(e) => {
            error!("failed to load {}: {}", filename, e);
            Vec::new()
        }
    }
}

#[cfg(feature = "turso-rust")]
const TURSO_INSERT_CHUNK_SIZE: usize = 2000;

#[cfg(feature = "turso-rust")]
const QUERY_SELECT_A: &str = "select * from a";
#[cfg(feature = "turso-rust")]
const QUERY_SORT_A: &str = "select * from a order by a.key";
#[cfg(feature = "turso-rust")]
const QUERY_DIFF: &str = "select a.key, a.val, b.val from a left outer join b on a.key = b.key where (b.val is null)";
#[cfg(feature = "turso-rust")]
const QUERY_DIFF_WITH_BASE: &str =
    "select b.key, b.val, c.val from b left outer join a on b.key = a.key inner join c on b.key = c.key where (a.val is null)";
#[cfg(feature = "turso-rust")]
const QUERY_CHANGE: &str =
    "select a.key, a.val, b.val from a left outer join b on a.key = b.key where (b.val is null) or (a.val <> b.val)";
#[cfg(feature = "turso-rust")]
const QUERY_MERGE: &str = "select a.key, case when b.val is not null then b.val else a.val end from a left outer join b on a.key = b.key union select b.key, b.val from b left outer join a on a.key = b.key where (a.key is null)";
#[cfg(feature = "turso-rust")]
const QUERY_INTERSECT: &str = "select * from a intersect select * from b";
#[cfg(feature = "turso-rust")]
const QUERY_SUBTRACT: &str = "select * from a where a.key not in (select b.key from b)";
#[cfg(feature = "turso-rust")]
const QUERY_PULL_LEFT_JOIN: &str = "select a.key, a.val from a left outer join b on a.key = b.key";
#[cfg(feature = "turso-rust")]
const QUERY_PUSH_CHANGED_VALUES: &str =
    "select b.key, b.val from b inner join a on (a.key = b.key) and (a.val <> b.val)";

#[cfg(feature = "turso-rust")]
fn append_sql_quoted(out: &mut String, value: &str) {
    out.push('\'');
    for ch in value.chars() {
        if ch == '\'' {
            out.push('\'');
            out.push('\'');
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
}

#[cfg(feature = "turso-rust")]
fn build_multi_insert_sql(table: &str, resources: &[Resource], out: &mut String) {
    out.clear();
    out.push_str("INSERT INTO ");
    out.push_str(table);
    out.push_str(" (key, val) VALUES ");

    for (index, resource) in resources.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }

        out.push('(');
        append_sql_quoted(out, &resource.name);
        out.push(',');
        append_sql_quoted(out, &resource.value);
        out.push(')');
    }

    out.push(';');
}

#[cfg(feature = "turso-rust")]
fn build_key_index_sql(table: &str) -> String {
    format!("CREATE INDEX IF NOT EXISTS idx_{table}_key ON {table} (key);")
}

#[cfg(feature = "turso-rust")]
fn canonical_sql(input: &str) -> String {
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

#[cfg(feature = "turso-rust")]
fn remote_url_from_config(turso_config: &TursoConfig) -> Option<String> {
    turso_config.url.clone().or_else(|| {
        std::env::var("CIRUP_TURSO_URL")
            .ok()
            .or_else(|| std::env::var("LIBSQL_URL").ok())
            .or_else(|| std::env::var("LIBSQL_HRANA_URL").ok())
    })
}

#[cfg(feature = "turso-rust")]
fn remote_auth_token_from_config(turso_config: &TursoConfig) -> String {
    turso_config.auth_token.clone().unwrap_or_else(|| {
        std::env::var("CIRUP_TURSO_AUTH_TOKEN")
            .ok()
            .or_else(|| std::env::var("LIBSQL_AUTH_TOKEN").ok())
            .or_else(|| std::env::var("TURSO_AUTH_TOKEN").ok())
            .unwrap_or_default()
    })
}

fn query_resource_from_statement(statement: &mut Statement<'_>) -> Vec<Resource> {
    let mut resources: Vec<Resource> = Vec::new();
    let mut response = match statement.query(&[]) {
        Ok(response) => response,
        Err(e) => {
            error!("query failed: {}", e);
            return resources;
        }
    };

    while let Some(v) = response.next() {
        if let Ok(res) = v {
            let name = &res.get::<usize, String>(0);
            let value = &res.get::<usize, String>(1);
            let resource = Resource::new(name, value);
            resources.push(resource);
        }
    }

    resources
}

fn query_triple_from_statement(statement: &mut Statement<'_>) -> Vec<Triple> {
    let mut resources: Vec<Triple> = Vec::new();
    let mut response = match statement.query(&[]) {
        Ok(response) => response,
        Err(e) => {
            error!("query failed: {}", e);
            return resources;
        }
    };

    while let Some(v) = response.next() {
        if let Ok(res) = v {
            let name = &res.get::<usize, String>(0);
            let value = &res.get::<usize, String>(1);
            let base = &res.get::<usize, String>(2);
            let resource = Triple::new(name, value, base);
            resources.push(resource);
        }
    }

    resources
}

pub(crate) struct RusqliteBackend {
    db: Connection,
}

impl RusqliteBackend {
    pub(crate) fn new() -> Self {
        let db = Connection::open_in_memory().expect("failed to open in-memory database");
        Self { db }
    }

    fn register_table_with_resources(&mut self, table: &str, resources: &[Resource]) {
        if !valid_table_name(table) {
            error!("invalid table name {}", table);
            return;
        }

        let sql = format!("DROP TABLE IF EXISTS {table}; CREATE TABLE {table} (key TEXT, val TEXT)");

        if let Err(e) = self.db.execute_batch(&sql) {
            error!("failed to initialize table {}: {}", table, e);
            return;
        }

        let insert_sql = format!("INSERT INTO {table} (key, val) VALUES (?1, ?2)");

        let tx = match self.db.transaction() {
            Ok(tx) => tx,
            Err(e) => {
                error!("failed to start transaction for {}: {}", table, e);
                return;
            }
        };

        {
            let mut statement = match tx.prepare(&insert_sql) {
                Ok(statement) => statement,
                Err(e) => {
                    error!("failed to prepare insert statement for {}: {}", table, e);
                    return;
                }
            };

            for resource in resources {
                if let Err(e) = statement.execute(&[&resource.name, &resource.value]) {
                    error!("failed to insert resource into {}: {}", table, e);
                    return;
                }
            }
        }

        if let Err(e) = tx.commit() {
            error!("failed to commit transaction for {}: {}", table, e);
        }
    }

    fn prepare_statement(&self, query: &str) -> Result<Statement<'_>, SqlError> {
        self.db.prepare(query)
    }
}

impl QueryBackend for RusqliteBackend {
    #[cfg(test)]
    fn register_table_from_str(&mut self, table: &str, filename: &str, data: &str) {
        vfile_set(filename, data);
        let resources = load_resources(filename);
        self.register_table_with_resources(table, &resources);
    }

    fn register_table_from_file(&mut self, table: &str, filename: &str) {
        let resources = load_resources(filename);
        self.register_table_with_resources(table, &resources);
    }

    fn query_resource(&self, query: &str) -> Vec<Resource> {
        let mut statement = match self.prepare_statement(query) {
            Ok(statement) => statement,
            Err(e) => {
                error!("failed to prepare query: {}", e);
                return Vec::new();
            }
        };

        query_resource_from_statement(&mut statement)
    }

    fn query_triple(&self, query: &str) -> Vec<Triple> {
        let mut statement = match self.prepare_statement(query) {
            Ok(statement) => statement,
            Err(e) => {
                error!("failed to prepare query: {}", e);
                return Vec::new();
            }
        };

        query_triple_from_statement(&mut statement)
    }
}

#[cfg(feature = "turso-rust")]
pub(crate) struct TursoLocalBackend {
    runtime: Runtime,
    _db: TursoDatabase,
    conn: TursoConnection,
    tables: HashMap<String, Vec<Resource>>,
    loaded_tables: RefCell<HashSet<String>>,
}

#[cfg(feature = "turso-rust")]
impl TursoLocalBackend {
    pub(crate) fn new() -> Self {
        let runtime = RuntimeBuilder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");

        let db = runtime
            .block_on(async { TursoBuilder::new_local(":memory:").build().await })
            .expect("failed to create turso local database");

        let conn = db.connect().expect("failed to connect to turso local database");

        Self {
            runtime,
            _db: db,
            conn,
            tables: HashMap::new(),
            loaded_tables: RefCell::new(HashSet::new()),
        }
    }

    fn register_table_with_resources(&mut self, table: &str, resources: &[Resource]) -> Result<(), TursoError> {
        if !valid_table_name(table) {
            error!("invalid table name {}", table);
            return Ok(());
        }

        self.tables.insert(table.to_owned(), resources.to_vec());
        self.loaded_tables.borrow_mut().remove(table);

        Ok(())
    }

    fn materialize_table_with_resources(&self, table: &str, resources: &[Resource]) -> Result<(), TursoError> {
        let sql = format!("DROP TABLE IF EXISTS {table}; CREATE TABLE {table} (key TEXT, val TEXT);");
        self.runtime.block_on(async { self.conn.execute_batch(&sql).await })?;

        if resources.is_empty() {
            return Ok(());
        }

        self.runtime.block_on(async {
            self.conn.execute("BEGIN", ()).await?;

            let mut insert_sql = String::new();

            for chunk in resources.chunks(TURSO_INSERT_CHUNK_SIZE) {
                build_multi_insert_sql(table, chunk, &mut insert_sql);

                if let Err(e) = self.conn.execute_batch(&insert_sql).await {
                    let _ = self.conn.execute("ROLLBACK", ()).await;
                    return Err(e);
                }
            }

            let index_sql = build_key_index_sql(table);
            if let Err(e) = self.conn.execute_batch(&index_sql).await {
                let _ = self.conn.execute("ROLLBACK", ()).await;
                return Err(e);
            }

            self.conn.execute("COMMIT", ()).await?;
            Ok(())
        })
    }

    fn materialize_cached_tables(&self) -> Result<(), TursoError> {
        let table_names = self.tables.keys().cloned().collect::<Vec<_>>();

        for table in table_names {
            let needs_materialization = {
                let loaded_tables = self.loaded_tables.borrow();
                !loaded_tables.contains(&table)
            };

            if !needs_materialization {
                continue;
            }

            let Some(resources) = self.tables.get(&table) else {
                continue;
            };

            self.materialize_table_with_resources(&table, resources)?;
            self.loaded_tables.borrow_mut().insert(table);
        }

        Ok(())
    }

    fn query_resource_fast(&self, query: &str) -> Option<Vec<Resource>> {
        let query = canonical_sql(query);

        let table_a = self.tables.get("A");
        let table_b = self.tables.get("B");

        if query == QUERY_SELECT_A {
            return table_a.cloned();
        }

        if query == QUERY_SORT_A {
            let mut resources = table_a?.clone();
            resources.sort_by(|left, right| left.name.cmp(&right.name));
            return Some(resources);
        }

        if query == QUERY_DIFF {
            let a = table_a?;
            let b = table_b?;
            let b_keys: HashSet<&str> = b.iter().map(|resource| resource.name.as_str()).collect();

            let resources = a
                .iter()
                .filter(|resource| !b_keys.contains(resource.name.as_str()))
                .cloned()
                .collect::<Vec<_>>();

            return Some(resources);
        }

        if query == QUERY_CHANGE {
            let a = table_a?;
            let b = table_b?;
            let b_values: HashMap<&str, &str> = b
                .iter()
                .map(|resource| (resource.name.as_str(), resource.value.as_str()))
                .collect();

            let resources = a
                .iter()
                .filter(|resource| {
                    let key = resource.name.as_str();
                    let value = resource.value.as_str();
                    match b_values.get(key) {
                        None => true,
                        Some(other) => *other != value,
                    }
                })
                .cloned()
                .collect::<Vec<_>>();

            return Some(resources);
        }

        if query == QUERY_MERGE {
            let a = table_a?;
            let b = table_b?;

            let a_values: HashMap<&str, &str> = a
                .iter()
                .map(|resource| (resource.name.as_str(), resource.value.as_str()))
                .collect();
            let b_values: HashMap<&str, &str> = b
                .iter()
                .map(|resource| (resource.name.as_str(), resource.value.as_str()))
                .collect();

            let mut resources = Vec::with_capacity(a.len() + b.len());
            let mut dedupe: HashSet<(String, String)> = HashSet::with_capacity(a.len() + b.len());

            for resource in a {
                let key = resource.name.as_str();
                let merged_value = b_values.get(key).copied().unwrap_or(resource.value.as_str());
                let merged = Resource::new(key, merged_value);
                if dedupe.insert((merged.name.clone(), merged.value.clone())) {
                    resources.push(merged);
                }
            }

            for resource in b {
                let key = resource.name.as_str();
                if !a_values.contains_key(key) {
                    if dedupe.insert((resource.name.clone(), resource.value.clone())) {
                        resources.push(resource.clone());
                    }
                }
            }

            return Some(resources);
        }

        if query == QUERY_INTERSECT {
            let a = table_a?;
            let b = table_b?;
            let b_pairs: HashSet<(&str, &str)> = b
                .iter()
                .map(|resource| (resource.name.as_str(), resource.value.as_str()))
                .collect();

            let mut resources = Vec::new();
            let mut dedupe: HashSet<(String, String)> = HashSet::new();

            for resource in a {
                let pair = (resource.name.as_str(), resource.value.as_str());
                if b_pairs.contains(&pair)
                    && dedupe.insert((resource.name.clone(), resource.value.clone()))
                {
                    resources.push(resource.clone());
                }
            }

            return Some(resources);
        }

        if query == QUERY_SUBTRACT {
            let a = table_a?;
            let b = table_b?;
            let b_keys: HashSet<&str> = b.iter().map(|resource| resource.name.as_str()).collect();

            let resources = a
                .iter()
                .filter(|resource| !b_keys.contains(resource.name.as_str()))
                .cloned()
                .collect::<Vec<_>>();

            return Some(resources);
        }

        if query == QUERY_PULL_LEFT_JOIN {
            let a = table_a?;
            let b = table_b?;

            let mut b_match_count: HashMap<&str, usize> = HashMap::new();
            for resource in b {
                *b_match_count.entry(resource.name.as_str()).or_insert(0) += 1;
            }

            let mut resources = Vec::new();
            for resource in a {
                let repeat = b_match_count
                    .get(resource.name.as_str())
                    .copied()
                    .unwrap_or(1);
                for _ in 0..repeat {
                    resources.push(resource.clone());
                }
            }

            return Some(resources);
        }

        if query == QUERY_PUSH_CHANGED_VALUES {
            let a = table_a?;
            let b = table_b?;

            let mut a_values: HashMap<&str, Vec<&str>> = HashMap::new();
            for resource in a {
                a_values
                    .entry(resource.name.as_str())
                    .or_default()
                    .push(resource.value.as_str());
            }

            let mut resources = Vec::new();
            for resource in b {
                let key = resource.name.as_str();
                let Some(left_values) = a_values.get(key) else {
                    continue;
                };

                for left_value in left_values {
                    if *left_value != resource.value.as_str() {
                        resources.push(resource.clone());
                    }
                }
            }

            return Some(resources);
        }

        None
    }

    fn query_triple_fast(&self, query: &str) -> Option<Vec<Triple>> {
        let query = canonical_sql(query);
        if query != QUERY_DIFF_WITH_BASE {
            return None;
        }

        let a = self.tables.get("A")?;
        let b = self.tables.get("B")?;
        let c = self.tables.get("C")?;

        let a_keys: HashSet<&str> = a.iter().map(|resource| resource.name.as_str()).collect();
        let c_values: HashMap<&str, &str> = c
            .iter()
            .map(|resource| (resource.name.as_str(), resource.value.as_str()))
            .collect();

        let mut triples = Vec::new();
        for resource in b {
            let key = resource.name.as_str();
            if !a_keys.contains(key) {
                if let Some(base) = c_values.get(key) {
                    triples.push(Triple::new(key, resource.value.as_str(), base));
                }
            }
        }

        Some(triples)
    }
}

#[cfg(feature = "turso-rust")]
impl QueryBackend for TursoLocalBackend {
    #[cfg(test)]
    fn register_table_from_str(&mut self, table: &str, filename: &str, data: &str) {
        vfile_set(filename, data);
        let resources = load_resources(filename);
        if let Err(e) = self.register_table_with_resources(table, &resources) {
            error!("failed to register table {} in turso local backend: {}", table, e);
        }
    }

    fn register_table_from_file(&mut self, table: &str, filename: &str) {
        let resources = load_resources(filename);
        if let Err(e) = self.register_table_with_resources(table, &resources) {
            error!("failed to register table {} in turso local backend: {}", table, e);
        }
    }

    fn query_resource(&self, query: &str) -> Vec<Resource> {
        if let Some(resources) = self.query_resource_fast(query) {
            return resources;
        }

        if let Err(e) = self.materialize_cached_tables() {
            error!("failed to materialize cached tables in turso local backend: {}", e);
            return Vec::new();
        }

        match self.runtime.block_on(async {
            let mut statement = self.conn.prepare(query).await?;
            let mut rows = statement.query(()).await?;
            let mut resources: Vec<Resource> = Vec::new();

            while let Some(row) = rows.next().await? {
                let name: String = row.get(0)?;
                let value: String = row.get(1)?;
                resources.push(Resource::new(&name, &value));
            }

            Ok::<Vec<Resource>, TursoError>(resources)
        }) {
            Ok(resources) => resources,
            Err(e) => {
                error!("failed to run query in turso local backend: {}", e);
                Vec::new()
            }
        }
    }

    fn query_triple(&self, query: &str) -> Vec<Triple> {
        if let Some(triples) = self.query_triple_fast(query) {
            return triples;
        }

        if let Err(e) = self.materialize_cached_tables() {
            error!("failed to materialize cached tables in turso local backend: {}", e);
            return Vec::new();
        }

        match self.runtime.block_on(async {
            let mut statement = self.conn.prepare(query).await?;
            let mut rows = statement.query(()).await?;
            let mut triples: Vec<Triple> = Vec::new();

            while let Some(row) = rows.next().await? {
                let name: String = row.get(0)?;
                let value: String = row.get(1)?;
                let base: String = row.get(2)?;
                triples.push(Triple::new(&name, &value, &base));
            }

            Ok::<Vec<Triple>, TursoError>(triples)
        }) {
            Ok(triples) => triples,
            Err(e) => {
                error!("failed to run triple query in turso local backend: {}", e);
                Vec::new()
            }
        }
    }
}

#[cfg(feature = "turso-rust")]
pub(crate) struct TursoRemoteBackend {
    runtime: Runtime,
    _db: LibsqlDatabase,
    conn: LibsqlConnection,
}

#[cfg(feature = "turso-rust")]
impl TursoRemoteBackend {
    fn try_new(turso_config: &TursoConfig) -> Result<Self, String> {
        let runtime = RuntimeBuilder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("failed to build tokio runtime: {}", e))?;

        let url = remote_url_from_config(turso_config)
            .ok_or_else(|| "missing Turso remote URL: set [query.turso].url or CIRUP_TURSO_URL".to_string())?;
        let auth_token = remote_auth_token_from_config(turso_config);

        let db = runtime
            .block_on(async { LibsqlBuilder::new_remote(url, auth_token).build().await })
            .map_err(|e| format!("failed to connect to Turso remote: {}", e))?;

        let conn = db
            .connect()
            .map_err(|e| format!("failed to open Turso remote connection: {}", e))?;

        Ok(Self { runtime, _db: db, conn })
    }

    fn register_table_with_resources(&self, table: &str, resources: &[Resource]) -> Result<(), LibsqlError> {
        if !valid_table_name(table) {
            error!("invalid table name {}", table);
            return Ok(());
        }

        let sql = format!("DROP TABLE IF EXISTS {table}; CREATE TABLE {table} (key TEXT, val TEXT);");
        self.runtime.block_on(async { self.conn.execute_batch(&sql).await })?;

        if resources.is_empty() {
            return Ok(());
        }

        self.runtime.block_on(async {
            self.conn.execute("BEGIN", ()).await?;

            let mut insert_sql = String::new();

            for chunk in resources.chunks(TURSO_INSERT_CHUNK_SIZE) {
                build_multi_insert_sql(table, chunk, &mut insert_sql);

                if let Err(e) = self.conn.execute_batch(&insert_sql).await {
                    let _ = self.conn.execute("ROLLBACK", ()).await;
                    return Err(e);
                }
            }

            let index_sql = build_key_index_sql(table);
            if let Err(e) = self.conn.execute_batch(&index_sql).await {
                let _ = self.conn.execute("ROLLBACK", ()).await;
                return Err(e);
            }

            self.conn.execute("COMMIT", ()).await?;
            Ok(())
        })
    }
}

#[cfg(feature = "turso-rust")]
impl QueryBackend for TursoRemoteBackend {
    #[cfg(test)]
    fn register_table_from_str(&mut self, table: &str, filename: &str, data: &str) {
        vfile_set(filename, data);
        let resources = load_resources(filename);
        if let Err(e) = self.register_table_with_resources(table, &resources) {
            error!("failed to register table {} in turso remote backend: {}", table, e);
        }
    }

    fn register_table_from_file(&mut self, table: &str, filename: &str) {
        let resources = load_resources(filename);
        if let Err(e) = self.register_table_with_resources(table, &resources) {
            error!("failed to register table {} in turso remote backend: {}", table, e);
        }
    }

    fn query_resource(&self, query: &str) -> Vec<Resource> {
        match self.runtime.block_on(async {
            let statement = self.conn.prepare(query).await?;
            let mut rows = statement.query(()).await?;
            let mut resources: Vec<Resource> = Vec::new();

            while let Some(row) = rows.next().await? {
                let name: String = row.get(0)?;
                let value: String = row.get(1)?;
                resources.push(Resource::new(&name, &value));
            }

            Ok::<Vec<Resource>, LibsqlError>(resources)
        }) {
            Ok(resources) => resources,
            Err(e) => {
                error!("failed to run query in turso remote backend: {}", e);
                Vec::new()
            }
        }
    }

    fn query_triple(&self, query: &str) -> Vec<Triple> {
        match self.runtime.block_on(async {
            let statement = self.conn.prepare(query).await?;
            let mut rows = statement.query(()).await?;
            let mut triples: Vec<Triple> = Vec::new();

            while let Some(row) = rows.next().await? {
                let name: String = row.get(0)?;
                let value: String = row.get(1)?;
                let base: String = row.get(2)?;
                triples.push(Triple::new(&name, &value, &base));
            }

            Ok::<Vec<Triple>, LibsqlError>(triples)
        }) {
            Ok(triples) => triples,
            Err(e) => {
                error!("failed to run triple query in turso remote backend: {}", e);
                Vec::new()
            }
        }
    }
}

pub(crate) fn build_backend(query_config: &QueryConfig) -> Box<dyn QueryBackend> {
    match query_config.backend {
        QueryBackendKind::Rusqlite => Box::new(RusqliteBackend::new()),
        QueryBackendKind::TursoRemote => {
            #[cfg(feature = "turso-rust")]
            {
                match TursoRemoteBackend::try_new(&query_config.turso) {
                    Ok(backend) => return Box::new(backend),
                    Err(e) => {
                        warn!("{}", e);
                        warn!("falling back to rusqlite backend");
                    }
                }
            }

            #[cfg(not(feature = "turso-rust"))]
            {
                warn!("turso-remote backend requested but 'turso-rust' feature is disabled, falling back to rusqlite");
            }

            Box::new(RusqliteBackend::new())
        }
        QueryBackendKind::TursoLocal => {
            #[cfg(feature = "turso-rust")]
            {
                return Box::new(TursoLocalBackend::new());
            }

            #[cfg(not(feature = "turso-rust"))]
            {
                warn!("turso-local backend requested but 'turso-rust' feature is disabled, falling back to rusqlite");
                Box::new(RusqliteBackend::new())
            }
        }
    }
}
