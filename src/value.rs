use std::collections::HashMap;
use std::ops::Index;

/// An HOCON document
///
/// Values can be retrieved as a basic type, with basic cast between some of the value types:
/// [Automatic type conversions](https://github.com/lightbend/config/blob/master/HOCON.md#automatic-type-conversions).
/// If the value if not of the expected type, a `None` will be returned.
///
/// If the value is a [`Hocon::Hash`](enum.Hocon.html#variant.Hash), its values can be
/// accessed by indexing with a `str`, as in `hash[key]`. If the value is an
/// [`Hocon::Array`](enum.Hocon.html#variant.Array), its values can be accessed by indexing
/// with a `usize`. An [`Hocon::Hash`](enum.Hocon.html#variant.Hash) whose keys can be
/// converted to numeric values (`"0"`, `"1"`, ...) can be indexed with a `usize` following
/// the rules described in
/// [Conversion of numerically-indexed objects to arrays](https://github.com/lightbend/config/blob/master/HOCON.md#conversion-of-numerically-indexed-objects-to-arrays).
///
/// Indexing a `Hocon` value with a wrong key type, or a type of value that can't be indexed
/// will return a [`Hocon::BadValue`](enum.Hocon.html#variant.BadValue) with an error of type
/// [`crate::Error::InvalidKey`](enum.Error.html#variant.InvalidKey).
///
/// Indexing a `Hocon` value with a key that is not present will return a
/// [`Hocon::BadValue`](enum.Hocon.html#variant.BadValue) with an error of type
/// [`crate::Error::MissingKey`](enum.Error.html#variant.MissingKey).
///
/// Values can also be accessed as a `Duration` or a size following the rules described in
/// [Units format](https://github.com/lightbend/config/blob/master/HOCON.md#units-format).
///
/// # Usage
///
/// ```rust
/// # use hocon::{HoconLoader, Error, Hocon};
/// # fn main() -> Result<(), failure::Error> {
/// // Accessing a value of the expected type
/// assert_eq!(
///     HoconLoader::new().load_str(r#"{ a: 7 }"#)?.hocon()?["a"].as_i64(),
///     Some(7)
/// );
///
/// // Accessing a value with automatic conversion
/// assert_eq!(
///     HoconLoader::new().load_str(r#"{ a: off }"#)?.hocon()?["a"].as_bool(),
///     Some(false)
/// );
///
/// // Accessing an Array
/// assert_eq!(
///     HoconLoader::new().load_str(r#"{ a: [ first, second ] }"#)?.hocon()?["a"][0].as_string(),
///     Some(String::from("first"))
/// );
///
/// // Accessing an Hash with a missing key
/// assert_eq!(
///     HoconLoader::new().load_str(r#"{ a: 7 }"#)?.hocon()?["b"],
///     Hocon::BadValue(Error::MissingKey)
/// );
///
/// // Accessing an Hash as if it was an Array
/// assert_eq!(
///     HoconLoader::new().load_str(r#"{ a: 7 }"#)?.hocon()?[0],
///     Hocon::BadValue(Error::InvalidKey)
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Hocon {
    /// A floating value
    Real(f64),
    /// An integer value
    Integer(i64),
    /// A string
    String(String),
    /// A boolean
    Boolean(bool),
    /// An array of `Hocon` values
    Array(Vec<Hocon>),
    /// An HashMap of `Hocon` values with keys
    Hash(HashMap<String, Hocon>),
    /// A null value
    Null,
    /// A `BadValue`, marking an error in parsing or a missing value
    BadValue(crate::Error),
}

static NOT_FOUND: Hocon = Hocon::BadValue(crate::Error::MissingKey);
static INVALID_KEY: Hocon = Hocon::BadValue(crate::Error::InvalidKey);

impl<'a> Index<&'a str> for Hocon {
    type Output = Hocon;

    fn index(&self, idx: &'a str) -> &Self::Output {
        match self {
            Hocon::Hash(hash) => hash.get(idx).unwrap_or(&NOT_FOUND),
            _ => &INVALID_KEY,
        }
    }
}
impl Index<usize> for Hocon {
    type Output = Hocon;

