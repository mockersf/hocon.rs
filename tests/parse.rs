use std::collections::HashMap;

use hocon::{Error, Hocon, HoconLoader};

#[test]
fn parse_string() {
    let s = r#"{"a":"dndjf"}"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"].as_string().expect("during test"), "dndjf");
}

#[test]
fn parse_int() {
    let s = r#"{"a":5}"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"].as_i64().expect("during test"), 5);
}

#[test]
fn parse_float() {
    let s = r#"{"a":5.7}"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"].as_f64().expect("during test"), 5.7);
}

#[test]
fn parse_bool() {
    let s = r#"{"a":true}"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"].as_bool().expect("during test"), true);
}

#[test]
fn parse_int_array() {
    let s = r#"{"a":[5, 6, 7]}"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"][1].as_i64().expect("during test"), 6);
    assert_eq!(doc["a"][2].as_i64().expect("during test"), 7);
}

#[test]
fn parse_int_array_newline_as_separator() {
    let s = r#"{"a":[5
    6
    ]}"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"][0].as_i64().expect("during test"), 5);
    assert_eq!(doc["a"][1].as_i64().expect("during test"), 6);
}

#[test]
fn parse_object_newline_as_separator() {
    let s = r#"{"a":5
"b":6
}"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"].as_i64().expect("during test"), 5);
    assert_eq!(doc["b"].as_i64().expect("during test"), 6);
}

#[test]
fn parse_trailing_commas() {
    let s = r#"{"a":[5, 6, 7,
],
}"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"][1].as_i64().expect("during test"), 6);
}

#[test]
fn parse_nested() {
    let s = r#"{"a":{"b":[{"c":5},{"c":6}]}}"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"]["b"][1]["c"].as_i64().expect("during test"), 6);
}

#[test]
fn parse_newlines() {
    let s = r#"{"a":
    5
    }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"].as_i64().expect("during test"), 5);
}

#[test]
fn parse_comment() {
    let s = r#"{
    // comment 0
    "a":5, // comment1
    "b": 6 # comment 2
    # comment 3
    "c": [7 // comment 4
    # comment 5

    // comment 6
    8]
}"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"].as_i64().expect("during test"), 5);
}

#[test]
fn parse_keyvalue_separator() {
    let s = r#"{"a":5,"b"=6,"c" {"a":1}}}"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"].as_i64().expect("during test"), 5);
    assert_eq!(doc["b"].as_i64().expect("during test"), 6);
    assert_eq!(doc["c"]["a"].as_i64().expect("during test"), 1);
}

#[test]
fn parse_object_merging() {
    let s = r#"{
            "foo" : { "a" : 42 },
            "foo" : { "b" : 43 }
        }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["foo"]["a"].as_i64().expect("during test"), 42);
    assert_eq!(doc["foo"]["b"].as_i64().expect("during test"), 43);
}

#[test]
fn parse_change_type_to_object() {
    let s = r#"{
            "foo" : [0, 1, 2],
            "foo" : { "b" : 43 }
        }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert!(doc["foo"][0].as_i64().is_none());
    assert_eq!(doc["foo"]["b"].as_i64().expect("during test"), 43);
}

#[test]
fn parse_change_type_to_array() {
    let s = r#"{
            "foo" : { "b" : 43 },
            "foo" : [0, 1, 2],
        }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["foo"][0].as_i64().expect("during test"), 0);
    assert!(doc["foo"]["b"].as_i64().is_none());
}

#[test]
fn parse_reset_array_index() {
    let s = r#"{
            "foo" : [0, 1, 2],
            "foo" : [5, 6, 7]
        }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["foo"][0].as_i64().expect("during test"), 5);
}

#[test]
fn parse_error() {
    let s = r#"{
            "foo" : { "a" : 42 },
            "foo" : {
        }"#;
    let doc = dbg!(HoconLoader::new().load_str(dbg!(s)));

    assert!(doc.is_err());
}

#[test]
fn wrong_index() {
    let s = r#"{ "a" : 42 }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["missing"], Hocon::BadValue(Error::MissingKey));
    assert_eq!(doc[0], Hocon::BadValue(Error::InvalidKey));
}

#[test]
fn wrong_casts() {
    let s = r#"{ "a" : 42 }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert!(doc["missing"].as_i64().is_none());
    assert!(doc["missing"].as_f64().is_none());
    assert!(doc["missing"].as_bool().is_none());
    assert!(doc["missing"].as_string().is_none());
}

