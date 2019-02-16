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
//! use hocon::HoconLoader;
//!
//! let s = r#"{"a":5}"#;
//! let doc = HoconLoader::load_from_str(s).unwrap();
//! let a = doc["a"].as_i64();
//! ```
//!
//! Support serde to deserialize to a `struct`
//!
//! ```rust
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct Configuration {
//!     host: String,
//!     port: u8,
//!     auto_connect: bool,
//! }
//!
//! let s = r#"{host: 127.0.0.1, port: 80, auto_connect: false}"#;
//!
//! let conf: Configuration = hocon::serde::from_str(s).unwrap();
//!  ````
//!

use std::collections::HashMap;
use std::ops::Index;

use std::ffi::OsStr;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

mod internals;
mod parser;

// #[cfg(feature = "serde-support")]
pub mod serde;

/// HOCON document
#[derive(Debug, Clone, PartialEq)]
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
    /// A null value
    Null,
    /// A `BadValue`, marking an error in parsing or an missing value
    BadValue,
}

#[derive(Debug, Clone)]
pub(crate) enum FileType {
    Properties,
    Hocon,
}

#[derive(Debug, Clone)]
pub(crate) struct ConfFileMeta {
    path: PathBuf,
    file_type: FileType,
}
impl ConfFileMeta {
    fn from_path(path: PathBuf) -> Self {
        let file = path.file_name().unwrap();
        let mut parent_path = path.clone();
        parent_path.pop();

        Self {
            path: parent_path,
            file_type: match Path::new(file).extension().and_then(OsStr::to_str) {
                Some("properties") => FileType::Properties,
                _ => FileType::Hocon,
            },
        }
    }
}

/// Helper to load an HOCON file
#[derive(Debug, Clone)]
pub struct HoconLoader {
    include_depth: usize,
    file_meta: Option<ConfFileMeta>,
}

impl HoconLoader {
    pub(crate) fn parse_str_to_internal(&self, s: &str) -> Result<internals::HoconInternal, ()> {
        Ok(parser::root(format!("{}\n\0", s).as_bytes(), self)
            .map_err(|_| ())?
            .1)
    }

    pub(crate) fn load_from_str_of_conf_file(&self, s: &str) -> Result<Hocon, ()> {
        match self.file_meta {
            Some(ConfFileMeta {
                file_type: FileType::Properties,
                ..
            }) => java_properties::read(s.as_bytes())
                .map(internals::HoconInternal::from_properties)
                .map_err(|_| ()),
            _ => self.parse_str_to_internal(s),
        }
        .and_then(|hocon| hocon.merge())
        .map(|intermediate| intermediate.finalize())
    }

