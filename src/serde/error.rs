pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub(crate) struct Error {
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
impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::Error as SerdeError;
    use std::error::Error as StdError;

    #[test]
    fn can_display_error() {
        let error: Error = Error::custom("my error");

        assert_eq!(format!("{}", error), "my error");
        assert_eq!(error.to_string(), "my error");
        assert!(error.source().is_none());
    }
}
