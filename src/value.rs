use std::collections::HashMap;
use std::ops::Index;

/// HOCON document
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
    /// A `BadValue`, marking an error in parsing or an missing value
    BadValue,
}

static BAD_VALUE: Hocon = Hocon::BadValue;

impl<'a> Index<&'a str> for Hocon {
    type Output = Hocon;

    fn index(&self, idx: &'a str) -> &Self::Output {
        match self {
            Hocon::Hash(hash) => hash.get(idx).unwrap_or(&BAD_VALUE),
            _ => &BAD_VALUE,
        }
    }
}
impl Index<usize> for Hocon {
    type Output = Hocon;

    fn index(&self, idx: usize) -> &Self::Output {
        match self {
            Hocon::Array(vec) => vec.get(idx).unwrap_or(&BAD_VALUE),
            Hocon::Hash(hash) => {
                let mut keys_as_usize = hash
                    .keys()
                    .filter_map(|k| k.parse::<usize>().ok().map(|v| (k, v)))
                    .collect::<Vec<_>>();
                keys_as_usize.sort_by(|(_, v0), (_, v1)| v0.cmp(v1));
                keys_as_usize
                    .get(idx)
                    .and_then(|(k, _)| hash.get(*k))
                    .unwrap_or(&BAD_VALUE)
            }
            _ => &BAD_VALUE,
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

    /// Try to return a value as a size in bytes
    ///
    /// https://github.com/lightbend/config/blob/master/HOCON.md#size-in-bytes-format
    ///
    /// Bare numbers are taken to be in bytes already, while strings are parsed as a number
    /// plus an optional unit string.
    pub fn get_bytes(&self) -> Option<f64> {
        match *self {
            Hocon::Integer(ref i) => Some(*i as f64),
            Hocon::Real(ref f) => Some(*f),
            Hocon::String(ref s) => {
                match unit_format::value_and_unit(s).map(|(value, unit)| (value, unit.trim())) {
                    Some((value, ""))
                    | Some((value, "B"))
                    | Some((value, "b"))
                    | Some((value, "byte"))
                    | Some((value, "bytes")) => Some(value),
                    Some((value, "kB"))
                    | Some((value, "kilobyte"))
                    | Some((value, "kilobytes")) => Some(value * 10.0f64.powf(3.0)),
                    Some((value, "MB"))
                    | Some((value, "megabyte"))
                    | Some((value, "megabytes")) => Some(value * 10.0f64.powf(6.0)),
                    Some((value, "GB"))
                    | Some((value, "gigabyte"))
                    | Some((value, "gigabytes")) => Some(value * 10.0f64.powf(9.0)),
                    Some((value, "TB"))
                    | Some((value, "terabyte"))
                    | Some((value, "terabytes")) => Some(value * 10.0f64.powf(12.0)),
                    Some((value, "PB"))
                    | Some((value, "petabyte"))
                    | Some((value, "petabytes")) => Some(value * 10.0f64.powf(15.0)),
                    Some((value, "EB")) | Some((value, "exabyte")) | Some((value, "exabytes")) => {
                        Some(value * 10.0f64.powf(18.0))
                    }
                    Some((value, "ZB"))
                    | Some((value, "zettabyte"))
                    | Some((value, "zettabytes")) => Some(value * 10.0f64.powf(21.0)),
                    Some((value, "YB"))
                    | Some((value, "yottabyte"))
                    | Some((value, "yottabytes")) => Some(value * 10.0f64.powf(24.0)),
                    Some((value, "K"))
                    | Some((value, "k"))
                    | Some((value, "Ki"))
                    | Some((value, "KiB"))
                    | Some((value, "kibibyte"))
                    | Some((value, "kibibytes")) => Some(value * 2.0f64.powf(10.0)),
                    Some((value, "M"))
                    | Some((value, "m"))
                    | Some((value, "Mi"))
                    | Some((value, "MiB"))
                    | Some((value, "mebibyte"))
                    | Some((value, "mebibytes")) => Some(value * 2.0f64.powf(20.0)),
                    Some((value, "G"))
                    | Some((value, "g"))
                    | Some((value, "Gi"))
                    | Some((value, "GiB"))
                    | Some((value, "gibibyte"))
                    | Some((value, "gibibytes")) => Some(value * 2.0f64.powf(30.0)),
                    Some((value, "T"))
                    | Some((value, "t"))
                    | Some((value, "Ti"))
                    | Some((value, "TiB"))
                    | Some((value, "tebibyte"))
                    | Some((value, "tebibytes")) => Some(value * 2.0f64.powf(40.0)),
                    Some((value, "P"))
                    | Some((value, "p"))
                    | Some((value, "Pi"))
                    | Some((value, "PiB"))
                    | Some((value, "pebibyte"))
                    | Some((value, "pebibytes")) => Some(value * 2.0f64.powf(50.0)),
                    Some((value, "E"))
                    | Some((value, "e"))
                    | Some((value, "Ei"))
                    | Some((value, "EiB"))
                    | Some((value, "exbibyte"))
                    | Some((value, "exbibytes")) => Some(value * 2.0f64.powf(60.0)),
                    Some((value, "Z"))
                    | Some((value, "z"))
                    | Some((value, "Zi"))
                    | Some((value, "ZiB"))
                    | Some((value, "zebibyte"))
                    | Some((value, "zebibytes")) => Some(value * 2.0f64.powf(70.0)),
                    Some((value, "Y"))
                    | Some((value, "y"))
                    | Some((value, "Yi"))
                    | Some((value, "YiB"))
                    | Some((value, "yobibyte"))
                    | Some((value, "yobibytes")) => Some(value * 2.0f64.powf(80.0)),
                    _ => None,
                }
            }
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
        assert_eq!(val[0], Hocon::BadValue);
        assert_eq!(val["a"], Hocon::BadValue);
    }

    #[test]
    fn access_on_real() {
        let val = Hocon::Real(5.6);

        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_f64(), Some(5.6));
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), Some(String::from("5.6")));
        assert_eq!(val[0], Hocon::BadValue);
        assert_eq!(val["a"], Hocon::BadValue);
    }

