#[cfg(feature = "turso-rust")]
use crate::config::TursoConfig;
use crate::config::{QueryBackendKind, QueryConfig};
use crate::file::load_resource_file;
#[cfg(test)]
use crate::file::vfile_set;
use crate::{Resource, Triple};

use rusqlite::{Connection, Error as SqlError, Statement};

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

        Self { runtime, _db: db, conn }
    }

    fn register_table_with_resources(&self, table: &str, resources: &[Resource]) -> Result<(), TursoError> {
        if !valid_table_name(table) {
            error!("invalid table name {}", table);
            return Ok(());
        }

        let sql = format!("DROP TABLE IF EXISTS {table}; CREATE TABLE {table} (key TEXT, val TEXT);");
        self.runtime.block_on(async { self.conn.execute_batch(&sql).await })?;

        let insert_sql = format!("INSERT INTO {table} (key, val) VALUES (?1, ?2)");

        self.runtime.block_on(async {
            let mut statement = self.conn.prepare(&insert_sql).await?;
            for resource in resources {
                statement
                    .execute([resource.name.as_str(), resource.value.as_str()])
                    .await?;
            }
            Ok(())
        })
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

        let insert_sql = format!("INSERT INTO {table} (key, val) VALUES (?1, ?2)");
        self.runtime.block_on(async {
            let statement = self.conn.prepare(&insert_sql).await?;
            for resource in resources {
                statement
                    .execute([resource.name.as_str(), resource.value.as_str()])
                    .await?;
            }
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
