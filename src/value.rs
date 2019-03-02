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
                    .into_iter()
                    .filter_map(|k| k.parse::<usize>().ok().map(|v| (k, v)))
                    .collect::<Vec<_>>();
                keys_as_usize.sort_by(|(_, v0), (_, v1)| v0.cmp(v1));
                keys_as_usize
                    .get(idx)
                    .and_then(|(k, _)| hash.get(k.clone()))
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
}