    fn index(&self, idx: usize) -> &Self::Output {
        match self {
            Hocon::Array(vec) => vec.get(idx).unwrap_or(&NOT_FOUND),
            Hocon::Hash(hash) => {
                let mut keys_as_usize = hash
                    .keys()
                    .filter_map(|k| k.parse::<usize>().ok().map(|v| (k, v)))
                    .collect::<Vec<_>>();
                keys_as_usize.sort_by(|(_, v0), (_, v1)| v0.cmp(v1));
                keys_as_usize
                    .get(idx)
                    .and_then(|(k, _)| hash.get(*k))
                    .unwrap_or(&INVALID_KEY)
            }
            _ => &INVALID_KEY,
        }
    }
}

impl Hocon {
    /// Try to cast a value as a `f64` value
    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Hocon::Real(ref v) => Some(*v),
            Hocon::Integer(ref v) => Some(*v as f64),
            Hocon::String(ref v) => v.parse::<f64>().ok(),
            _ => None,
        }
    }

    /// Try to cast a value as a `i64` value
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Hocon::Integer(ref v) => Some(*v),
            Hocon::String(ref v) => v.parse::<i64>().ok(),
            _ => None,
        }
    }

    /// Try to cast a value as a `String` value
    pub fn as_string(&self) -> Option<String> {
        match *self {
            Hocon::String(ref v) => Some(v.to_string()),
            Hocon::Boolean(true) => Some("true".to_string()),
            Hocon::Boolean(false) => Some("false".to_string()),
            Hocon::Integer(i) => Some(i.to_string()),
            Hocon::Real(f) => Some(f.to_string()),
            _ => None,
        }
    }

    pub(crate) fn as_internal_string(&self) -> Option<String> {
        match *self {
            Hocon::String(ref v) => Some(v.to_string()),
            Hocon::Boolean(true) => Some("true".to_string()),
            Hocon::Boolean(false) => Some("false".to_string()),
            Hocon::Integer(i) => Some(i.to_string()),
            Hocon::Real(f) => Some(f.to_string()),
            Hocon::Null => Some("null".to_string()),
            _ => None,
        }
    }

    /// Try to cast a value as a `bool` value
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Hocon::Boolean(ref v) => Some(*v),
            Hocon::String(ref v) if v == "yes" || v == "true" || v == "on" => Some(true),
            Hocon::String(ref v) if v == "no" || v == "false" || v == "off" => Some(false),
            _ => None,
        }
    }
}

mod unit_format {
    use nom::*;

    named!(
        parse_float<types::CompleteStr, f64>,
        complete!(flat_map!(recognize_float, parse_to!(f64)))
    );

    pub(crate) fn value_and_unit(s: &str) -> Option<(f64, &str)> {
        match parse_float(types::CompleteStr(s)) {
            Ok((remaining, float)) => Some((float, &remaining)),
            _ => None,
        }
    }
}

macro_rules! units {
    ( match $input:expr, $( $first_unit:expr, $( $unit:expr ),* => $scale:expr ),* ) => {
        match $input {
            $(
                Some((value, $first_unit))
                $(
                    | Some((value, $unit))
                )* => Some(value * $scale),
            )*
            _ => None
        }
    };
}

