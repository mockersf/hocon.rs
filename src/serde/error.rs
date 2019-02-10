//! This type represents all possible errors that can occur when serializing or deserializing DynamoDB data.

use serde;
use std;

/// Alias for a Result with the error type `serde_dynamodb::Error`.
pub type Result<T> = std::result::Result<T, Error>;

/// This type represents all possible errors that can occur when serializing to or deserializing from DynamoDB.
#[derive(Debug)]
pub struct Error {
    /// Message describing the error
    pub message: String,
}
impl serde::ser::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Error {
        Error {
            message: format!("{}", msg),
        }
    }
}
impl serde::de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Error {
        Error {
            message: format!("{}", msg),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.message, f)
    }
}
impl std::error::Error for Error {
    fn description(&self) -> &str {
        &self.message
    }

    fn cause(&self) -> Option<&std::error::Error> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::Error as SerdeError;
    use std::error::Error as StdError;

    #[test]
    fn can_display_error() {
        let error: Error = Error::custom("my error");
        
        assert_eq!(format!("{}", error), "my error");
        assert_eq!(error.description(), "my error");
        assert!(error.cause().is_none());
    }
}