//! Wrapper for custom deserialization from Hocon

use std::{
    fmt,
    ops::{Deref, DerefMut},
    time::Duration,
};

use serde::{
    de::{self, Deserialize, Visitor},
    Deserializer,
};

use crate::Hocon;

/// Wrapper for custom deserialization from Hocon.
///
/// Implemented for [`Duration`]
///
/// ## As a newtype wrapper
///
/// ```rust
/// # use std::time::Duration;
/// # use hocon::de::wrappers::Serde;
/// # use serde::Deserialize;
/// #[derive(Deserialize, Debug)]
/// struct StructWithDuration {
///     timeout: Serde<Duration>,
/// }
/// # fn usage() {
/// # let doc = r#"{"a":"1 second"}"#;
///
/// let my_struct: StructWithDuration = hocon::de::from_str(doc).unwrap();
/// assert_eq!(*my_struct.timeout, Duration::from_secs(1));
/// # }
/// ```
///
/// ## As a serde attribute
///
/// ```rust
/// # use std::time::Duration;
/// # use hocon::de::wrappers::Serde;
/// # use serde::Deserialize;
/// #[derive(Deserialize, Debug)]
/// struct StructWithDuration {
///     #[serde(deserialize_with = "Serde::<Duration>::with")]
///     timeout: Duration,
/// }
/// # fn usage() {
/// # let doc = r#"{"a":"1 second"}"#;
///
/// let my_struct: StructWithDuration = hocon::de::from_str(doc).unwrap();
/// assert_eq!(my_struct.timeout, Duration::from_secs(1));
/// # }
/// ```
#[doc(alias = "Duration")]
#[derive(Debug)]
pub struct Serde<T>(T);

impl<T> Deref for Serde<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Serde<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

struct StringDurationVisitor;

impl<'de> Visitor<'de> for StringDurationVisitor {
    type Value = Duration;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a duration")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let duration = Hocon::str_as_milliseconds(&v)
            .ok_or_else(|| E::custom(format!("expected duration, found \"{}\"", v)))?;

        Ok(Duration::from_secs_f64(duration / 1000.0))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let duration = Hocon::str_as_milliseconds(v)
            .ok_or_else(|| E::custom(format!("expected duration, found \"{}\"", v)))?;

        Ok(Duration::from_secs_f64(duration / 1000.0))
    }
}

impl<'de> Deserialize<'de> for Serde<Duration> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Serde(deserializer.deserialize_str(StringDurationVisitor)?))
    }
}

impl Serde<Duration> {
    /// Custom deserializer for a duration, to use with Serde `deserialize_with` attribute
    pub fn with<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(deserializer.deserialize_str(StringDurationVisitor)?)
    }
}
