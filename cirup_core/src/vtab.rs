use rusqlite::vtab::{
    dequote, read_only_module, sqlite3_vtab, sqlite3_vtab_cursor, Context, CreateVTab, IndexInfo,
    Module, VTab, VTabConnection, VTabCursor, Values,
};

use rusqlite::types::*;
use rusqlite::{Connection, Error, Result};

use std::os::raw::c_int;
use std::str;

use file::load_resource_file;

fn query_table(filename: &str) -> Vec<Vec<Value>> {
    let mut rows: Vec<Vec<Value>> = Vec::new();
    match load_resource_file(filename) {
        Ok(val) => {
            for resource in val.iter() {
                let mut row: Vec<Value> = Vec::new();
                row.push(Value::from(resource.name.clone()));
                row.push(Value::from(resource.value.clone()));
                rows.push(row);
            }
        }
        Err(_e) => {} // TODO: we couldn't parse the file
    };

    rows
}

fn create_schema(
    column_name: &Vec<&'static str>,
    column_types: &Vec<&'static str>,
) -> Option<String> {
    let mut schema = None;
    if schema.is_none() {
        let mut sql = String::from("CREATE TABLE x(");
        for (i, col) in column_name.iter().enumerate() {
            sql.push('"');
            sql.push_str(col);
            sql.push_str(column_types[i]);
            if i == column_name.len() - 1 {
                sql.push_str(");");
            } else {
                sql.push_str(", ");
            }
        }
        schema = Some(sql);
    }
    schema
}

fn get_schema() -> Option<String> {
    let names = vec!["key", "val"];
    let types = vec!["\" TEXT", "\" TEXT"];
    create_schema(&names, &types)
}

pub fn register_table(db: &Connection, table: &str, filename: &str) {
    let mut sql = String::from("CREATE VIRTUAL TABLE ");
    sql.push_str(table);
    sql.push_str(" USING cirup(filename=\"");
    sql.push_str(filename);
    sql.push_str("\")");
    &db.execute_batch(&sql).unwrap();
}

pub fn create_db() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    load_module(&db).unwrap();
    db
}

pub fn init_db(table: &str, filename: &str) -> Connection {
    let db = Connection::open_in_memory().unwrap();
    load_module(&db).unwrap();
    register_table(&db, table, filename);
    db
}

pub fn load_module(conn: &Connection) -> Result<()> {
    let aux: Option<()> = None;
    conn.create_module("cirup", &CIRUP_MODULE, aux)
}

lazy_static! {
    static ref CIRUP_MODULE: Module<CirupTab> = read_only_module::<CirupTab>(1);
}

#[repr(C)]
struct CirupTab {
    /// Base class. Must be first
    base: sqlite3_vtab,
    filename: String,
}

impl CirupTab {
    fn parameter(c_slice: &[u8]) -> Result<(&str, &str)> {
        let arg = str::from_utf8(c_slice)?.trim();
        let mut split = arg.split('=');
        if let Some(key) = split.next() {
            if let Some(value) = split.next() {
                let param = key.trim();
                let value = dequote(value);
                return Ok((param, value));
            }
        }
        Err(Error::ModuleError(format!("illegal argument: '{}'", arg)))
    }
}

impl VTab for CirupTab {
    type Aux = ();
    type Cursor = CirupTabCursor;

    fn connect(
        _: &mut VTabConnection,
        _aux: Option<&()>,
        _args: &[&[u8]],
    ) -> Result<(String, CirupTab)> {
        if _args.len() < 4 {
            return Err(Error::ModuleError("no table name specified".to_owned()));
        }

        let mut vtab = CirupTab {
            base: sqlite3_vtab::default(),
            filename: String::new(),
        };
        let schema;
        let args = &_args[3..];

        for c_slice in args {
            let (param, value) = CirupTab::parameter(c_slice)?;
            match param {
                "filename" => {
                    vtab.filename = value.to_string();
                }
                _ => {
                    return Err(Error::ModuleError(format!(
                        "unrecognized parameter '{}'",
                        param
                    )));
                }
            }
        }

        schema = get_schema();
        Ok((schema.unwrap().to_owned(), vtab))
    }

    fn best_index(&self, info: &mut IndexInfo) -> Result<()> {
        info.set_estimated_cost(1_000_000.);
        Ok(())
    }

    fn open(&self) -> Result<CirupTabCursor> {
        Ok(CirupTabCursor::default())
    }
}

impl CreateVTab for CirupTab {}

#[derive(Default)]
#[repr(C)]
struct CirupTabCursor {
    /// Base class. Must be first
    base: sqlite3_vtab_cursor,
    /// table is in memory
    table_in_memory: bool,
    /// The rowid
    row_id: i64,
    /// columns name
    cols: Vec<Value>,
    /// rows
    rows: Vec<Vec<Value>>,
    /// the end of the table
    eot: bool,
}

impl VTabCursor for CirupTabCursor {
    fn filter(&mut self, _idx_num: c_int, _idx_str: Option<&str>, _args: &Values) -> Result<()> {
        let cirup_table = unsafe { &*(self.base.pVtab as *const CirupTab) };
        // register table in memory
        if !self.table_in_memory {
            self.rows = query_table(cirup_table.filename.as_str());
            self.table_in_memory = true;
        }
        self.row_id = 0;
        self.next()
    }

    fn next(&mut self) -> Result<()> {
        if self.row_id == self.rows.len() as i64 {
            self.eot = true;
        } else {
            self.cols = self.rows[self.row_id as usize].clone();
            self.row_id += 1;
            self.eot = false;
        }

        Ok(())
    }

    fn eof(&self) -> bool {
        self.eot
    }

    fn column(&self, ctx: &mut Context, col: c_int) -> Result<()> {
        if col < 0 || col as usize >= self.cols.len() {
            return Err(Error::ModuleError(format!(
                "column index out of bounds: {}",
                col
            )));
        }
        if self.cols.is_empty() {
            return ctx.set_result(&Null);
        }
        ctx.set_result(&self.cols[col as usize].to_owned())
    }

    fn rowid(&self) -> Result<i64> {
        Ok(self.row_id)
    }
}
