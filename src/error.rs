use failure::Fail;

/// Errors that can be encountered while reading a HOCON document
#[derive(Debug, Fail)]
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
    /// Error readign a file
    #[fail(display = "Error reading file")]
    ReadFileError,
    /// Error including document with External URL as feature has been disabled
    #[fail(display = "Error including document with External URL as feature has been disabled")]
    DisabledExternalUrError,
}
