use thiserror::Error;

/// Errors that can be encountered while reading a HOCON document
#[derive(Error, Debug, Clone, PartialEq)]
pub enum Error {
    /// Captures IO-Errors. Usually we would use a transparent error but io::Error is not clonable
    #[error("Error during IO")]
    IO {
        /// the description of the original IOError
        message: String,
    },

    /// Error reading a file. This can be a file not found, a permission issue, ...
    #[error("Error reading file '{path:?}'")]
    File {
        /// Path to the file being read
        path: String,
    },
    /// Error while parsing a document. The document is not valid HOCON
    #[error("Error wile parsing document")]
    Parse,
    /// Error including a document
    #[error("Error including document at '{path:?}'")]
    Include {
        /// Path of the included file
        path: String,
    },
    /// Error processing deep includes. You can change the maximum depth using max_include_depth
    #[error("Error processing deep includes")]
    TooManyIncludes,
    /// Error processing includes from a str source. This is not allowed
    #[error("Error processing includes from a str source")]
    IncludeNotAllowedFromStr,
    /// Error including document with External URL as feature has been disabled
    #[error("Error including document with External URL as feature has been disabled")]
    DisabledExternalUrl,
    /// Error looking for a key
    #[error("Error looking for key '{key:?}'")]
    KeyNotFound {
        /// Key that was searched
        key: String,
    },
    /// Error getting a value because key is not present
    #[error("Error getting a value because key is not present")]
    MissingKey,
    /// Error getting a value because of an invalid key type
    #[error("Error getting a value because of an invalid key type")]
    InvalidKey,
    /// Error deserializing
    #[error("Error deserializing: {message:?}")]
    Deserialization {
        /// Error message returned from deserialization
        message: String,
    },
}

/// this is only needed because this crate heavily relies on Clone and io:Error doesnt implement Clone
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IO {
            message: e.to_string(),
        }
    }
}
