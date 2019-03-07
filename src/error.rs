use std::error::Error as StdError;

impl StdError for Error {}
use std::fmt;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::File { path } => write!(f, "Error reading file '{}'", path),
            Error::Parse => write!(f, "Error wile parsing document"),
            Error::Include { path } => write!(f, "Error including document at '{}'", path),
            Error::TooManyIncludes => write!(f, "Error processing deep includes"),
            Error::IncludeNotAllowedFromStr => {
                write!(f, "Error processing includes from a str source")
            }
            Error::DisabledExternalUrl => write!(
                f,
                "Error including document with External URL as feature has been disabled"
            ),
            Error::KeyNotFound { key } => write!(f, "Error looking for key '{}'", key),
            Error::MissingKey => write!(f, "Error getting a value because key is not present"),
            Error::InvalidKey => write!(f, "Error getting a value because of an invalid key type"),
            Error::Deserialization { message } => write!(f, "Error deserializing: {}", message),
        }
    }
}

/// Errors that can be encountered while reading a HOCON document
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// Error reading a file. This can be a file not found, a permission issue, ...
    File {
        /// Path to the file being read
        path: String,
    },
    /// Error while parsing a document. The document is not valid HOCON
    Parse,
    /// Error including a document
    Include {
        /// Path of the included file
        path: String,
    },
    /// Error processing deep includes. You can change the maximum depth using max_include_depth
    TooManyIncludes,
    /// Error processing includes from a str source. This is not allowed
    IncludeNotAllowedFromStr,
    /// Error including document with External URL as feature has been disabled
    DisabledExternalUrl,
    /// Error looking for a key
    KeyNotFound {
        /// Key that was searched
        key: String,
    },
    /// Error getting a value because key is not present
    MissingKey,
    /// Error getting a value because of an invalid key type
    InvalidKey,
    /// Error deserializing
    Deserialization {
        /// Error message returned from deserialization
        message: String,
    },
}
