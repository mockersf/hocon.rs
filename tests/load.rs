use test_generator;

use std::fs::File;
use std::io::prelude::*;

use hocon;

test_generator::test_expand_paths! { file_load; "tests/data/*.conf" }

fn file_load(file_name: &str) {
    let doc: Result<hocon::Hocon, _> = hocon::HoconLoader::new()
        .no_system()
        .load_file(file_name)
        .unwrap()
        .hocon();

    let mut file = File::open(file_name).unwrap();
    let mut original_content = String::new();
    file.read_to_string(&mut original_content).unwrap();
    println!("original file: {}\n{}", file_name, original_content);

    assert!(doc.is_ok());
}
