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
//! This implementation goal is to be as permissive as possible, returning a valid document
//! with all errors wrapped in [`Hocon::BadValue`](enum.Hocon.html#variant.BadValue) when a
//! correct value cannot be computed. [`strict`](struct.HoconLoader.html#method.strict) mode
//! can be enabled to return the first [`Error`](enum.Error.html) encountered instead.
//!
//! # Examples
//!
//! ## Parsing a string to a struct using serde
//!
//! ```rust
//! use serde::Deserialize;
//! use hocon::Error;
//!
//! #[derive(Deserialize)]
//! struct Configuration {
//!     host: String,
//!     port: u8,
//!     auto_connect: bool,
//! }
//!
//! # fn main() -> Result<(), Error> {
//! let s = r#"{
//!     host: 127.0.0.1
//!     port: 80
//!     auto_connect: false
//! }"#;
//!
//! # #[cfg(feature = "serde-support")]
//! let conf: Configuration = hocon::de::from_str(s)?;
//!
//! # Ok(())
//! # }
//! ```
//!
//! ## Reading from a string and getting value directly
//!
//! ```rust
//! use hocon::{HoconLoader,Error};
//!
//! # fn main() -> Result<(), Error> {
//! let s = r#"{ a: 7 }"#;
//!
//! let doc = HoconLoader::new()
//!     .load_str(s)?
//!     .hocon()?;
//!
//! let a = doc["a"].as_i64();
//! assert_eq!(a, Some(7));
//! # Ok(())
//! # }
//! ```
//!
//! ## Deserializing to a struct using `serde`
//!
//! ```rust
//! use serde::Deserialize;
//!
//! use hocon::{HoconLoader,Error};
//!
//! #[derive(Deserialize)]
//! struct Configuration {
//!     host: String,
//!     port: u8,
//!     auto_connect: bool,
//! }
//!
//! # fn main() -> Result<(), Error> {
//! let s = r#"{
//!     host: 127.0.0.1
//!     port: 80
//!     auto_connect: false
//! }"#;
//!
//! # #[cfg(feature = "serde-support")]
//! let conf: Configuration = HoconLoader::new()
//!     .load_str(s)?
//!     .resolve()?;
//! # Ok(())
//! # }
//!  ```
//!
//! ## Reading from a file
//!
//! Example file:
//! [tests/data/basic.conf](https://raw.githubusercontent.com/mockersf/hocon.rs/master/tests/data/basic.conf)
//!
//! ```rust
//! use hocon::{HoconLoader,Error};
//!
//! # fn main() -> Result<(), Error> {
//! let doc = HoconLoader::new()
//!     .load_file("tests/data/basic.conf")?
//!     .hocon()?;
//!
//! let a = doc["a"].as_i64();
//! assert_eq!(a, Some(5));
//! # Ok(())
//! # }
//! ```
//!
//! ## Reading from several documents
//!
//! Example file:
//! [tests/data/basic.conf](https://raw.githubusercontent.com/mockersf/hocon.rs/master/tests/data/basic.conf)
//!
//! ```rust
//! use hocon::{HoconLoader,Error};
//!
//! # fn main() -> Result<(), Error> {
//! let s = r#"{
//!     a: will be changed
//!     unchanged: original value
//! }"#;
//!
//! let doc = HoconLoader::new()
//!     .load_str(s)?
//!     .load_file("tests/data/basic.conf")?
//!     .hocon()?;
//!
//! let a = doc["a"].as_i64();
//! assert_eq!(a, Some(5));
//! let unchanged = doc["unchanged"].as_string();
//! assert_eq!(unchanged, Some(String::from("original value")));
//! # Ok(())
//! # }
//! ```
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
//! use hocon::{HoconLoader,Error};
//!
//! #[derive(Deserialize)]
//! struct Configuration {
//!     host: String,
//!     port: u8,
//!     auto_connect: bool,
//! }
//!
//! # fn main() -> Result<(), Error> {
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
#[cfg(feature = "serde-support")]
pub use crate::serde::de;

