use failure::Fail;

/// Errors that can be encountered while reading a HOCON document
#[derive(Debug, Fail, Clone, PartialEq)]
pub enum Error {
    /// Error reading a file. This can be a file not found, a permission issue, ...
    #[fail(display = "Error reading file '{}'", path)]
    File {
        /// Path to the file being read
        path: String,
    },

    /// Error while parsing a document. The document is not valid HOCON
    #[fail(display = "Error wile parsing document")]
    Parse,

    /// Error including a document
    #[fail(display = "Error including document at {}", path)]
    Include {
        /// Path of the included file
        path: String,
    },

    /// Error processing deep includes. You can change the maximum depth using max_include_depth
    #[fail(
        display = "Error processing deep includes. You can change the maximum depth using max_include_depth"
    )]
    TooManyIncludes,

    /// Error processing includes from a str source. This is not allowed
    #[fail(display = "Error processing includes from a str source. This is not allowed")]
    IncludeNotAllowedFromStr,

    /// Error including document with External URL as feature has been disabled
    #[fail(display = "Error including document with External URL as feature has been disabled")]
    DisabledExternalUrl,

    /// Error looking for a key
    #[fail(display = "Error looking for key '{}'", key)]
    KeyNotFound {
        /// Key that was searched
        key: String,
    },

    /// Error getting a value because key is not present
    #[fail(display = "Error getting a value because key is not present")]
    MissingKey,

    /// Error getting a value because of an invalid key type
    #[fail(display = "Error getting a value because of an invalid key type")]
    InvalidKey,

    /// Error deserializing
    #[fail(display = "Error deserializing")]
    Deserialization {
        /// Error message returned from deserialization
        message: String,
    },
}
