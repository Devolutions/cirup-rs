use prettytable::{Cell, Row, Table};

use rusqlite::Rows;
use rusqlite::types::*;
use rusqlite::{Connection, Error, Statement};

use crate::file::{save_resource_file, vfile_set};
use crate::vtab::{create_db, init_db, register_table};

use crate::{Resource, Triple};

pub fn print_pretty(columns: Vec<String>, values: &mut Rows<'_>) {
    let mut row = Row::empty();
    let mut table: Table = Table::new();
    //write header first
    table.set_titles(columns.iter().collect());
    loop {
        if let Some(v) = values.next() {
            if let Some(res) = v.ok() {
                for i in 0..res.column_count() {
                    let val = Value::data_type(&res.get(i));
                    match val {
                        Type::Real | Type::Integer => {
                            row.add_cell(Cell::new(&res.get::<usize, i64>(i).to_string()));
                        }
                        Type::Text => row.add_cell(Cell::new(&res.get::<usize, String>(i))),
                        _ => {
                            // Do nothing.
                        }
                    }
                }
                table.add_row(row);
                row = Row::empty();
            }
        } else {
            break;
        }
    }
    println!("{}", table);
}

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

pub fn print_triples_pretty(triples: &[Triple]) {
    for triple in triples {
        println!("name: {}", triple.name);
        println!("base: {}", triple.base);
        println!("value: {}", triple.value);
        println!("");
    }
}

fn get_statement_column_names(statement: &Statement<'_>) -> Vec<String> {
    let mut column_names = Vec::new();
    for column_name in statement.column_names().iter() {
        column_names.push(column_name.to_string());
    }
    column_names
}

pub fn execute_query(db: &Connection, query: &str) {
    let stmt = db.prepare(&query);

    let mut table_result: Vec<Vec<Value>> = Vec::new();
    let mut row: Vec<Value> = Vec::new();

    match stmt {
        Ok(mut statement) => {
            let column_names = get_statement_column_names(&statement);

            for column_name in statement.column_names().iter() {
                let v: Value = Value::Text(column_name.to_string());
                row.push(v);
            }
            table_result.push(row);
            if let Ok(mut response) = statement.query(&[]) {
                print_pretty(column_names, &mut response);
            }
        }
        Err(e) => match e {
            Error::SqliteFailure(_r, m) => {
                if let Some(msg) = m {
                    error!("{}", msg)
                };
            }
            _ => error!("{:?}", Error::ModuleError(format!("{}", e))),
        },
    }
}

pub fn execute_query_resource(db: &Connection, query: &str) -> Vec<Resource> {
    let mut resources: Vec<Resource> = Vec::new();
    let mut statement = match db.prepare(&query) {
        Ok(statement) => statement,
        Err(_) => return resources,
    };
    let mut response = match statement.query(&[]) {
        Ok(response) => response,
        Err(_) => return resources,
    };

    loop {
        if let Some(v) = response.next() {
            if let Some(res) = v.ok() {
                let name = &res.get::<usize, String>(0);
                let value = &res.get::<usize, String>(1);
                let resource = Resource::new(name, value);
                resources.push(resource);
            }
        } else {
            break;
        }
    }

    resources
}

pub fn execute_query_triple(db: &Connection, query: &str) -> Vec<Triple> {
    let mut resources: Vec<Triple> = Vec::new();
    let mut statement = match db.prepare(&query) {
        Ok(statement) => statement,
        Err(_) => return resources,
    };
    let mut response = match statement.query(&[]) {
        Ok(response) => response,
        Err(_) => return resources,
    };

    loop {
        if let Some(v) = response.next() {
            if let Some(res) = v.ok() {
                let name = &res.get::<usize, String>(0);
                let value = &res.get::<usize, String>(1);
                let base = &res.get::<usize, String>(2);
                let resource = Triple::new(name, value, base);
                resources.push(resource);
            }
        } else {
            break;
        }
    }

    resources
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
        CirupEngine { db: create_db() }
    }

    #[allow(dead_code)]
    fn register_table_from_str(&self, table: &str, filename: &str, data: &str) {
        vfile_set(filename, data);
        register_table(&self.db, table, filename);
    }

    pub fn register_table_from_file(&self, table: &str, filename: &str) {
        register_table(&self.db, table, filename);
    }

    pub fn query_resource(&self, query: &str) -> Vec<Resource> {
        execute_query_resource(&self.db, query)
    }

    pub fn query_triple(&self, query: &str) -> Vec<Triple> {
        execute_query_triple(&self.db, query)
    }

    pub fn query(&self, query: &str) {
        execute_query(&self.db, query);
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
    CirupQuery::new(PRINT_QUERY, file, None, None)
}

pub fn query_convert(file: &str) -> CirupQuery {
    CirupQuery::new(CONVERT_QUERY, file, None, None)
}

pub fn query_sort(file: &str) -> CirupQuery {
    CirupQuery::new(SORT_QUERY, file, None, None)
}

pub fn query_diff(file_one: &str, file_two: &str) -> CirupQuery {
    CirupQuery::new(DIFF_QUERY, file_one, Some(file_two), None)
}

pub fn query_diff_with_base(old: &str, new: &str, base: &str) -> CirupQuery {
    CirupQuery::new(DIFF_WITH_BASE_QUERY, old, Some(new), Some(base))
}

pub fn query_change(file_one: &str, file_two: &str) -> CirupQuery {
    CirupQuery::new(CHANGE_QUERY, file_one, Some(file_two), None)
}

pub fn query_merge(file_one: &str, file_two: &str) -> CirupQuery {
    CirupQuery::new(MERGE_QUERY, file_one, Some(file_two), None)
}

pub fn query_intersect(file_one: &str, file_two: &str) -> CirupQuery {
    CirupQuery::new(INTERSECT_QUERY, file_one, Some(file_two), None)
}

pub fn query_subtract(file_one: &str, file_two: &str) -> CirupQuery {
    CirupQuery::new(SUBTRACT_QUERY, file_one, Some(file_two), None)
}

impl CirupQuery {
    pub fn new(query: &str, file_one: &str, file_two: Option<&str>, file_three: Option<&str>) -> Self {
        let engine = CirupEngine::new();
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
fn test_query() {
    let engine = CirupEngine::new();
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
    let engine = CirupEngine::new();

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
fn test_query_diff_with_base() {
    let engine = CirupEngine::new();
    engine.register_table_from_str("A", "test_old.resx", include_str!("../test/test_old.resx"));
    engine.register_table_from_str("B", "test_new.resx", include_str!("../test/test_new.resx"));
    engine.register_table_from_str("C", "test.resx", include_str!("../test/test.resx"));

    let triples = engine.query_triple(DIFF_WITH_BASE_QUERY);

    assert_eq!(triples.len(), 2);
    assert_eq!(triples[0].name, String::from("lblYolo"));
    assert_eq!(triples[0].base, String::from("You only live once"));
    assert_eq!(triples[0].value, String::from("Juste une vie a vivre"));
}
