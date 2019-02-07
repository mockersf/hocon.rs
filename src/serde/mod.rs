//! Deserialization module using serde

mod de;

pub mod error;

pub use de::{from_file_path, from_str};
pub use error::Error;
