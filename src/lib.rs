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
//! let doc = HoconLoader::new().load_str(s).unwrap().hocon().unwrap();
//! let a = doc["a"].as_i64();
//! ```
//!
//! Support serde to deserialize to a `struct`
//!
//! ```rust
//! use serde::Deserialize;
//!
//! use hocon::HoconLoader;
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
//! let conf: Configuration = HoconLoader::new().load_str(s).unwrap().resolve().unwrap();
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

#[cfg(feature = "serde-support")]
mod serde;

#[derive(Debug, Clone)]
pub(crate) enum FileType {
    Properties,
    Hocon,
    Json,
    All,
}

#[derive(Default, Debug)]
pub(crate) struct FileRead {
    properties: Option<String>,
    json: Option<String>,
    hocon: Option<String>,
}
impl FileRead {
    fn from_file_type(ft: &FileType, s: String) -> Self {
        match ft {
            FileType::Properties => Self {
                properties: Some(s),
                ..Default::default()
            },
            FileType::Json => Self {
                json: Some(s),
                ..Default::default()
            },
            FileType::Hocon => Self {
                hocon: Some(s),
                ..Default::default()
            },
            FileType::All => unimplemented!(),
        }
    }
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
                Some("json") => FileType::Json,
                Some("conf") => FileType::Hocon,
                _ => FileType::All,
            },
        }
    }
}

#[derive(Debug, Clone)]
struct HoconLoaderConfig {
    include_depth: usize,
    file_meta: Option<ConfFileMeta>,
    system: bool,
}

impl Default for HoconLoaderConfig {
    fn default() -> Self {
        Self {
            include_depth: 0,
            file_meta: None,
            system: true,
        }
    }
}

impl HoconLoaderConfig {
    pub(crate) fn included_from(&self) -> Self {
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

    pub(crate) fn parse_str_to_internal(
        &self,
        s: FileRead,
    ) -> Result<internals::HoconInternal, ()> {
        let mut internal = internals::HoconInternal::empty();
        if let Some(properties) = s.properties {
            internal = internal.add(
                java_properties::read(properties.as_bytes())
                    .map(internals::HoconInternal::from_properties)
                    .map_err(|_| ())?,
            );
        };
        if let Some(json) = s.json {
            internal = internal.add(
                parser::root(format!("{}\n\0", json).as_bytes(), self)
                    .map_err(|_| ())?
                    .1,
            );
        };
        if let Some(hocon) = s.hocon {
            internal = internal.add(
                parser::root(format!("{}\n\0", hocon).as_bytes(), self)
                    .map_err(|_| ())?
                    .1,
            );
        };

        Ok(internal)
    }

    pub(crate) fn read_file_to_string(path: PathBuf) -> Result<String, ()> {
        let mut file = File::open(path.as_os_str()).map_err(|_| ())?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|_| ())?;
        Ok(contents)
    }

    pub(crate) fn read_file(&self) -> Result<FileRead, ()> {
        let full_path = self.file_meta.clone().unwrap().full_path;
        match self.file_meta.as_ref().map(|fm| &fm.file_type) {
            Some(FileType::All) => Ok(FileRead {
                hocon: Self::read_file_to_string({
                    let mut path = full_path.clone();
                    path.set_extension("conf");
                    path
                })
                .ok(),
                json: Self::read_file_to_string({
                    let mut path = full_path.clone();
                    path.set_extension("json");
                    path
                })
                .ok(),
                properties: Self::read_file_to_string({
                    let mut path = full_path.clone();
                    path.set_extension("properties");
                    path
                })
                .ok(),
            }),
            Some(ft) => Ok(FileRead::from_file_type(
                ft,
                Self::read_file_to_string(full_path)?,
            )),
            _ => unimplemented!(),
        }
        // Ok(vec![contents])
    }
}

