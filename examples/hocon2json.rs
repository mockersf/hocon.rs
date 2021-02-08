use std::env;

use serde_json::{Number, Value};

use hocon::{Error, Hocon, HoconLoader};

fn hocon_to_json(hocon: Hocon) -> Option<Value> {
    match hocon {
        Hocon::Boolean(b) => Some(Value::Bool(b)),
        Hocon::Integer(i) => Some(Value::Number(Number::from(i))),
        Hocon::Real(f) => Some(Value::Number(
            Number::from_f64(f).unwrap_or(Number::from(0)),
        )),
        Hocon::String(s) => Some(Value::String(s)),
        Hocon::Array(vec) => Some(Value::Array(
            vec.into_iter()
                .map(hocon_to_json)
                .filter_map(|i| i)
                .collect(),
        )),
        Hocon::Hash(map) => Some(Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, hocon_to_json(v)))
                .filter_map(|(k, v)| v.map(|v| (k, v)))
                .collect(),
        )),
        Hocon::Null => Some(Value::Null),
        Hocon::BadValue(_) => None,
    }
}

fn parse_to_json(path: &str) -> Result<String, Error> {
    let hocon = dbg!(HoconLoader::new().no_system().load_file(path)?.hocon())?;
    let json: Option<_> = hocon_to_json(hocon);
    Ok(
        serde_json::to_string_pretty(&json).map_err(|e| Error::Deserialization {
            message: e.to_string(),
        })?,
    )
}

fn main() {
    match env::args().nth(1) {
        None => println!("please provide a HOCON file"),
        Some(file) => println!(
            "{}",
            dbg!(parse_to_json(&file).unwrap_or_else(|_| String::from("")))
        ),
    }
}
