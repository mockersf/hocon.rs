use failure::Fail;

/// Errors that can be encountered while reading a HOCON document
#[derive(Debug, Fail, Clone, PartialEq)]
pub enum Error {
    /// Error while parsing a document
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

    /// Error looking for a key
    #[fail(display = "Error looking for a key")]
    KeyNotFound,

    /// Error including document with External URL as feature has been disabled
    #[fail(display = "Error including document with External URL as feature has been disabled")]
    DisabledExternalUrl,

    /// Error reading a file
    #[fail(display = "Error reading file '{}'", path)]
    File {
        /// Path to the file being read
        path: String,
    },

    /// Error deserializing
    #[fail(display = "Error deserializing")]
    Deserialization {
        /// Error message returned from deserialization
        message: String,
    },
}
