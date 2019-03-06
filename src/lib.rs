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
//! Parse HOCON configuration files in Rust following the
//! [HOCON Specifications](https://github.com/lightbend/config/blob/master/HOCON.md).
//!
//! This implementation goal is to be as permissive as possible, returning a
//! [`Hocon::BadValue`](enum.Hocon.html#variant.BadValue) when a correct value cannot be computed.
//! [`strict`](struct.HoconLoader.html#method.strict) mode can be enabled to return an
//! [`Error`](enum.Error.html) instead.
//!
//! # Examples
//!
//! ```rust
//! use hocon::HoconLoader;
//!
//! # fn main() -> Result<(), failure::Error> {
//! let s = r#"{"a":5}"#;
//! let doc = HoconLoader::new().load_str(s)?.hocon()?;
//! let a = doc["a"].as_i64();
//! # Ok(())
//! # }
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
//! # fn main() -> Result<(), failure::Error> {
//! let s = r#"{host: 127.0.0.1, port: 80, auto_connect: false}"#;
//!
//! # #[cfg(feature = "serde-support")]
//! let conf: Configuration = HoconLoader::new().load_str(s)?.resolve()?;
//! # Ok(())
//! # }
//!  ```
//!
//! # Features
//!
//! All features are enabled by default. They can be disabled to reduce dependencies.
//!
//! ### `url-support`
//!
//! This feature enable fetching URLs in includes  with `include url("http://mydomain.com/myfile.conf")` (see
//! [spec](https://github.com/lightbend/config/blob/master/HOCON.md#include-syntax)). If disabled,
//! includes will only load local files specified with `include "path/to/file.conf"` or
//! `include file("path/to/file.conf")`.
//!
//! ### `serde-support`
//!
//! This feature enable deserializing to a `struct` implementing `Deserialize` using `serde`
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
//! # fn main() -> Result<(), failure::Error> {
//! let s = r#"{host: 127.0.0.1, port: 80, auto_connect: false}"#;
//!
//! # #[cfg(feature = "serde-support")]
//! let conf: Configuration = HoconLoader::new().load_str(s)?.resolve()?;
//! # Ok(())
//! # }
//!  ```
//!

use std::path::Path;

mod internals;
mod parser;
mod value;
pub use value::Hocon;
mod error;
pub use error::Error;
pub(crate) mod helper;
mod loader_config;
pub(crate) use loader_config::*;

#[cfg(feature = "serde-support")]
mod serde;

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

    /// Disable loading external urls
    #[cfg(feature = "url-support")]
    pub fn no_external_url(&self) -> Self {
        Self {
            config: HoconLoaderConfig {
                external_url: false,
                ..self.config.clone()
            },
            ..self.clone()
        }
    }

    /// Returns an error when encountering a `Hocon::BadValue`
    #[cfg(feature = "url-support")]
    pub fn strict(&self) -> Self {
        Self {
            config: HoconLoaderConfig {
                strict: true,
                ..self.config.clone()
            },
            ..self.clone()
        }
    }

    /// Set a new max include depth, by default 10
    #[cfg(feature = "url-support")]
    pub fn max_include_depth(&self, new_depth: u8) -> Self {
        Self {
            config: HoconLoaderConfig {
                max_include_depth: new_depth,
                ..self.config.clone()
            },
            ..self.clone()
        }
    }

    pub(crate) fn load_from_str_of_conf_file(self, s: FileRead) -> Result<Self, Error> {
        Ok(Self {
            internal: self.internal.add(self.config.parse_str_to_internal(s)?),
            config: self.config,
        })
    }

    /// Deserialize the loaded documents to the target type
    #[cfg(feature = "serde-support")]
    pub fn resolve<'de, T>(self) -> Result<T, Error>
    where
        T: ::serde::Deserialize<'de>,
    {
        Ok(
            crate::serde::from_hocon(self.hocon()?).map_err(|err| Error::Deserialization {
                message: err.message,
            })?,
        )
    }

    /// Load the documents as HOCON
    pub fn hocon(self) -> Result<Hocon, Error> {
        let config = &self.config;
        self.internal.merge(config)?.finalize(config)
    }

    /// Load a string containing an `Hocon` document. Includes are not supported when
    /// loading from a string
    pub fn load_str(self, s: &str) -> Result<Self, Error> {
        self.load_from_str_of_conf_file(FileRead {
            hocon: Some(String::from(s)),
            ..Default::default()
        })
    }

    /// Load the HOCON configuration file containing an `Hocon` document
    pub fn load_file(&self, path: &str) -> Result<Self, Error> {
        let file_path = Path::new(path).to_path_buf();
        let conf = self.config.with_file(file_path);
        let contents = conf.read_file().map_err(|err| Error::File {
            path: String::from(err.name().unwrap_or(path)),
        })?;
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

        let doc = loader.expect("during test").hocon().expect("during test");
        assert_eq!(doc["a"]["b"].as_string(), Some(String::from("c")));
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

        let doc: Hocon = loader.expect("during test").hocon().expect("during test");
        assert_eq!(doc["a"]["b"].as_string(), Some(String::from("c")));
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

        let res: Result<WithSubStruct, _> = dbg!(HoconLoader::new().load_str(doc))
            .expect("during test")
            .resolve();
        assert!(res.is_ok());
        let res = res.expect("during test");
        assert_eq!(res.int, 56);
        assert_eq!(res.float, 543.12);
        assert_eq!(res.boolean, false);
        assert_eq!(res.string, "test");
        assert_eq!(res.vec_sub[0].int, 8);
        assert_eq!(res.vec_sub[0].float, 1.5);
        assert_eq!(res.vec_sub[0].option_int, Some(1919));
        assert_eq!(res.vec_sub[1].int, 8);
        assert_eq!(res.vec_sub[1].float, 0.0);
        assert_eq!(res.vec_sub[1].option_int, None);
        assert_eq!(res.vec_sub[2].int, 1);
        assert_eq!(res.vec_sub[2].float, 2.0);
        assert_eq!(res.vec_sub[2].option_int, None);
    }

    #[cfg(feature = "serde-support")]
    #[test]
    fn can_deserialize_struct2() {
        let doc = r#"{int:56, float:543.12, boolean:false, string: test,
            vec_sub.1 = {int:8, float:1.5, option_int:1919},
            vec_sub.5 = {int:8, float:0                   },
            vec_sub.8 = {int:1, float:2,   option_int:null},
    }"#;

        let res: Result<WithSubStruct, _> = dbg!(HoconLoader::new().load_str(doc))
            .expect("during test")
            .resolve();
        assert!(res.is_ok());
        let res = res.expect("during test");
        assert_eq!(res.int, 56);
        assert_eq!(res.float, 543.12);
        assert_eq!(res.boolean, false);
        assert_eq!(res.string, "test");
        assert_eq!(res.vec_sub[0].int, 8);
        assert_eq!(res.vec_sub[0].float, 1.5);
        assert_eq!(res.vec_sub[0].option_int, Some(1919));
        assert_eq!(res.vec_sub[1].int, 8);
        assert_eq!(res.vec_sub[1].float, 0.0);
        assert_eq!(res.vec_sub[1].option_int, None);
        assert_eq!(res.vec_sub[2].int, 1);
        assert_eq!(res.vec_sub[2].float, 2.0);
        assert_eq!(res.vec_sub[2].option_int, None);
    }

}
