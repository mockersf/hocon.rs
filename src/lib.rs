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
//! let doc = HoconLoader::new().load_from_str(s).unwrap();
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

use std::ffi::OsStr;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

mod internals;
mod parser;
mod value;
pub use value::Hocon;

// #[cfg(feature = "serde-support")]
pub mod serde;

#[derive(Debug, Clone)]
pub(crate) enum FileType {
    Properties,
    Hocon,
}

#[derive(Debug, Clone)]
pub(crate) struct ConfFileMeta {
    path: PathBuf,
    file_name: String,
    full_path: PathBuf,
    file_type: FileType,
}
impl ConfFileMeta {
    fn from_path(path: PathBuf) -> Self {
        let file = path.file_name().unwrap().to_str().unwrap();
        let mut parent_path = path.clone();
        parent_path.pop();

        Self {
            path: parent_path,
            file_name: String::from(file),
            full_path: path.clone(),
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
    system: bool,
}

impl Default for HoconLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl HoconLoader {
    /// New default `HoconLoader`
    pub fn new() -> Self {
        Self {
            include_depth: 0,
            file_meta: None,
            system: true,
        }
    }

    /// Disable System environment substitutions
    pub fn no_system(&self) -> Self {
        Self {
            system: false,
            ..self.clone()
        }
    }

    pub(crate) fn included_from(&self) -> HoconLoader {
        Self {
            include_depth: self.include_depth + 1,
            ..self.clone()
        }
    }

    pub(crate) fn with_file(&self, path: PathBuf) -> Self {
        match self.file_meta.as_ref() {
            Some(file_meta) => Self {
                file_meta: Some(ConfFileMeta::from_path(file_meta.clone().path.join(path))),
                ..self.clone()
            },
            None => Self {
                file_meta: Some(ConfFileMeta::from_path(path)),
                ..self.clone()
            },
        }
    }

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
        .map(|intermediate| intermediate.finalize(self))
    }

    pub(crate) fn load_file(&self) -> Result<String, ()> {
        // let full_path = self.file_meta.clone().unwrap().path.as_path().join(path);
        let full_path = self.file_meta.clone().unwrap().full_path;
        let mut file = File::open(dbg!(full_path.as_os_str())).map_err(|_| ())?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|_| ())?;
        Ok(contents)
    }

    /// Load a string containing an `Hocon` document. Includes are not supported when
    /// loading from a string
    pub fn load_from_str(&self, s: &str) -> Result<Hocon, ()> {
        self.load_from_str_of_conf_file(s)
    }

    /// Load the HOCON configuration file containing an `Hocon` document
    pub fn load_from_file(&self, path: &str) -> Result<Hocon, ()> {
        let file_path = Path::new(path).to_path_buf();
        let conf = self.with_file(file_path);
        let contents = conf.load_file()?;
        conf.load_from_str(&contents)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_from_properties() {
        let s = r#"a.b:c"#;
        let doc = dbg!(HoconLoader {
            include_depth: 0,
            file_meta: Some(ConfFileMeta::from_path(
                Path::new("file.properties").to_path_buf()
            )),
            system: true,
        }
        .load_from_str_of_conf_file(s));
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
            )),
            system: true,
        }
        .load_from_str_of_conf_file(s));
        assert!(doc.is_ok());
        assert_eq!(doc.unwrap()["a"]["b"].as_string(), Some(String::from("c")));
    }

}