    #[test]
    fn access_on_integer() {
        let val = Hocon::Integer(5);

        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_f64(), Some(5.0));
        assert_eq!(val.as_i64(), Some(5));
        assert_eq!(val.as_string(), Some(String::from("5")));
        assert_eq!(val[0], Hocon::BadValue);
        assert_eq!(val["a"], Hocon::BadValue);
    }

    #[test]
    fn access_on_boolean_false() {
        let val = Hocon::Boolean(false);

        assert_eq!(val.as_bool(), Some(false));
        assert_eq!(val.as_f64(), None);
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), Some(String::from("false")));
        assert_eq!(val[0], Hocon::BadValue);
        assert_eq!(val["a"], Hocon::BadValue);
    }

    #[test]
    fn access_on_boolean_true() {
        let val = Hocon::Boolean(true);

        assert_eq!(val.as_bool(), Some(true));
        assert_eq!(val.as_f64(), None);
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), Some(String::from("true")));
        assert_eq!(val[0], Hocon::BadValue);
        assert_eq!(val["a"], Hocon::BadValue);
    }

    #[test]
    fn access_on_null() {
        let val = Hocon::Null;

        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_f64(), None);
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), None);
        assert_eq!(val[0], Hocon::BadValue);
        assert_eq!(val["a"], Hocon::BadValue);
    }

    #[test]
    fn access_on_bad_value() {
        let val = Hocon::BadValue;

        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_f64(), None);
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_string(), None);
        assert_eq!(val[0], Hocon::BadValue);
        assert_eq!(val["a"], Hocon::BadValue);
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
        assert_eq!(val[2], Hocon::BadValue);
        assert_eq!(val["a"], Hocon::BadValue);
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
        assert_eq!(val[0], Hocon::BadValue);
        assert_eq!(val["a"], Hocon::Integer(5));
        assert_eq!(val["b"], Hocon::Integer(6));
        assert_eq!(val["c"], Hocon::BadValue);
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
        assert_eq!(val[2], Hocon::BadValue);
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
        ]);

        assert_eq!(val[0].get_bytes(), Some(5.0));
        assert_eq!(val[1].get_bytes(), Some(6.5));
        assert_eq!(val[2].get_bytes(), Some(7.0));
        assert_eq!(val[3].get_bytes(), Some(8.0 * 1_000.0));
        assert_eq!(val[4].get_bytes(), Some(9.0 * 10.0f64.powf(18.0)));
        assert_eq!(val[5].get_bytes(), Some(10.5 * 2.0f64.powf(20.0)));
    }

    #[test]
    fn access_on_bytes_all_bytes_units() {
        for unit in vec!["B", "b", "byte", "bytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].get_bytes(), Some(8.0));
        }

        for unit in vec!["kB", "kilobyte", "kilobytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].get_bytes(), Some(8.0 * 10.0f64.powf(3.0)));
        }
        for unit in vec!["MB", "megabyte", "megabytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].get_bytes(), Some(8.0 * 10.0f64.powf(6.0)));
        }
        for unit in vec!["GB", "gigabyte", "gigabytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].get_bytes(), Some(8.0 * 10.0f64.powf(9.0)));
        }
        for unit in vec!["TB", "terabyte", "terabytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].get_bytes(), Some(8.0 * 10.0f64.powf(12.0)));
        }
        for unit in vec!["PB", "petabyte", "petabytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].get_bytes(), Some(8.0 * 10.0f64.powf(15.0)));
        }
        for unit in vec!["EB", "exabyte", "exabytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].get_bytes(), Some(8.0 * 10.0f64.powf(18.0)));
        }
        for unit in vec!["ZB", "zettabyte", "zettabytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].get_bytes(), Some(8.0 * 10.0f64.powf(21.0)));
        }
        for unit in vec!["YB", "yottabyte", "yottabytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].get_bytes(), Some(8.0 * 10.0f64.powf(24.0)));
        }

        for unit in vec!["K", "k", "Ki", "KiB", "kibibyte", "kibibytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].get_bytes(), Some(8.0 * 2.0f64.powf(10.0)));
        }
        for unit in vec!["M", "m", "Mi", "MiB", "mebibyte", "mebibytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].get_bytes(), Some(8.0 * 2.0f64.powf(20.0)));
        }
        for unit in vec!["G", "g", "Gi", "GiB", "gibibyte", "gibibytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].get_bytes(), Some(8.0 * 2.0f64.powf(30.0)));
        }
        for unit in vec!["T", "t", "Ti", "TiB", "tebibyte", "tebibytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].get_bytes(), Some(8.0 * 2.0f64.powf(40.0)));
        }
        for unit in vec!["P", "p", "Pi", "PiB", "pebibyte", "pebibytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].get_bytes(), Some(8.0 * 2.0f64.powf(50.0)));
        }
        for unit in vec!["E", "e", "Ei", "EiB", "exbibyte", "exbibytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].get_bytes(), Some(8.0 * 2.0f64.powf(60.0)));
        }
        for unit in vec!["Z", "z", "Zi", "ZiB", "zebibyte", "zebibytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].get_bytes(), Some(8.0 * 2.0f64.powf(70.0)));
        }
        for unit in vec!["Y", "y", "Yi", "YiB", "yobibyte", "yobibytes"] {
            let val = Hocon::Array(vec![Hocon::String(format!("8{}", unit))]);
            assert_eq!(dbg!(val)[0].get_bytes(), Some(8.0 * 2.0f64.powf(80.0)));
        }
    }

}
