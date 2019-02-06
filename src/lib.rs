#![deny(
    warnings,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications,
    missing_docs
)]

//! HOCON
//!
//! Parse HOCON configuration files in Rust
//!
//! ```rust
//! use hocon::Hocon;
//!
//! let s = r#"{"a":5}"#;
//! let doc = Hocon::load_from_str(s).unwrap();
//! let a = doc["a"].as_i64();
//! ```
//!  

use std::collections::HashMap;
use std::ops::Index;

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

mod internals;
mod parser;

/// HOCON document
#[derive(Debug, Clone)]
pub enum Hocon {
    /// A floating value
    Real(f64),
    /// An integer value
    Integer(i64),
    /// A string
    String(String),
    /// A boolean
    Boolean(bool),
    /// An array of `Hocon` values
    Array(Vec<Hocon>),
    /// An HashMap of `Hocon` values with keys
    Hash(HashMap<String, Hocon>),
    /// A `BadValue`, marking an error in parsing or an missing value
    BadValue,
}

impl Hocon {
    pub(crate) fn parse_str_to_internal(
        file_root: Option<&str>,
        s: &str,
    ) -> Result<internals::HoconInternal, ()> {
        Ok(parser::root(format!("{}\0", s).as_bytes(), file_root)
            .map_err(|_| ())?
            .1)
    }

    pub(crate) fn load_from_str_with_file_root(
        file_root: Option<&str>,
        s: &str,
    ) -> Result<Hocon, ()> {
        Self::parse_str_to_internal(file_root, s)
            .and_then(|hocon| hocon.merge())
            .map(|intermediate| intermediate.finalize())
    }

    pub(crate) fn load_file(file_root: &str, path: &str) -> Result<(String, String), ()> {
        let full_path = Path::new(file_root).join(path);
        let mut file = File::open(full_path.as_os_str()).map_err(|_| ())?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|_| ())?;
        Ok((
            String::from(full_path.parent().and_then(|p| p.to_str()).unwrap_or("")),
            contents,
        ))
    }

    /// Load a string containing an `Hocon` document. Includes are not supported when
    /// loading from a string
    pub fn load_from_str(s: &str) -> Result<Hocon, ()> {
        Self::load_from_str_with_file_root(None, s)
    }

    /// Load the HOCON configuration file containing an `Hocon` document
    pub fn load_from_file(path: &str) -> Result<Hocon, ()> {
        let (root, contents) = Self::load_file("", path)?;
        Self::load_from_str_with_file_root(Some(&root), &contents)
    }
}

static BAD_VALUE: Hocon = Hocon::BadValue;

impl<'a> Index<&'a str> for Hocon {
    type Output = Hocon;

    fn index(&self, idx: &'a str) -> &Self::Output {
        match self {
            Hocon::Hash(hash) => hash.get(idx).unwrap_or(&BAD_VALUE),
            _ => &BAD_VALUE,
        }
    }
}
impl Index<usize> for Hocon {
    type Output = Hocon;

    fn index(&self, idx: usize) -> &Self::Output {
        match self {
            Hocon::Array(vec) => vec.get(idx).unwrap_or(&BAD_VALUE),
            _ => &BAD_VALUE,
        }
    }
}