    pub(crate) fn load_file(
        file_root: &str,
        path: &str,
        depth: usize,
    ) -> Result<(Self, String), ()> {
        let full_path = Path::new(file_root).join(path);
        let mut file = File::open(full_path.as_os_str()).map_err(|_| ())?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|_| ())?;
        Ok((
            Self {
                include_depth: depth,
                file_meta: Some(ConfFileMeta::from_path(full_path)),
            },
            contents,
        ))
    }

    /// Load a string containing an `Hocon` document. Includes are not supported when
    /// loading from a string
    pub fn load_from_str(s: &str) -> Result<Hocon, ()> {
        let conf = Self {
            include_depth: 0,
            file_meta: None,
        };
        conf.load_from_str_of_conf_file(s)
    }

    /// Load the HOCON configuration file containing an `Hocon` document
    pub fn load_from_file(path: &str) -> Result<Hocon, ()> {
        let (conf, contents) = Self::load_file("", path, 0)?;
        conf.load_from_str_of_conf_file(&contents)
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
            Hocon::Integer(ref v) => Some(*v as f64),
            Hocon::String(ref v) => v.parse::<f64>().ok(),
            _ => None,
        }
    }

    /// Try to cast a value as a `i64` value
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Hocon::Integer(ref v) => Some(*v),
            Hocon::String(ref v) => v.parse::<i64>().ok(),
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
            Hocon::String(ref v) if v == "yes" || v == "true" || v == "on" => Some(true),
            Hocon::String(ref v) if v == "no" || v == "false" || v == "off" => Some(false),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_on_string() {
        let val = Hocon::String(String::from("test"));

        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_f64(), None);
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), Some(String::from("test")));
        assert_eq!(val[0], Hocon::BadValue);
        assert_eq!(val["a"], Hocon::BadValue);
    }

    #[test]
    fn access_on_real() {
        let val = Hocon::Real(5.6);

        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_f64(), Some(5.6));
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), Some(String::from("5.6")));
        assert_eq!(val[0], Hocon::BadValue);
        assert_eq!(val["a"], Hocon::BadValue);
    }

    #[test]
    fn access_on_integer() {
        let val = Hocon::Integer(5);

        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_f64(), Some(5.0));
        assert_eq!(val.as_i64(), Some(5));
        assert_eq!(val.as_string(), Some(String::from("5")));
        assert_eq!(val[0], Hocon::BadValue);
        assert_eq!(val["a"], Hocon::BadValue);
    }

    #[test]
    fn access_on_boolean_false() {
        let val = Hocon::Boolean(false);

        assert_eq!(val.as_bool(), Some(false));
        assert_eq!(val.as_f64(), None);
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), Some(String::from("false")));
        assert_eq!(val[0], Hocon::BadValue);
        assert_eq!(val["a"], Hocon::BadValue);
    }

    #[test]
    fn access_on_boolean_true() {
        let val = Hocon::Boolean(true);

        assert_eq!(val.as_bool(), Some(true));
        assert_eq!(val.as_f64(), None);
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), Some(String::from("true")));
        assert_eq!(val[0], Hocon::BadValue);
        assert_eq!(val["a"], Hocon::BadValue);
    }

    #[test]
    fn access_on_null() {
        let val = Hocon::Null;

        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_f64(), None);
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), None);
        assert_eq!(val[0], Hocon::BadValue);
        assert_eq!(val["a"], Hocon::BadValue);
    }

    #[test]
    fn access_on_bad_value() {
        let val = Hocon::BadValue;

        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_f64(), None);
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), None);
        assert_eq!(val[0], Hocon::BadValue);
        assert_eq!(val["a"], Hocon::BadValue);
    }

    #[test]
    fn access_on_array() {
        let val = Hocon::Array(vec![Hocon::Integer(5), Hocon::Integer(6)]);

        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_f64(), None);
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), None);
        assert_eq!(val[0], Hocon::Integer(5));
        assert_eq!(val[1], Hocon::Integer(6));
        assert_eq!(val[2], Hocon::BadValue);
        assert_eq!(val["a"], Hocon::BadValue);
    }

    #[test]
    fn access_on_hash() {
        let mut hm = HashMap::new();
        hm.insert(String::from("a"), Hocon::Integer(5));
        hm.insert(String::from("b"), Hocon::Integer(6));
        let val = Hocon::Hash(hm);

        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_f64(), None);
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), None);
        assert_eq!(val[0], Hocon::BadValue);
        assert_eq!(val["a"], Hocon::Integer(5));
        assert_eq!(val["b"], Hocon::Integer(6));
        assert_eq!(val["c"], Hocon::BadValue);
    }

    #[test]
    fn cast_string() {
        assert_eq!(Hocon::String(String::from("true")).as_bool(), Some(true));
        assert_eq!(Hocon::String(String::from("yes")).as_bool(), Some(true));
        assert_eq!(Hocon::String(String::from("on")).as_bool(), Some(true));
        assert_eq!(Hocon::String(String::from("false")).as_bool(), Some(false));
        assert_eq!(Hocon::String(String::from("no")).as_bool(), Some(false));
        assert_eq!(Hocon::String(String::from("off")).as_bool(), Some(false));

        assert_eq!(Hocon::String(String::from("5.6")).as_f64(), Some(5.6));
        assert_eq!(Hocon::String(String::from("5.6")).as_i64(), None);
        assert_eq!(Hocon::String(String::from("5")).as_f64(), Some(5.0));
        assert_eq!(Hocon::String(String::from("5")).as_i64(), Some(5));
    }

    #[test]
    fn read_from_properties() {
        let s = r#"a.b:c"#;
        let doc = dbg!(HoconLoader {
            include_depth: 0,
            file_meta: Some(ConfFileMeta::from_path(
                Path::new("file.properties").to_path_buf()
            ))
        }
        .load_from_str_of_conf_file(s,));
        assert!(doc.is_ok());
        assert_eq!(doc.unwrap()["a"]["b"].as_string(), Some(String::from("c")));
    }

    #[test]
    fn read_from_hocon() {
        let s = r#"a.b:c"#;
        let doc = dbg!(HoconLoader {
            include_depth: 0,
            file_meta: Some(ConfFileMeta::from_path(
                Path::new("file.conf").to_path_buf()
            ))
        }
        .load_from_str_of_conf_file(s,));
        assert!(doc.is_ok());
        assert_eq!(doc.unwrap()["a"]["b"].as_string(), Some(String::from("c")));
    }

}