impl Hocon {
    /// Try to return a value as a size in bytes according to
    /// [size in bytes format](https://github.com/lightbend/config/blob/master/HOCON.md#size-in-bytes-format).
    ///
    /// Bare numbers are taken to be in bytes already, while strings are parsed as a number
    /// plus an optional unit string.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use hocon::{Hocon, HoconLoader, Error};
    /// # fn main() -> Result<(), failure::Error> {
    /// assert_eq!(
    ///     HoconLoader::new().load_str(r#"{ size = 1.5KiB }"#)?.hocon()?["size"].as_bytes(),
    ///     Some(1536.0)
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_bytes(&self) -> Option<f64> {
        match *self {
            Hocon::Integer(ref i) => Some(*i as f64),
            Hocon::Real(ref f) => Some(*f),
            Hocon::String(ref s) => units!(
                match unit_format::value_and_unit(s).map(|(value, unit)| (value, unit.trim())),
                 "", "B", "b", "byte", "bytes"                     => 1.0,
                 "kB", "kilobyte", "kilobytes"                     => 10.0f64.powf(3.0),
                 "MB", "megabyte", "megabytes"                     => 10.0f64.powf(6.0),
                 "GB", "gigabyte", "gigabytes"                     => 10.0f64.powf(9.0),
                 "TB", "terabyte", "terabytes"                     => 10.0f64.powf(12.0),
                 "PB", "petabyte", "petabytes"                     => 10.0f64.powf(15.0),
                 "EB", "exabyte", "exabytes"                       => 10.0f64.powf(18.0),
                 "ZB", "zettabyte", "zettabytes"                   => 10.0f64.powf(21.0),
                 "YB", "yottabyte", "yottabytes"                   => 10.0f64.powf(24.0),
                 "K", "k", "Ki", "KiB", "kibibyte", "kibibytes"    => 2.0f64.powf(10.0),
                 "M", "m", "Mi", "MiB", "mebibyte", "mebibytes"    => 2.0f64.powf(20.0),
                 "G", "g", "Gi", "GiB", "gibibyte", "gibibytes"    => 2.0f64.powf(30.0),
                 "T", "t", "Ti", "TiB", "tebibyte", "tebibytes"    => 2.0f64.powf(40.0),
                 "P", "p", "Pi", "PiB", "pebibyte", "pebibytes"    => 2.0f64.powf(50.0),
                 "E", "e", "Ei", "EiB", "exbibyte", "exbibytes"    => 2.0f64.powf(60.0),
                 "Z", "z", "Zi", "ZiB", "zebibyte", "zebibytes"    => 2.0f64.powf(70.0),
                 "Y", "y", "Yi", "YiB", "yobibyte", "yobibytes"    => 2.0f64.powf(80.0)
            ),
            _ => None,
        }
    }

    /// Try to return a value as a duration in milliseconds according to
    /// [duration format](https://github.com/lightbend/config/blob/master/HOCON.md#duration-format).
    ///
    /// Bare numbers are taken to be in bytes already, while strings are parsed as a number
    /// plus an optional unit string.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use hocon::{Hocon, HoconLoader, Error};
    /// # fn main() -> Result<(), failure::Error> {
    /// assert_eq!(
    ///     HoconLoader::new().load_str(r#"{ duration = 1.5 hour  }"#)?
    ///         .hocon()?["duration"].as_milliseconds(),
    ///     Some(5400000.0)
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_milliseconds(&self) -> Option<f64> {
        match *self {
            Hocon::Integer(ref i) => Some(*i as f64),
            Hocon::Real(ref f) => Some(*f),
            Hocon::String(ref s) => units!(
                match unit_format::value_and_unit(s).map(|(value, unit)| (value, unit.trim())),
                "ns", "nano", "nanos", "nanosecond", "nanoseconds"          => 10.0f64.powf(-6.0),
                "us", "micro", "micros", "microsecond", "microseconds"      => 10.0f64.powf(-3.0),
                "", "ms", "milli", "millis", "millisecond", "milliseconds"  => 1.0,
                "s", "second", "seconds"                                    => 1_000.0,
                "m", "minute", "minutes"                                    => 1_000.0 * 60.0,
                "h", "hour", "hours"                                        => 1_000.0 * 60.0 * 60.0,
                "d", "day", "days"                                          => 1_000.0 * 60.0 * 60.0 * 24.0,
                "w", "week", "weeks"                                        => 1_000.0 * 60.0 * 60.0 * 24.0 * 7.0,
                "mo", "month", "months"                                     => 1_000.0 * 60.0 * 60.0 * 24.0 * 30.0,
                "y", "year", "years"                                        => 1_000.0 * 60.0 * 60.0 * 24.0 * 365.0
            ),
            _ => None,
        }
    }