impl Hocon {
    /// Try to cast a value as a `f64` value
    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Hocon::Real(ref v) => Some(*v),
            _ => None,
        }
    }

    /// Try to cast a value as a `i64` value
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Hocon::Integer(ref v) => Some(*v),
            _ => None,
        }
    }

    /// Try to cast a value as a `String` value
    pub fn as_string(&self) -> Option<String> {
        match *self {
            Hocon::String(ref v) => Some(v.to_string()),
            Hocon::Boolean(true) => Some("true".to_string()),
            Hocon::Boolean(false) => Some("false".to_string()),
            Hocon::Integer(i) => Some(i.to_string()),
            Hocon::Real(f) => Some(f.to_string()),
            _ => None,
        }
    }

    /// Try to cast a value as a `bool` value
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Hocon::Boolean(ref v) => Some(*v),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_string() {
        let s = r#"{"a":"dndjf"}"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"].as_string().unwrap(), "dndjf");
    }

    #[test]
    fn parse_int() {
        let s = r#"{"a":5}"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"].as_i64().unwrap(), 5);
    }

    #[test]
    fn parse_float() {
        let s = r#"{"a":5.7}"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"].as_f64().unwrap(), 5.7);
    }

    #[test]
    fn parse_bool() {
        let s = r#"{"a":true}"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"].as_bool().unwrap(), true);
    }

    #[test]
    fn parse_int_array() {
        let s = r#"{"a":[5, 6, 7]}"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"][1].as_i64().unwrap(), 6);
        assert_eq!(doc["a"][2].as_i64().unwrap(), 7);
    }

    #[test]
    fn parse_int_array_newline_as_separator() {
        let s = r#"{"a":[5
    6
    ]}"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"][0].as_i64().unwrap(), 5);
        assert_eq!(doc["a"][1].as_i64().unwrap(), 6);
    }

    #[test]
    fn parse_object_newline_as_separator() {
        let s = r#"{"a":5
"b":6
}"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"].as_i64().unwrap(), 5);
        assert_eq!(doc["b"].as_i64().unwrap(), 6);
    }

    #[test]
    fn parse_trailing_commas() {
        let s = r#"{"a":[5, 6, 7,
],
}"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"][1].as_i64().unwrap(), 6);
    }

    #[test]
    fn parse_nested() {
        let s = r#"{"a":{"b":[{"c":5},{"c":6}]}}"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"]["b"][1]["c"].as_i64().unwrap(), 6);
    }

    #[test]
    fn parse_newlines() {
        let s = r#"{"a":
    5
    }"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"].as_i64().unwrap(), 5);
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
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"].as_i64().unwrap(), 5);
    }

    #[test]
    fn parse_keyvalue_separator() {
        let s = r#"{"a":5,"b"=6,"c" {"a":1}}}"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"].as_i64().unwrap(), 5);
        assert_eq!(doc["b"].as_i64().unwrap(), 6);
        assert_eq!(doc["c"]["a"].as_i64().unwrap(), 1);
    }

    #[test]
    fn parse_object_merging() {
        let s = r#"{
            "foo" : { "a" : 42 },
            "foo" : { "b" : 43 }
        }"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["foo"]["a"].as_i64().unwrap(), 42);
        assert_eq!(doc["foo"]["b"].as_i64().unwrap(), 43);
    }

    #[test]
    fn parse_change_type_to_object() {
        let s = r#"{
            "foo" : [0, 1, 2],
            "foo" : { "b" : 43 }
        }"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert!(doc["foo"][0].as_i64().is_none());
        assert_eq!(doc["foo"]["b"].as_i64().unwrap(), 43);
    }

    #[test]
    fn parse_change_type_to_array() {
        let s = r#"{
            "foo" : { "b" : 43 },
            "foo" : [0, 1, 2],
        }"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["foo"][0].as_i64().unwrap(), 0);
        assert!(doc["foo"]["b"].as_i64().is_none());
    }

    #[test]
    fn parse_reset_array_index() {
        let s = r#"{
            "foo" : [0, 1, 2],
            "foo" : [5, 6, 7]
        }"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["foo"][0].as_i64().unwrap(), 5);
    }

    #[test]
    fn parse_error() {
        let s = r#"{
            "foo" : { "a" : 42 },
            "foo" : {
        }"#;
        let doc = Hocon::load_from_str(s);

        assert!(doc.is_err());
    }

    #[test]
    fn wrong_index() {
        let s = r#"{ "a" : 42 }"#;
        let doc = Hocon::load_from_str(s).unwrap();

        if let Hocon::BadValue = doc["missing"] {

        } else {
            assert!(false);
        }
        if let Hocon::BadValue = doc[0] {

        } else {
            assert!(false);
        }
    }

    #[test]
    fn wrong_casts() {
        let s = r#"{ "a" : 42 }"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert!(doc["missing"].as_i64().is_none());
        assert!(doc["missing"].as_f64().is_none());
        assert!(doc["missing"].as_bool().is_none());
        assert!(doc["missing"].as_string().is_none());
    }

    #[test]
    fn parse_root_braces_omitted() {
        let s = r#""foo" : { "b" : 43 }"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["foo"]["b"].as_i64().unwrap(), 43);
    }

    #[test]
    fn parse_unquoted_string() {
        let s = r#"{"foo" : { b : hello }}"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["foo"]["b"].as_string().unwrap(), "hello");
    }

    #[test]
    fn parse_path() {
        let s = r#"{foo.b : hello }"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["foo"]["b"].as_string().unwrap(), "hello");
    }

    #[test]
    fn parse_concat() {
        let s = r#"{"foo" : "hello"" world n°"1 }"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["foo"].as_string().unwrap(), "hello world n°1");
    }

    #[test]
    fn parse_path_substitution() {
        let s = r#"{"who" : "world", "number": 1, "bar": "hello "${who}" n°"${number} }"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["bar"].as_string().unwrap(), "hello world n°1");
    }

}
