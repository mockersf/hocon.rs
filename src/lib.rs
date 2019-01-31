use std::collections::HashMap;
use std::ops::Index;

mod internals;
mod parser;

#[derive(Debug, Clone)]
pub enum Hocon {
    Real(f64),
    Integer(i64),
    String(std::string::String),
    Boolean(bool),
    Array(Vec<Hocon>),
    Hash(HashMap<String, Hocon>),
    BadValue,
}

impl Hocon {
    pub fn load_from_str(s: &str) -> Result<Hocon, ()> {
        parser::wrapper(format!("{}\0", s).as_bytes())
            .map(|success| success.1)
            .map_err(|_err| ())
            .and_then(|hocon| hocon.merge())
            .map(|intermediate| intermediate.finalize())
    }
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
            _ => &BAD_VALUE,
        }
    }
}

impl Hocon {
    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Hocon::Real(ref v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Hocon::Integer(ref v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Hocon::String(ref v) => Some(v),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Hocon::Boolean(ref v) => Some(*v),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_string() {
        let s = r#"{"a":"dndjf"}"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"].as_str().unwrap(), "dndjf");
    }

    #[test]
    fn parse_int() {
        let s = r#"{"a":5}"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"].as_i64().unwrap(), 5);
    }

    #[test]
    fn parse_float() {
        let s = r#"{"a":5.7}"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"].as_f64().unwrap(), 5.7);
    }

    #[test]
    fn parse_bool() {
        let s = r#"{"a":true}"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"].as_bool().unwrap(), true);
    }

    #[test]
    fn parse_int_array() {
        let s = r#"{"a":[5, 6, 7]}"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"][1].as_i64().unwrap(), 6);
        assert_eq!(doc["a"][2].as_i64().unwrap(), 7);
    }

    //     #[test]
    //     fn parse_int_array_newline_as_separator() {
    //         let s = r#"{"a":[5
    // 6
    // 7]}"#;
    //         let doc = Hocon::load_from_str(s).unwrap();

    //         assert_eq!(doc["a"][1].as_i64().unwrap(), 6);
    //         assert_eq!(doc["a"][2].as_i64().unwrap(), 7);
    //     }

    #[test]
    fn parse_int_array_trailing_comma() {
        let s = r#"{"a":[5, 6, 7,]}"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"][1].as_i64().unwrap(), 6);
    }

    #[test]
    fn parse_nested() {
        let s = r#"{"a":{"b":[{"c":5},{"c":6}]}}"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"]["b"][1]["c"].as_i64().unwrap(), 6);
    }

    #[test]
    fn parse_newlines() {
        let s = r#"{"a":
    5
    }"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"].as_i64().unwrap(), 5);
    }

    //     #[test]
    //     fn parse_comment() {
    //         let s = r#"{"a":5//value
    // }"#;
    //         let doc = Hocon::load_from_str(s).unwrap();

    //         assert_eq!(doc["a"].as_i64().unwrap(), 5);
    //     }

    #[test]
    fn parse_keyvalue_separator() {
        let s = r#"{"a":5,"b"=6,"c" {"a":1}}}"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["a"].as_i64().unwrap(), 5);
        assert_eq!(doc["b"].as_i64().unwrap(), 6);
        assert_eq!(doc["c"]["a"].as_i64().unwrap(), 1);
    }

    #[test]
    fn parse_object_merging() {
        let s = r#"{
        "foo" : { "a" : 42 },
        "foo" : { "b" : 43 }
    }"#;
        let doc = Hocon::load_from_str(s).unwrap();

        assert_eq!(doc["foo"]["a"].as_i64().unwrap(), 42);
        assert_eq!(doc["foo"]["b"].as_i64().unwrap(), 43);
    }

}
