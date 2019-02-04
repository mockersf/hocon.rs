use test_generator;

use hocon;

test_generator::test_expand_paths! { file_load; "tests/data/*.conf" }

fn file_load(file_name: &str) {
    let doc = hocon::Hocon::load_from_file(file_name);

    assert!(doc.is_ok());
}