    /// Try to return a value as a duration in nanoseconds according to
    /// [duration format](https://github.com/lightbend/config/blob/master/HOCON.md#duration-format).
    ///
    /// Bare numbers are taken to be in bytes already, while strings are parsed as a number
    /// plus an optional unit string.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use hocon::{Hocon, HoconLoader, Error};
    /// # fn main() -> Result<(), failure::Error> {
    /// assert_eq!(
    ///     HoconLoader::new().load_str(r#"{ duration = 1.5 hour  }"#)?
    ///         .hocon()?["duration"].as_nanoseconds(),
    ///     Some(5400000000000.0)
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_nanoseconds(&self) -> Option<f64> {
        self.as_milliseconds().map(|v| v * 10.0f64.powf(6.0))
    }

    /// Try to return a value as a duration in microseconds according to
    /// [duration format](https://github.com/lightbend/config/blob/master/HOCON.md#duration-format).
    ///
    /// Bare numbers are taken to be in bytes already, while strings are parsed as a number
    /// plus an optional unit string.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use hocon::{Hocon, HoconLoader, Error};
    /// # fn main() -> Result<(), failure::Error> {
    /// assert_eq!(
    ///     HoconLoader::new().load_str(r#"{ duration = 1.5 hour  }"#)?
    ///         .hocon()?["duration"].as_microseconds(),
    ///     Some(5400000000.0)
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_microseconds(&self) -> Option<f64> {
        self.as_milliseconds().map(|v| v * 10.0f64.powf(3.0))
    }

    /// Try to return a value as a duration in seconds according to
    /// [duration format](https://github.com/lightbend/config/blob/master/HOCON.md#duration-format).
    ///
    /// Bare numbers are taken to be in bytes already, while strings are parsed as a number
    /// plus an optional unit string.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use hocon::{Hocon, HoconLoader, Error};
    /// # fn main() -> Result<(), failure::Error> {
    /// assert_eq!(
    ///     HoconLoader::new().load_str(r#"{ duration = 1.5 hour  }"#)?
    ///         .hocon()?["duration"].as_seconds(),
    ///     Some(5400.0)
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_seconds(&self) -> Option<f64> {
        self.as_milliseconds().map(|v| v * 10.0f64.powf(-3.0))
    }

    /// Try to return a value as a duration in minutes according to
    /// [duration format](https://github.com/lightbend/config/blob/master/HOCON.md#duration-format).
    ///
    /// Bare numbers are taken to be in bytes already, while strings are parsed as a number
    /// plus an optional unit string.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use hocon::{Hocon, HoconLoader, Error};
    /// # fn main() -> Result<(), failure::Error> {
    /// assert_eq!(
    ///     HoconLoader::new().load_str(r#"{ duration = 1.5 hour  }"#)?
    ///         .hocon()?["duration"].as_minutes(),
    ///     Some(90.0)
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_minutes(&self) -> Option<f64> {
        self.as_milliseconds()
            .map(|v| v * 10.0f64.powf(-3.0) / 60.0)
    }

    /// Try to return a value as a duration in hours according to
    /// [duration format](https://github.com/lightbend/config/blob/master/HOCON.md#duration-format).
    ///
    /// Bare numbers are taken to be in bytes already, while strings are parsed as a number
    /// plus an optional unit string.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use hocon::{Hocon, HoconLoader, Error};
    /// # fn main() -> Result<(), failure::Error> {
    /// assert_eq!(
    ///     HoconLoader::new().load_str(r#"{ duration = 1.5 hour  }"#)?
    ///         .hocon()?["duration"].as_hours(),
    ///     Some(1.5)
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_hours(&self) -> Option<f64> {
        self.as_milliseconds()
            .map(|v| v * 10.0f64.powf(-3.0) / 60.0 / 60.0)
    }

