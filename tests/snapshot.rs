#![cfg(feature = "test-snapshot")]

use std::fs::File;
use std::io::prelude::*;

use insta::assert_debug_snapshot_matches;
use test_generator;

use hocon::{self, Hocon};

test_generator::test_expand_paths! { snapshot; "tests/data/*.conf" }

fn stable_readable_display(value: &Hocon) -> String {
    match value {
        Hocon::Real(v) => format!("{}", v),
        Hocon::Integer(v) => format!("{}", v),
        Hocon::String(v) => format!("\"{}\"", v),
        Hocon::Boolean(v) => format!("{}", v),
        Hocon::Array(v) => format!(
            "[{}]",
            v.iter()
                .map(|i| stable_readable_display(i))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Hocon::Hash(v) => {
            let values = v.iter().collect::<Vec<(&String, &Hocon)>>();
            let mut slice_vals = values.into_boxed_slice();
            slice_vals.sort_by(|a, b| a.0.partial_cmp(b.0).expect("during test"));
            format!(
                "{{{}}}",
                slice_vals
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, stable_readable_display(v)))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        Hocon::Null => String::from("null"),
        Hocon::BadValue(_) => String::from("BadValue"),
    }
}

fn snapshot(file_name: &str) {
    let doc = hocon::HoconLoader::new()
        .no_system()
        .load_file(file_name)
        .expect("during test")
        .hocon()
        .expect("during test");

    let mut file = File::open(file_name).expect("during test");
    let mut original_content = String::new();
    file.read_to_string(&mut original_content)
        .expect("during test");
    println!("original file: {}\n{}", file_name, original_content);

    assert_debug_snapshot_matches!(
        file_name.split('/').last().expect("during test"),
        stable_readable_display(&doc)
    );
}
