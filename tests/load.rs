use insta::assert_debug_snapshot_matches;
use test_generator;

use hocon::{self, Hocon};

test_generator::test_expand_paths! { file_load; "tests/data/*.conf" }

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
            slice_vals.sort_by(|a, b| a.0.partial_cmp(b.0).unwrap());
            format!(
                "{{{}}}",
                slice_vals
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, stable_readable_display(v)))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }

        Hocon::BadValue => String::from("BadValue"),
    }
}

fn file_load(file_name: &str) {
    let doc = hocon::Hocon::load_from_file(file_name);

    assert!(doc.is_ok());
    assert_debug_snapshot_matches!(
        file_name.split('/').last().unwrap(),
        stable_readable_display(&doc.unwrap())
    );
}