/// Helper to load an HOCON file. This is used to set up the HOCON loader's option,
/// like strict mode, disabling system environment, and to buffer several documents.
///
/// # Strict mode
///
/// If strict mode is enabled with [`strict()`](struct.HoconLoader.html#method.strict),
/// loading a document will return the first error encountered. Otherwise, most errors
/// will be wrapped in a [`Hocon::BadValue`](enum.Hocon.html#variant.BadValue).
///
/// # Usage
///
/// ```rust
/// # use hocon::{HoconLoader,Error};
/// # fn main() -> Result<(), Error> {
/// # #[cfg(not(feature = "url-support"))]
/// # let mut loader = HoconLoader::new()         // Creating new loader with default configuration
/// #     .no_system();                           // Disable substituting from system environment
///
/// # #[cfg(feature = "url-support")]
/// let mut loader = HoconLoader::new()         // Creating new loader with default configuration
///     .no_system()                            // Disable substituting from system environment
///     .no_url_include();                      // Disable including files from URLs
///
/// let default_values = r#"{ a = 7 }"#;
/// loader = loader.load_str(default_values)?   // Load values from a string
///     .load_file("tests/data/basic.conf")?    // Load first file
///     .load_file("tests/data/test01.conf")?;  // Load another file
///
/// let hocon = loader.hocon()?;                // Create the Hocon document from the loaded sources
/// # Ok(())
/// # }
/// ```
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
    /// New `HoconLoader` with default configuration
    pub fn new() -> Self {
        Self {
            config: HoconLoaderConfig::default(),
            internal: internals::HoconInternal::empty(),
        }
    }

    /// Disable System environment substitutions
    ///
    /// # Example HOCON document
    ///
    /// ```no_test
    /// "system" : {
    ///     "home"  : ${HOME},
    ///     "pwd"   : ${PWD},
    ///     "shell" : ${SHELL},
    ///     "lang"  : ${LANG},
    /// }
    /// ```
    ///
    /// with system:
    /// ```rust
    /// # use hocon::{Hocon, HoconLoader, Error};
    /// # fn main() -> Result<(), Error> {
    /// # std::env::set_var("SHELL", "/bin/bash");
    /// # let example = r#"{system.shell: ${SHELL}}"#;
    /// assert_eq!(
    ///     HoconLoader::new().load_str(example)?.hocon()?["system"]["shell"],
    ///     Hocon::String(String::from("/bin/bash"))
    /// );
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// without system:
    /// ```rust
    /// # use hocon::{Hocon, HoconLoader, Error};
    /// # fn main() -> Result<(), Error> {
    /// # let example = r#"{system.shell: ${SHELL}}"#;
    /// assert_eq!(
    ///     HoconLoader::new().no_system().load_str(example)?.hocon()?["system"]["shell"],
    ///     Hocon::BadValue(Error::KeyNotFound { key: String::from("SHELL") })
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn no_system(&self) -> Self {
        Self {
            config: HoconLoaderConfig {
                system: false,
                ..self.config.clone()
            },
            ..self.clone()
        }
    }

    /// Disable loading included files from external urls.
    ///
    /// # Example HOCON document
    ///
    /// ```no_test
    /// include url("https://raw.githubusercontent.com/mockersf/hocon.rs/master/tests/data/basic.conf")
    /// ```
    ///
    /// with url include:
    /// ```rust
    /// # use hocon::{Hocon, HoconLoader, Error};
    /// # fn main() -> Result<(), Error> {
    /// assert_eq!(
    ///     HoconLoader::new().load_file("tests/data/include_url.conf")?.hocon()?["d"],
    ///     Hocon::Boolean(true)
    /// );
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// without url include:
    /// ```rust
    /// # use hocon::{Hocon, HoconLoader, Error};
    /// # fn main() -> Result<(), Error> {
    /// assert_eq!(
    ///     HoconLoader::new().no_url_include().load_file("tests/data/include_url.conf")?.hocon()?["d"],
    ///     Hocon::BadValue(Error::MissingKey)
    /// );
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Feature
    ///
    /// This method depends on feature `url-support`
    #[cfg(feature = "url-support")]
    pub fn no_url_include(&self) -> Self {
        Self {
            config: HoconLoaderConfig {
                external_url: false,
                ..self.config.clone()
            },
            ..self.clone()
        }
    }

    /// Sets the HOCON loader to return the first [`Error`](enum.Error.html) encoutered instead
    /// of wrapping it in a [`Hocon::BadValue`](enum.Hocon.html#variant.BadValue) and
    /// continuing parsing
    ///
    /// # Example HOCON document
    ///
    /// ```no_test
    /// {
    ///     a = ${b}
    /// }
    /// ```
    ///
    /// in permissive mode:
    /// ```rust
    /// # use hocon::{Hocon, HoconLoader, Error};
    /// # fn main() -> Result<(), Error> {
    /// # let example = r#"{ a = ${b} }"#;
    /// assert_eq!(
    ///     HoconLoader::new().load_str(example)?.hocon()?["a"],
    ///     Hocon::BadValue(Error::KeyNotFound { key: String::from("b") })
    /// );
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// in strict mode:
    /// ```rust
    /// # use hocon::{Hocon, HoconLoader, Error};
    /// # fn main() -> Result<(), Error> {
    /// # let example = r#"{ a = ${b} }"#;
    /// assert_eq!(
    ///     HoconLoader::new().strict().load_str(example)?.hocon(),
    ///     Err(Error::KeyNotFound { key: String::from("b") })
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn strict(&self) -> Self {
        Self {
            config: HoconLoaderConfig {
                strict: true,
                ..self.config.clone()
            },
            ..self.clone()
        }
    }

    /// Set a new maximum include depth, by default 10
    pub fn max_include_depth(&self, new_max_depth: u8) -> Self {
        Self {
            config: HoconLoaderConfig {
                max_include_depth: new_max_depth,
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

    /// Load a string containing an `Hocon` document. Includes are not supported when
    /// loading from a string
    ///
    /// # Errors
    ///
    /// * [`Error::Parse`](enum.Error.html#variant.Parse) if the document is invalid
    ///
    /// # Additional errors in strict mode
    ///
    /// * [`Error::IncludeNotAllowedFromStr`](enum.Error.html#variant.IncludeNotAllowedFromStr)
    /// if there is an include in the string
    pub fn load_str(self, s: &str) -> Result<Self, Error> {
        self.load_from_str_of_conf_file(FileRead {
            hocon: Some(String::from(s)),
            ..Default::default()
        })
    }

    /// Load the HOCON configuration file containing an `Hocon` document
    ///
    /// # Errors
    ///
    /// * [`Error::File`](enum.Error.html#variant.File) if there was an error reading the
    /// file content
    /// * [`Error::Parse`](enum.Error.html#variant.Parse) if the document is invalid
    ///
    /// # Additional errors in strict mode
    ///
    /// * [`Error::TooManyIncludes`](enum.Error.html#variant.TooManyIncludes)
    /// if there are too many included files within included files. The limit can be
    /// changed with [`max_include_depth`](struct.HoconLoader.html#method.max_include_depth)
    pub fn load_file<P: AsRef<Path>>(&self, path: P) -> Result<Self, Error> {
        let mut file_path = path.as_ref().to_path_buf();
        // pub fn load_file(&self, path: &str) -> Result<Self, Error> {
        // let mut file_path = Path::new(path).to_path_buf();
        if !file_path.has_root() {
            let mut current_path = std::env::current_dir().map_err(|_| Error::File {
                path: String::from(path.as_ref().to_str().unwrap_or("invalid path")),
            })?;
            current_path.push(path.as_ref());
            file_path = current_path;
        }
        let conf = self.config.with_file(file_path);
        let contents = conf.read_file().map_err(|err| {
            let path = match err {
                Error::File { path } => path,
                Error::Include { path } => path,
                Error::Io { message } => message,
                _ => "unmatched error".to_string(),
            };
            Error::File { path }
        })?;
        Self {
            config: conf,
            ..self.clone()
        }
        .load_from_str_of_conf_file(contents)
    }

    /// Load the documents as HOCON
    ///
    /// # Errors in strict mode
    ///
    /// * [`Error::Include`](enum.Error.html#variant.Include) if there was an issue with an
    /// included file
    /// * [`Error::KeyNotFound`](enum.Error.html#variant.KeyNotFound) if there is a substitution
    /// with a key that is not present in the document
    /// * [`Error::DisabledExternalUrl`](enum.Error.html#variant.DisabledExternalUrl) if crate
    /// was built without feature `url-support` and an `include url("...")` was found
    pub fn hocon(self) -> Result<Hocon, Error> {
        let config = &self.config;
        self.internal.merge(config)?.finalize(config)
    }

    /// Deserialize the loaded documents to the target type
    ///
    /// # Errors
    ///
    /// * [`Error::Deserialization`](enum.Error.html#variant.Deserialization) if there was a
    /// serde error during deserialization (missing required field, type issue, ...)
    ///
    /// # Additional errors in strict mode
    ///
    /// * [`Error::Include`](enum.Error.html#variant.Include) if there was an issue with an
    /// included file
    /// * [`Error::KeyNotFound`](enum.Error.html#variant.KeyNotFound) if there is a substitution
    /// with a key that is not present in the document
    /// * [`Error::DisabledExternalUrl`](enum.Error.html#variant.DisabledExternalUrl) if crate
    /// was built without feature `url-support` and an `include url("...")` was found
    #[cfg(feature = "serde-support")]
    pub fn resolve<'de, T>(self) -> Result<T, Error>
    where
        T: ::serde::Deserialize<'de>,
    {
        self.hocon()?.resolve()
    }
}

#[cfg(test)]
mod tests {
    use super::{ConfFileMeta, Hocon, HoconLoader, HoconLoaderConfig};
    use std::path::Path;

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

    use serde::Deserialize;

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

    #[cfg(feature = "serde-support")]
    #[test]
    fn error_deserializing_struct() {
        let doc = r#"{
            int:"not an int", float:543.12, boolean:false, string: test,
            vec_sub:[]
        }"#;

        let res: Result<WithSubStruct, _> = dbg!(HoconLoader::new().load_str(doc))
            .expect("during test")
            .resolve();
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            super::Error::Deserialization {
                message: String::from("int: Invalid type for field \"int\", expected integer")
            }
        );
    }

    #[cfg(feature = "url-support")]
    #[test]
    fn can_disable_url_include() {
        let doc = dbg!(HoconLoader::new()
            .no_url_include()
            .load_file("tests/data/include_url.conf")
            .unwrap()
            .hocon())
        .unwrap();
        assert_eq!(doc["d"], Hocon::BadValue(super::Error::MissingKey));
        assert_eq!(
            doc["https://raw.githubusercontent.com/mockersf/hocon.rs/master/tests/data/basic.conf"],
            Hocon::BadValue(
                super::Error::Include {
                    path: String::from("https://raw.githubusercontent.com/mockersf/hocon.rs/master/tests/data/basic.conf")
                }
            )
        );
    }
}
