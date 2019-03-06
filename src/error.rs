use failure::Fail;

/// Errors that can be encountered while reading a HOCON document
#[derive(Debug, Fail, Clone, PartialEq)]
pub enum HoconError {
    /// Error parsing a document
    #[fail(display = "Error parsing document")]
    ParseError,

    /// Error including a document
    #[fail(display = "Error including document at {}", path)]
    IncludeError {
        /// Path of the included file
        path: String,
    },

    /// Error processing more than 10 includes deep
    #[fail(display = "Error processing more than 10 includes deep")]
    TooManyIncludesError,

    /// Error looking for a key
    #[fail(display = "Error looking for a key")]
    KeyNotFoundError,

    /// Error including document with External URL as feature has been disabled
    #[fail(display = "Error including document with External URL as feature has been disabled")]
    DisabledExternalUrlError,

    /// Error reading a file
    #[fail(display = "Error reading a file")]
    FileError(String),

    /// Error deserializing
    #[fail(display = "Error deserializing")]
    DeserializationError(String),
}
