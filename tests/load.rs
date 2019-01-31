use test_generator;

use hocon;

test_generator::test_expand_paths! { file_load; "tests/data/*" }

fn file_load(file_name: &str) {
    assert!(hocon::Hocon::load_from_file(file_name).is_ok());
}
