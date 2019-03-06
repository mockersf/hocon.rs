use test_generator;

use std::fs::File;
use std::io::prelude::*;

use hocon;

test_generator::test_expand_paths! { file_load; "tests/data/*.conf" }

fn file_load(file_name: &str) {
    let doc = hocon::HoconLoader::new()
        .no_system()
        .load_file(file_name)
        .map(|doc| doc.hocon());

    let mut file = File::open(file_name).expect("during test");
    let mut original_content = String::new();
    file.read_to_string(&mut original_content)
        .expect("during test");
    println!("original file: {}\n{}", file_name, original_content);

    assert!(dbg!(doc).is_ok());
}
