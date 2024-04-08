use std::env;

use cirup_core::query::CirupEngine;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        println!("diff_with_base <old.resx> <new.resx> <base.resx>");
        return;
    }

    let engine = CirupEngine::new();
    engine.register_table_from_file("old", &args[1]);
    engine.register_table_from_file("new", &args[2]);
    engine.register_table_from_file("base", &args[3]);

    let triples = engine.query_triple(r"SELECT new.key, new.val, base.val 
    FROM new 
    LEFT OUTER JOIN old ON new.key = old.key 
    INNER JOIN base ON new.key = base.key 
    WHERE (old.val IS NULL)");

    for triple in triples.iter() {
        println!("key: {}", triple.name);
        println!("orig: {}", triple.base);
        println!("trad: {}", triple.value);
        println!("");
    };
}