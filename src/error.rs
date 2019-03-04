//! Errors that can be encountered while reading a HOCON document

use failure::Fail;

/// Errors that can be encountered while reading a HOCON document
#[derive(Debug, Fail, Copy, Clone)]
pub enum HoconError {
    /// Error parsing a document
    #[fail(display = "Error parsing document")]
    ParseError,
    /// Error finalizing a document
    #[fail(display = "Error finalizing document")]
    FinalizeError,
    /// Error including a document
    #[fail(display = "Error including document")]
    IncludeError,
    /// Error readign a file
    #[fail(display = "Error reading file")]
    ReadFileError,
}