/// Helper to load an HOCON file
#[derive(Debug, Clone)]
pub struct HoconLoader {
    config: HoconLoaderConfig,
    internal: internals::HoconInternal,
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
            config: HoconLoaderConfig::default(),
            internal: internals::HoconInternal::empty(),
        }
    }

    /// Disable System environment substitutions
    pub fn no_system(&self) -> Self {
        Self {
            config: HoconLoaderConfig {
                system: false,
                ..self.config.clone()
            },
            ..self.clone()
        }
    }

    pub(crate) fn load_from_str_of_conf_file(self, s: FileRead) -> Result<Self, ()> {
        Ok(Self {
            internal: self.internal.add(self.config.parse_str_to_internal(s)?),
            config: self.config,
        })
    }

    /// Deserialize the loaded documents to the target type
    #[cfg(feature = "serde-support")]
    pub fn resolve<'de, T>(self) -> Result<T, ()>
    where
        T: ::serde::Deserialize<'de>,
    {
        self.hocon()
            .and_then(|hocon| crate::serde::from_hocon(hocon).map_err(|_| ()))
    }

    /// Load the documents as HOCON
    pub fn hocon(self) -> Result<Hocon, ()> {
        let config = &self.config;
        self.internal
            .merge()
            .map(|intermediate| intermediate.finalize(config))
    }

    /// Load a string containing an `Hocon` document. Includes are not supported when
    /// loading from a string
    pub fn load_str(self, s: &str) -> Result<Self, ()> {
        self.load_from_str_of_conf_file(FileRead {
            hocon: Some(String::from(s)),
            ..Default::default()
        })
    }

    /// Load the HOCON configuration file containing an `Hocon` document
    pub fn load_file(&self, path: &str) -> Result<Self, ()> {
        let file_path = Path::new(path).to_path_buf();
        let conf = self.config.with_file(file_path);
        let contents = conf.read_file()?;
        Self {
            config: conf,
            ..self.clone()
        }
        .load_from_str_of_conf_file(contents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_from_properties() {
        let s = r#"a.b:c"#;
        let loader = dbg!(HoconLoader {
            config: HoconLoaderConfig {
                file_meta: Some(ConfFileMeta::from_path(
                    Path::new("file.properties").to_path_buf()
                )),
                ..Default::default()
            },
            ..Default::default()
        }
        .load_str(s));
        assert!(loader.is_ok());

        let doc: Result<Hocon, _> = loader.unwrap().hocon();
        assert!(doc.is_ok());
        assert_eq!(doc.unwrap()["a"]["b"].as_string(), Some(String::from("c")));
    }

    #[test]
    fn read_from_hocon() {
        let s = r#"a.b:c"#;
        let loader = dbg!(HoconLoader {
            config: HoconLoaderConfig {
                file_meta: Some(ConfFileMeta::from_path(
                    Path::new("file.conf").to_path_buf()
                )),
                ..Default::default()
            },
            ..Default::default()
        }
        .load_str(s));
        assert!(loader.is_ok());

        let doc: Result<Hocon, _> = loader.unwrap().hocon();
        assert!(doc.is_ok());
        assert_eq!(doc.unwrap()["a"]["b"].as_string(), Some(String::from("c")));
    }

    use ::serde::Deserialize;

    #[derive(Deserialize, Debug)]
    struct Simple {
        int: i64,
        float: f64,
        option_int: Option<u64>,
    }
    #[derive(Deserialize, Debug)]
    struct WithSubStruct {
        vec_sub: Vec<Simple>,
        int: i32,
        float: f32,
        boolean: bool,
        string: String,
    }

    #[cfg(feature = "serde-support")]
    #[test]
    fn can_deserialize_struct() {
        let doc = r#"{int:56, float:543.12, boolean:false, string: test,
        vec_sub:[
            {int:8, float:1.5, option_int:1919},
            {int:8, float:0                   },
            {int:1, float:2,   option_int:null},
]}"#;

        let res: Result<WithSubStruct, _> =
            dbg!(HoconLoader::new().load_str(doc).unwrap().resolve());
        assert!(res.is_ok());
        assert_eq!(res.unwrap().int, 56)
    }

}