#[test]
fn parse_root_braces_omitted() {
    let s = r#""foo" : { "b" : 43 }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["foo"]["b"].as_i64().expect("during test"), 43);
}

#[test]
fn parse_unquoted_string() {
    let s = r#"{"foo" : { b : hello world }}"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(
        doc["foo"]["b"].as_string().expect("during test"),
        "hello world"
    );
}

#[test]
fn parse_path() {
    let s = r#"{foo.b : hello }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["foo"]["b"].as_string().expect("during test"), "hello");
}

#[test]
fn parse_concat() {
    let s = r#"{"foo" : "hello"" world n°"1 }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(
        doc["foo"].as_string().expect("during test"),
        "hello world n°1"
    );
}

#[test]
fn parse_path_substitution() {
    let s = r#"{"who" : "world", "number": 1, "bar": "hello "${who}" n°"${number} }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(
        doc["bar"].as_string().expect("during test"),
        "hello world n°1"
    );
}

#[test]
fn parse_file_ends_with_unquoted_string() {
    let s = r#"#
foo:bar"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["foo"].as_string().expect("during test"), "bar");
}

#[test]
fn parse_comment_in_array_no_comma() {
    let s = r#"a=[1 // zut
        2]"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"][0].as_i64().expect("during test"), 1);
}

#[test]
fn parse_substitute_array() {
    let s = r#"a=[1, 2, 3],b=[${a}]"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["b"][0][1].as_i64().expect("during test"), 2);
}

#[test]
fn parse_empty_objects() {
    let s = r#"a={b{}},b=5"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["b"].as_i64().expect("during test"), 5);
}

#[test]
fn parse_missing_substitution() {
    let s = r#"{a={c=${?b}}}"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(
        doc["a"]["c"],
        Hocon::BadValue(Error::KeyNotFound {
            key: String::from("b")
        })
    );
}

#[test]
fn parse_empty_object() {
    let s = r#"a=[{},{}],b=[]"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"][0], Hocon::Hash(HashMap::new()));
    assert_eq!(doc["b"], Hocon::Array(vec![]));
}

#[test]
fn parse_comment_after_object() {
    let s = r#"{
    a = {
        b = 2
    }
    # zut
}
#zut"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"]["b"].as_i64().expect("during test"), 2);
}

#[test]
fn substitute_before_and_after() {
    let s = r#"{"a" : "before", "before": ${a}, "after": ${b}, "b": "after" }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["before"].as_string().expect("during test"), "before");
    assert_eq!(doc["after"].as_string().expect("during test"), "after");
}

#[test]
fn environment_variable() {
    std::env::set_var("MY_VAR_TO_TEST", "GREAT_VALUE");

    let s = r#"{"var" : ${MY_VAR_TO_TEST} }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["var"].as_string().expect("during test"), "GREAT_VALUE");
}

#[test]
fn environment_variable_disabled() {
    std::env::set_var("MY_VAR_TO_TEST", "GREAT_VALUE");

    let s = r#"{"var" : ${MY_VAR_TO_TEST} }"#;
    let doc: Hocon = dbg!(HoconLoader::new().no_system().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(
        doc["var"],
        Hocon::BadValue(Error::KeyNotFound {
            key: String::from("MY_VAR_TO_TEST")
        })
    );
}

#[test]
fn parse_triple_quote() {
    let s = r#"{"a" : """my "single line" string""" }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(
        doc["a"].as_string().expect("during test"),
        r#"my "single line" string"#
    );
}

#[test]
fn parse_multiline_string() {
    let s = r#"{"a" : """my
multi
line
string""" }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(
        doc["a"].as_string().expect("during test"),
        r#"my
multi
line
string"#
    );
}

#[test]
fn parse_triple_quote_with_extra_quote() {
    let s = r#"{"a" : """foo"""" }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"].as_string().expect("during test"), r#"foo""#);
}

#[test]
fn parse_multiple_triple_quote_strings() {
    let s = r#"{"a" : """foo"""", b: """hohoho""", c: """my
multi
line
string"""""}"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"].as_string().expect("during test"), r#"foo""#);
    assert_eq!(doc["b"].as_string().expect("during test"), r#"hohoho"#);
    assert_eq!(
        doc["c"].as_string().expect("during test"),
        r#"my
multi
line
string"""#
    );
}