    /// Try to return a value as a duration in days according to
    /// [duration format](https://github.com/lightbend/config/blob/master/HOCON.md#duration-format).
    ///
    /// Bare numbers are taken to be in bytes already, while strings are parsed as a number
    /// plus an optional unit string.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use hocon::{Hocon, HoconLoader, Error};
    /// # fn main() -> Result<(), failure::Error> {
    /// assert_eq!(
    ///     HoconLoader::new().load_str(r#"{ duration = 1.5 hour  }"#)?
    ///         .hocon()?["duration"].as_days(),
    ///     Some(0.0625)
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_days(&self) -> Option<f64> {
        self.as_milliseconds()
            .map(|v| v * 10.0f64.powf(-3.0) / 60.0 / 60.0 / 24.0)
    }

    /// Try to return a value as a duration in weeks according to
    /// [duration format](https://github.com/lightbend/config/blob/master/HOCON.md#duration-format).
    ///
    /// Bare numbers are taken to be in bytes already, while strings are parsed as a number
    /// plus an optional unit string.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use hocon::{Hocon, HoconLoader, Error};
    /// # fn main() -> Result<(), failure::Error> {
    /// assert_eq!(
    ///     HoconLoader::new().load_str(r#"{ duration = 1.5 days  }"#)?
    ///         .hocon()?["duration"].as_weeks(),
    ///     Some(0.21428571428571427)
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_weeks(&self) -> Option<f64> {
        self.as_milliseconds()
            .map(|v| v * 10.0f64.powf(-3.0) / 60.0 / 60.0 / 24.0 / 7.0)
    }

    /// Try to return a value as a duration in months according to
    /// [duration format](https://github.com/lightbend/config/blob/master/HOCON.md#duration-format).
    ///
    /// Bare numbers are taken to be in bytes already, while strings are parsed as a number
    /// plus an optional unit string.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use hocon::{Hocon, HoconLoader, Error};
    /// # fn main() -> Result<(), failure::Error> {
    /// assert_eq!(
    ///     HoconLoader::new().load_str(r#"{ duration = 1.5 days  }"#)?
    ///         .hocon()?["duration"].as_months(),
    ///     Some(0.05)
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_months(&self) -> Option<f64> {
        self.as_milliseconds()
            .map(|v| v * 10.0f64.powf(-3.0) / 60.0 / 60.0 / 24.0 / 30.0)
    }

    /// Try to return a value as a duration in years according to
    /// [duration format](https://github.com/lightbend/config/blob/master/HOCON.md#duration-format).
    ///
    /// Bare numbers are taken to be in bytes already, while strings are parsed as a number
    /// plus an optional unit string.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use hocon::{Hocon, HoconLoader, Error};
    /// # fn main() -> Result<(), failure::Error> {
    /// assert_eq!(
    ///     HoconLoader::new().load_str(r#"{ duration = 1.5 days  }"#)?
    ///         .hocon()?["duration"].as_years(),
    ///     Some(0.00410958904109589)
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_years(&self) -> Option<f64> {
        self.as_milliseconds()
            .map(|v| v * 10.0f64.powf(-3.0) / 60.0 / 60.0 / 24.0 / 365.0)
    }

    /// Try to return a value as a duration according to
    /// [duration format](https://github.com/lightbend/config/blob/master/HOCON.md#duration-format).
    ///
    /// Bare numbers are taken to be in bytes already, while strings are parsed as a number
    /// plus an optional unit string.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use hocon::{Hocon, HoconLoader, Error};
    /// # fn main() -> Result<(), failure::Error> {
    /// assert_eq!(
    ///     HoconLoader::new().load_str(r#"{ duration = 1.5 hours  }"#)?
    ///         .hocon()?["duration"].as_duration(),
    ///     Some(std::time::Duration::from_secs(5400))
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_duration(&self) -> Option<std::time::Duration> {
        self.as_nanoseconds()
            .map(|v| std::time::Duration::from_nanos(v as u64))
    }
}

