
use prettytable::Table;
use prettytable::row::Row;
use prettytable::cell::Cell;

use rusqlite::types::*;
use rusqlite::{Connection, Result, Error};
use rusqlite::{Rows, Row as RusqliteRow};

use vtab::{create_db, init_db, register_table};
use file::{vfile_id, vfile_get, vfile_set};

pub fn print_pretty(columns: Vec<String>, values: &mut Rows) {
    let mut row = Row::empty();
    let mut table: Table = Table::new();
    //write header first
    table.set_titles(columns.iter().collect());
    loop {
        if let Some(v) = values.next(){
            if let Some (res) = v.ok() {
                for i in 0..res.column_count() {
                    let val = Value::data_type(&res.get(i));
                    match val {
                        Type::Real | Type::Integer => {
                            row.add_cell(Cell::new(&res.get::<usize,i64>(i).to_string()));
                        },
                        Type::Text => {
                            row.add_cell(Cell::new(&res.get::<usize,String>(i)))
                        },
                        _ => {
                            // Do nothing.
                        }
                    }
                }
                table.add_row(row);
                row = Row::empty();
            }
        } else {
            break
        }
    }
    println!("{}", table);
}

pub fn execute_query(db: &Connection, query: &str) {
    let mut table_result: Vec<Vec<Value>> = Vec::new();
    let mut row: Vec<Value> = Vec::new();
    let stmt = db.prepare(&query);

    match stmt {
        Ok(mut statement_res) => {
            let mut col_name_internal = Vec::new();
            for col_name in statement_res.column_names().iter() {
                col_name_internal.push(col_name.to_string());
                let v: Value = Value::Text(col_name.to_string());
                row.push(v);
            }
            table_result.push(row);

            let mut response = statement_res.query(&[]).unwrap();
            print_pretty(col_name_internal, &mut response);
        },
        Err(e) => {
            match e {
                Error::SqliteFailure(_r, m) => {
                    if let Some(msg) = m { println!("{}", msg) };
                },
                _ => println!("{:?}", Error::ModuleError(format!("{}", e)))
            }
        }
    }
}

pub fn query_file(input: &str, table: &str, query: &str) {
    let db = init_db(table, input);
    execute_query(&db, query);
}

pub struct CirupEngine {
    pub db: Connection,
}

impl CirupEngine {
    pub fn new() -> Self {
        CirupEngine {
            db: create_db(),
        }
    }

    pub fn register_table_from_str(&self, table: &str, filename: &str, data: &str) {
        vfile_set(filename, data);
        register_table(&self.db, table, filename);
    }

    pub fn register_table_from_file(&self, table: &str, filename: &str) {
        register_table(&self.db, table, filename);
    }

    pub fn query(&self, query: &str) {
        execute_query(&self.db, query);
    }
}

#[test]
fn test_query() {
    let engine = CirupEngine::new();
    engine.register_table_from_str("test_json", "test.json", include_str!("../test/test.json"));
    engine.register_table_from_str("test_resx", "test.resx", include_str!("../test/test.resx"));

    // find the union of the two tables (merge strings)
    engine.query("SELECT * FROM test_json UNION SELECT * from test_resx");

    // find the intersection of the two tables (common strings)
    engine.query("SELECT * FROM test_json INTERSECT SELECT * from test_resx");
}