#[test]
fn parse_concat_objects() {
    let s = r#"{"a": {"a": 1} {"b": 2}}"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"]["a"].as_i64().expect("during test"), 1);
    assert_eq!(doc["a"]["b"].as_i64().expect("during test"), 2);
}

#[test]
fn parse_concat_objects_with_substitution() {
    let s = r#"{
        "a": {"a": 1}
        "b": ${a} {"b": 2}
    }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"]["a"].as_i64().expect("during test"), 1);
    assert_eq!(doc["b"]["a"].as_i64().expect("during test"), 1);
    assert_eq!(doc["b"]["b"].as_i64().expect("during test"), 2);
}

#[test]
fn parse_concat_objects_with_self_substitution() {
    let s = r#"{
        "a": {"a": 1}
        "a": ${a} {"b": 2}
    }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"]["a"].as_i64().expect("during test"), 1);
    assert_eq!(doc["a"]["b"].as_i64().expect("during test"), 2);
}

#[test]
fn parse_concat_arrays() {
    let s = r#"{a : [ 1, 2 ] [ 3, 4 ]}"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"][0].as_i64().expect("during test"), 1);
    assert_eq!(doc["a"][1].as_i64().expect("during test"), 2);
    assert_eq!(doc["a"][2].as_i64().expect("during test"), 3);
    assert_eq!(doc["a"][3].as_i64().expect("during test"), 4);
}

#[test]
fn parse_concat_arrays_with_substitution() {
    let s = r#"{
        a : [ 1, 2 ]
        b : ${a} [ 3, 4 ]
    }"#;
    let doc: Hocon = dbg!(dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon())
    .expect("during test");

    assert_eq!(doc["a"][0].as_i64().expect("during test"), 1);
    assert_eq!(doc["a"][1].as_i64().expect("during test"), 2);
    assert_eq!(doc["b"][0].as_i64().expect("during test"), 1);
    assert_eq!(doc["b"][1].as_i64().expect("during test"), 2);
    assert_eq!(doc["b"][2].as_i64().expect("during test"), 3);
    assert_eq!(doc["b"][3].as_i64().expect("during test"), 4);
}

#[test]
fn parse_concat_arrays_with_substitution_and_replace() {
    let s = r#"{
        a : [ 1, 2 ]
        b : ${a} [ 3, 4 ]
        b : [ 5, 6 ]
    }"#;
    let doc: Hocon = dbg!(dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon())
    .expect("during test");

    assert_eq!(doc["a"][0].as_i64().expect("during test"), 1);
    assert_eq!(doc["a"][1].as_i64().expect("during test"), 2);
    assert_eq!(doc["b"][0].as_i64().expect("during test"), 5);
    assert_eq!(doc["b"][1].as_i64().expect("during test"), 6);
    assert!(doc["b"][2].as_i64().is_none());
    assert!(doc["b"][3].as_i64().is_none());
}

#[test]
fn parse_replace_array() {
    let s = r#"{
        a : [ 1, 2, 3, 4 ]
        a : [ 5, 6 ]
    }"#;
    let doc: Hocon = dbg!(dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon())
    .expect("during test");

    assert_eq!(doc["a"][0].as_i64().expect("during test"), 5);
    assert_eq!(doc["a"][1].as_i64().expect("during test"), 6);
    assert!(doc["a"][2].as_i64().is_none());
    assert!(doc["a"][3].as_i64().is_none());
}

#[test]
fn parse_concat_arrays_with_self_substitution() {
    let s = r#"{
        a : [ 1, 2 ]
        a : ${a} [ 3, 4 ]
    }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"][0].as_i64().expect("during test"), 1);
    assert_eq!(doc["a"][1].as_i64().expect("during test"), 2);
    assert_eq!(doc["a"][2].as_i64().expect("during test"), 3);
    assert_eq!(doc["a"][3].as_i64().expect("during test"), 4);
}

#[test]
fn parse_concat_arrays_of_object_with_self_substitution() {
    let s = r#"{
        a : [ { a : 1, b : 2 }, { a : 2, b : 4 } ]
        a : ${a} [ {a : 3, b : 6}, {a : 4, b : 8 } ]
    }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"][0]["a"].as_i64().expect("during test"), 1);
    assert_eq!(doc["a"][0]["b"].as_i64().expect("during test"), 2);
    assert_eq!(doc["a"][1]["a"].as_i64().expect("during test"), 2);
    assert_eq!(doc["a"][1]["b"].as_i64().expect("during test"), 4);
    assert_eq!(doc["a"][2]["a"].as_i64().expect("during test"), 3);
    assert_eq!(doc["a"][2]["b"].as_i64().expect("during test"), 6);
    assert_eq!(doc["a"][3]["a"].as_i64().expect("during test"), 4);
    assert_eq!(doc["a"][3]["b"].as_i64().expect("during test"), 8);
}