impl Hocon {
    /// Deserialize the loaded documents to the target type
    ///
    /// # Errors
    ///
    /// * [`Error::Deserialization`](enum.Error.html#variant.Deserialization) if there was a
    /// serde error during deserialization (missing required field, type issue, ...)
    ///
    /// # Additional errors in strict mode
    ///
    /// * [`Error::Include`](enum.Error.html#variant.Include) if there was an issue with an
    /// included file
    /// * [`Error::KeyNotFound`](enum.Error.html#variant.KeyNotFound) if there is a substitution
    /// with a key that is not present in the document
    /// * [`Error::DisabledExternalUrl`](enum.Error.html#variant.DisabledExternalUrl) if crate
    /// was built without feature `url-support` and an `include url("...")` was found
    #[cfg(feature = "serde-support")]
    pub fn resolve<'de, T>(self) -> Result<T, crate::Error>
    where
        T: ::serde::Deserialize<'de>,
    {
        Ok(
            crate::serde::from_hocon(self).map_err(|err| crate::Error::Deserialization {
                message: err.message,
            })?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_on_string() {
        let val = Hocon::String(String::from("test"));

        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_f64(), None);
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), Some(String::from("test")));
        assert_eq!(val[0], INVALID_KEY);
        assert_eq!(val["a"], INVALID_KEY);
    }

    #[test]
    fn access_on_real() {
        let val = Hocon::Real(5.6);

        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_f64(), Some(5.6));
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), Some(String::from("5.6")));
        assert_eq!(val[0], INVALID_KEY);
        assert_eq!(val["a"], INVALID_KEY);
    }

    #[test]
    fn access_on_integer() {
        let val = Hocon::Integer(5);

        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_f64(), Some(5.0));
        assert_eq!(val.as_i64(), Some(5));
        assert_eq!(val.as_string(), Some(String::from("5")));
        assert_eq!(val[0], INVALID_KEY);
        assert_eq!(val["a"], INVALID_KEY);
    }

    #[test]
    fn access_on_boolean_false() {
        let val = Hocon::Boolean(false);

        assert_eq!(val.as_bool(), Some(false));
        assert_eq!(val.as_f64(), None);
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), Some(String::from("false")));
        assert_eq!(val[0], INVALID_KEY);
        assert_eq!(val["a"], INVALID_KEY);
    }

    #[test]
    fn access_on_boolean_true() {
        let val = Hocon::Boolean(true);

        assert_eq!(val.as_bool(), Some(true));
        assert_eq!(val.as_f64(), None);
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), Some(String::from("true")));
        assert_eq!(val[0], INVALID_KEY);
        assert_eq!(val["a"], INVALID_KEY);
    }

    #[test]
    fn access_on_null() {
        let val = Hocon::Null;

        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_f64(), None);
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), None);
        assert_eq!(val[0], INVALID_KEY);
        assert_eq!(val["a"], INVALID_KEY);
    }

    #[test]
    fn access_on_bad_value() {
        let val = Hocon::BadValue(crate::Error::DisabledExternalUrl);

        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_f64(), None);
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), None);
        assert_eq!(val[0], INVALID_KEY);
        assert_eq!(val["a"], INVALID_KEY);
    }

    #[test]
    fn access_on_array() {
        let val = Hocon::Array(vec![Hocon::Integer(5), Hocon::Integer(6)]);

        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_f64(), None);
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), None);
        assert_eq!(val[0], Hocon::Integer(5));
        assert_eq!(val[1], Hocon::Integer(6));
        assert_eq!(val[2], NOT_FOUND);
        assert_eq!(val["a"], INVALID_KEY);
    }

    #[test]
    fn access_on_hash() {
        let mut hm = HashMap::new();
        hm.insert(String::from("a"), Hocon::Integer(5));
        hm.insert(String::from("b"), Hocon::Integer(6));
        let val = Hocon::Hash(hm);

        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_f64(), None);
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), None);
        assert_eq!(val[0], INVALID_KEY);
        assert_eq!(val["a"], Hocon::Integer(5));
        assert_eq!(val["b"], Hocon::Integer(6));
        assert_eq!(val["c"], NOT_FOUND);
    }

    #[test]
    fn cast_string() {
        assert_eq!(Hocon::String(String::from("true")).as_bool(), Some(true));
        assert_eq!(Hocon::String(String::from("yes")).as_bool(), Some(true));
        assert_eq!(Hocon::String(String::from("on")).as_bool(), Some(true));
        assert_eq!(Hocon::String(String::from("false")).as_bool(), Some(false));
        assert_eq!(Hocon::String(String::from("no")).as_bool(), Some(false));
        assert_eq!(Hocon::String(String::from("off")).as_bool(), Some(false));

        assert_eq!(Hocon::String(String::from("5.6")).as_f64(), Some(5.6));
        assert_eq!(Hocon::String(String::from("5.6")).as_i64(), None);
        assert_eq!(Hocon::String(String::from("5")).as_f64(), Some(5.0));
        assert_eq!(Hocon::String(String::from("5")).as_i64(), Some(5));
    }

    #[test]
    fn access_hash_as_array() {
        let mut hm = HashMap::new();
        hm.insert(String::from("0"), Hocon::Integer(5));
        hm.insert(String::from("a"), Hocon::Integer(6));
        hm.insert(String::from("2"), Hocon::Integer(7));
        let val = Hocon::Hash(hm);

        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_f64(), None);
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), None);
        assert_eq!(val[0], Hocon::Integer(5));
        assert_eq!(val[1], Hocon::Integer(7));
        assert_eq!(val[2], INVALID_KEY);
        assert_eq!(val["0"], Hocon::Integer(5));
        assert_eq!(val["a"], Hocon::Integer(6));
        assert_eq!(val["2"], Hocon::Integer(7));
    }

    #[test]
    fn access_on_bytes() {
        let val = Hocon::Array(vec![
            Hocon::Integer(5),
            Hocon::Real(6.5),
            Hocon::String(String::from("7")),
            Hocon::String(String::from("8kB")),
            Hocon::String(String::from("9 EB")),
            Hocon::String(String::from("10.5MiB")),
            Hocon::String(String::from("5unit")),
            Hocon::Boolean(false),
        ]);

        assert_eq!(val[0].as_bytes(), Some(5.0));
        assert_eq!(val[1].as_bytes(), Some(6.5));
        assert_eq!(val[2].as_bytes(), Some(7.0));
        assert_eq!(val[3].as_bytes(), Some(8.0 * 1_000.0));
        assert_eq!(val[4].as_bytes(), Some(9.0 * 10.0f64.powf(18.0)));
        assert_eq!(val[5].as_bytes(), Some(10.5 * 2.0f64.powf(20.0)));
        assert_eq!(val[6].as_bytes(), None);
        assert_eq!(val[7].as_bytes(), None);
    }

    #[test]
    fn access_on_bytes_all_bytes_units() {
        for unit in vec!["B", "b", "byte", "bytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].as_bytes(), Some(8.0));
        }

        for unit in vec!["kB", "kilobyte", "kilobytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].as_bytes(), Some(8.0 * 10.0f64.powf(3.0)));
        }
        for unit in vec!["MB", "megabyte", "megabytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].as_bytes(), Some(8.0 * 10.0f64.powf(6.0)));
        }
        for unit in vec!["GB", "gigabyte", "gigabytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].as_bytes(), Some(8.0 * 10.0f64.powf(9.0)));
        }
        for unit in vec!["TB", "terabyte", "terabytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].as_bytes(), Some(8.0 * 10.0f64.powf(12.0)));
        }
        for unit in vec!["PB", "petabyte", "petabytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].as_bytes(), Some(8.0 * 10.0f64.powf(15.0)));
        }
        for unit in vec!["EB", "exabyte", "exabytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].as_bytes(), Some(8.0 * 10.0f64.powf(18.0)));
        }
        for unit in vec!["ZB", "zettabyte", "zettabytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].as_bytes(), Some(8.0 * 10.0f64.powf(21.0)));
        }
        for unit in vec!["YB", "yottabyte", "yottabytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].as_bytes(), Some(8.0 * 10.0f64.powf(24.0)));
        }

        for unit in vec!["K", "k", "Ki", "KiB", "kibibyte", "kibibytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].as_bytes(), Some(8.0 * 2.0f64.powf(10.0)));
        }
        for unit in vec!["M", "m", "Mi", "MiB", "mebibyte", "mebibytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].as_bytes(), Some(8.0 * 2.0f64.powf(20.0)));
        }
        for unit in vec!["G", "g", "Gi", "GiB", "gibibyte", "gibibytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].as_bytes(), Some(8.0 * 2.0f64.powf(30.0)));
        }
        for unit in vec!["T", "t", "Ti", "TiB", "tebibyte", "tebibytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].as_bytes(), Some(8.0 * 2.0f64.powf(40.0)));
        }
        for unit in vec!["P", "p", "Pi", "PiB", "pebibyte", "pebibytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].as_bytes(), Some(8.0 * 2.0f64.powf(50.0)));
        }
        for unit in vec!["E", "e", "Ei", "EiB", "exbibyte", "exbibytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].as_bytes(), Some(8.0 * 2.0f64.powf(60.0)));
        }
        for unit in vec!["Z", "z", "Zi", "ZiB", "zebibyte", "zebibytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].as_bytes(), Some(8.0 * 2.0f64.powf(70.0)));
        }
        for unit in vec!["Y", "y", "Yi", "YiB", "yobibyte", "yobibytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].as_bytes(), Some(8.0 * 2.0f64.powf(80.0)));
        }
    }

    #[test]
    fn access_on_duration() {
        let mut hm = HashMap::new();
        hm.insert(String::from("ns"), Hocon::String(String::from("1ns")));
        hm.insert(String::from("us"), Hocon::String(String::from("1us")));
        hm.insert(String::from("ms"), Hocon::String(String::from("1ms")));
        hm.insert(String::from("s"), Hocon::String(String::from("1s")));
        hm.insert(String::from("m"), Hocon::String(String::from("1m")));
        hm.insert(String::from("h"), Hocon::String(String::from("1h")));
        hm.insert(String::from("d"), Hocon::String(String::from("1d")));
        hm.insert(String::from("w"), Hocon::String(String::from("1w")));
        hm.insert(String::from("mo"), Hocon::String(String::from("1mo")));
        hm.insert(String::from("y"), Hocon::String(String::from("1y")));
        let val = Hocon::Hash(hm);

        assert_eq!(val["ns"].as_nanoseconds(), Some(1.0));
        assert_eq!(
            val["ns"].as_duration(),
            Some(std::time::Duration::from_nanos(1))
        );
        assert_eq!(val["us"].as_microseconds(), Some(1.0));
        assert_eq!(
            val["us"].as_duration(),
            Some(std::time::Duration::from_micros(1))
        );
        assert_eq!(val["ms"].as_milliseconds(), Some(1.0));
        assert_eq!(
            val["ms"].as_duration(),
            Some(std::time::Duration::from_millis(1))
        );
        assert_eq!(val["s"].as_seconds(), Some(1.0));
        assert_eq!(
            val["s"].as_duration(),
            Some(std::time::Duration::from_secs(1))
        );
        assert_eq!(val["m"].as_minutes(), Some(1.0));
        assert_eq!(
            val["m"].as_duration(),
            Some(std::time::Duration::from_secs(60))
        );
        assert_eq!(val["h"].as_hours(), Some(1.0));
        assert_eq!(
            val["h"].as_duration(),
            Some(std::time::Duration::from_secs(60 * 60))
        );
        assert_eq!(val["d"].as_days(), Some(1.0));
        assert_eq!(
            val["d"].as_duration(),
            Some(std::time::Duration::from_secs(60 * 60 * 24))
        );
        assert_eq!(val["w"].as_weeks(), Some(1.0));
        assert_eq!(
            val["w"].as_duration(),
            Some(std::time::Duration::from_secs(60 * 60 * 24 * 7))
        );
        assert_eq!(val["mo"].as_months(), Some(1.0));
        assert_eq!(
            val["mo"].as_duration(),
            Some(std::time::Duration::from_secs(60 * 60 * 24 * 30))
        );
        assert_eq!(val["y"].as_years(), Some(1.0));
        assert_eq!(
            val["y"].as_duration(),
            Some(std::time::Duration::from_secs(60 * 60 * 24 * 365))
        );
    }
}