#[test]
fn parse_concat_arrays_of_object_with_several_self_substitution() {
    let s = r#"{
        a : [ { a : 1, b : 2 }, { a : 2, b : 4 } ]
        a : ${a} [ {a : 3, b : 6}, {a : 4, b : 8 } ]
        a : ${a} [ {a : 5, b : 10}, {a : 6, b : 12 } ]
    }"#;
    let doc: Hocon = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon()
        .expect("during test");

    assert_eq!(doc["a"][0]["a"].as_i64().expect("during test"), 1);
    assert_eq!(doc["a"][0]["b"].as_i64().expect("during test"), 2);
    assert_eq!(doc["a"][1]["a"].as_i64().expect("during test"), 2);
    assert_eq!(doc["a"][1]["b"].as_i64().expect("during test"), 4);
    assert_eq!(doc["a"][2]["a"].as_i64().expect("during test"), 3);
    assert_eq!(doc["a"][2]["b"].as_i64().expect("during test"), 6);
    assert_eq!(doc["a"][3]["a"].as_i64().expect("during test"), 4);
    assert_eq!(doc["a"][3]["b"].as_i64().expect("during test"), 8);
    assert_eq!(doc["a"][4]["a"].as_i64().expect("during test"), 5);
    assert_eq!(doc["a"][4]["b"].as_i64().expect("during test"), 10);
    assert_eq!(doc["a"][5]["a"].as_i64().expect("during test"), 6);
    assert_eq!(doc["a"][5]["b"].as_i64().expect("during test"), 12);
}

#[test]
fn parse_concat_arrays_with_plus_equal() {
    let s = r#"{
        a += 1
        a += 2
    }"#;
    let doc: Hocon = dbg!(dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon())
    .expect("during test");

    assert_eq!(doc["a"][0].as_i64().expect("during test"), 1);
    assert_eq!(doc["a"][1].as_i64().expect("during test"), 2);
}

#[test]
fn parse_concat_arrays_with_plus_equal_with_init() {
    let s = r#"{
        a = [ 1 ]
        a += 2
        "a" += 3
    }"#;
    let doc: Hocon = dbg!(dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon())
    .expect("during test");

    assert_eq!(doc["a"][0].as_i64().expect("during test"), 1);
    assert_eq!(doc["a"][1].as_i64().expect("during test"), 2);
    assert_eq!(doc["a"][2].as_i64().expect("during test"), 3);
}

#[test]
fn parse_concat_arrays_with_plus_equal_with_object() {
    let s = r#"{
        a += { b : 1 }
        a += { b : 2 }
        a += { b : 3, c : 4 }
        a += { d : 5, f : { g : 6 } }
    }"#;
    let doc: Hocon = dbg!(HoconLoader::new()
        .load_str(s)
        .expect("during test")
        .hocon()
        .expect("during test"));

    assert_eq!(doc["a"][0]["b"].as_i64().expect("during test"), 1);
    assert_eq!(doc["a"][1]["b"].as_i64().expect("during test"), 2);
    assert_eq!(doc["a"][2]["b"].as_i64().expect("during test"), 3);
    assert_eq!(doc["a"][2]["c"].as_i64().expect("during test"), 4);
    assert_eq!(doc["a"][3]["d"].as_i64().expect("during test"), 5);
    assert_eq!(doc["a"][3]["f"]["g"].as_i64().expect("during test"), 6);
}

#[test]
fn parse_null_value() {
    let s = r#"{
        a = null
    }"#;
    let doc: Hocon = dbg!(dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test")
        .hocon())
    .expect("during test");

    assert_eq!(doc["a"], Hocon::Null);
}

#[test]
fn parse_include_from_str() {
    let s = r#"{"a":5, include "data/basic.conf" }"#;
    let loader = dbg!(HoconLoader::new().strict().load_str(dbg!(s)));

    assert!(loader.is_err());
    assert_eq!(loader.err(), Some(hocon::Error::IncludeNotAllowedFromStr))
}
