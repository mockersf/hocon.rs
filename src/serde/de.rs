//! Deserializer methods using serde

use super::error::{Error, Result};
use crate::Hocon;

pub use super::wrappers;

macro_rules! impl_deserialize_n {
    ($method:ident, $visit:ident) => {
        fn $method<V>(self, visitor: V) -> Result<V::Value>
        where
            V: serde::de::Visitor<'de>,
        {
            visitor.$visit({
                let value = self
                    .read
                    .get_attribute_value(&self.current_field)
                    .ok_or_else(|| Error {
                        message: format!("missing integer for field \"{}\"", self.current_field),
                    })?
                    .clone();
                value
                    .as_i64()
                    .or_else(|| value.as_bytes().map(|v| v as i64))
                    .ok_or_else(|| Error {
                        message: format!(
                            "Invalid type for field \"{}\", expected integer",
                            self.current_field
                        ),
                    })?
            })
        }
    };
    ($type:ty, $method:ident, $visit:ident) => {
        fn $method<V>(self, visitor: V) -> Result<V::Value>
        where
            V: serde::de::Visitor<'de>,
        {
            visitor.$visit({
                let value = self
                    .read
                    .get_attribute_value(&self.current_field)
                    .ok_or_else(|| Error {
                        message: format!("missing integer for field \"{}\"", self.current_field),
                    })?
                    .clone();
                value
                    .as_i64()
                    .or_else(|| value.as_bytes().map(|v| v as i64))
                    .ok_or_else(|| Error {
                        message: format!(
                            "Invalid type for field \"{}\", expected integer",
                            self.current_field
                        ),
                    })? as $type
            })
        }
    };
}
macro_rules! impl_deserialize_f {
    ($method:ident, $visit:ident) => {
        fn $method<V>(self, visitor: V) -> Result<V::Value>
        where
            V: serde::de::Visitor<'de>,
        {
            visitor.$visit({
                let value = self
                    .read
                    .get_attribute_value(&self.current_field)
                    .ok_or_else(|| Error {
                        message: format!("missing float for field \"{}\"", self.current_field),
                    })?
                    .clone();
                value
                    .as_f64()
                    .or_else(|| value.as_bytes().map(|v| v as f64))
                    .ok_or_else(|| Error {
                        message: format!(
                            "Invalid type for field \"{}\", expected float",
                            self.current_field
                        ),
                    })?
            })
        }
    };
    ($type:ty, $method:ident, $visit:ident) => {
        fn $method<V>(self, visitor: V) -> Result<V::Value>
        where
            V: serde::de::Visitor<'de>,
        {
            visitor.$visit({
                let value = self
                    .read
                    .get_attribute_value(&self.current_field)
                    .ok_or_else(|| Error {
                        message: format!("missing float for field \"{}\"", self.current_field),
                    })?
                    .clone();
                value
                    .as_f64()
                    .or_else(|| value.as_bytes().map(|v| v as f64))
                    .ok_or_else(|| Error {
                        message: format!(
                            "Invalid type for field \"{}\", expected float",
                            self.current_field
                        ),
                    })? as $type
            })
        }
    };
}

#[derive(Debug)]
enum Index {
    String(String),
    Number(usize),
    None,
}

impl std::fmt::Display for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Index::String(field) => write!(f, "{}", field),
            Index::Number(field) => write!(f, "{}", field),
            Index::None => write!(f, ""),
        }
    }
}

trait Read {
    fn get_attribute_value(&self, index: &Index) -> Option<&Hocon>;
    fn get_keys(&self) -> Vec<String>;
}

struct HoconRead {
    hocon: Hocon,
}
impl HoconRead {
    fn new(hocon: Hocon) -> Self {
        HoconRead { hocon }
    }
}
impl Read for HoconRead {
    fn get_attribute_value(&self, index: &Index) -> Option<&Hocon> {
        match *index {
            Index::String(ref key) => match &self.hocon[key.as_ref()] {
                Hocon::BadValue(_) => None,
                v => Some(v),
            },
            Index::Number(key) => match &self.hocon[key] {
                Hocon::BadValue(_) => None,
                v => Some(v),
            },
            _ => None,
        }
    }

    fn get_keys(&self) -> Vec<String> {
        match &self.hocon {
            Hocon::Hash(map) => map.keys().cloned().collect(),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
struct Deserializer<R> {
    read: R,
    current_field: Index,
    as_key: bool,
}
impl<'de, R> Deserializer<R>
where
    R: Read,
{
    pub fn new(read: R) -> Self {
        Deserializer {
            read,
            current_field: Index::None,
            as_key: false,
        }
    }
}

impl<'de, 'a, R: Read> serde::de::Deserializer<'de> for &'a mut Deserializer<R> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.as_key {
            self.deserialize_identifier(visitor)
        } else {
            let f: Hocon = self
                .read
                .get_attribute_value(&self.current_field)
                .ok_or_else(|| Error {
                    message: format!("missing value for field \"{}\"", self.current_field),
                })?
                .clone();
            match f {
                Hocon::Boolean(_) => self.deserialize_bool(visitor),
                Hocon::Real(_) => self.deserialize_f64(visitor),
                Hocon::Integer(_) => self.deserialize_i64(visitor),
                Hocon::String(_) => self.deserialize_string(visitor),
                Hocon::Array(_) => self.deserialize_seq(visitor),
                Hocon::Hash(_) => self.deserialize_map(visitor),
                Hocon::Null => self.deserialize_option(visitor),
                Hocon::BadValue(err) => Err(Error {
                    message: format!("error for field \"{}\": {}", self.current_field, err),
                }),
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_bool(
            self.read
                .get_attribute_value(&self.current_field)
                .ok_or_else(|| Error {
                    message: format!("Missing field \"{}\"", self.current_field),
                })?
                .clone()
                .as_bool()
                .ok_or_else(|| Error {
                    message: format!(
                        "Invalid type for field \"{}\", expected bool",
                        self.current_field
                    ),
                })?,
        )
    }

    impl_deserialize_n!(i8, deserialize_i8, visit_i8);
    impl_deserialize_n!(i16, deserialize_i16, visit_i16);
    impl_deserialize_n!(i32, deserialize_i32, visit_i32);
    impl_deserialize_n!(deserialize_i64, visit_i64);
    // impl_deserialize_n!(i64, deserialize_i64, visit_i64);

    impl_deserialize_n!(u8, deserialize_u8, visit_u8);
    impl_deserialize_n!(u16, deserialize_u16, visit_u16);
    impl_deserialize_n!(u32, deserialize_u32, visit_u32);
    impl_deserialize_n!(u64, deserialize_u64, visit_u64);

    impl_deserialize_f!(f32, deserialize_f32, visit_f32);
    impl_deserialize_f!(deserialize_f64, visit_f64);
    // impl_deserialize_f!(f64, deserialize_f64, visit_f64);

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_char(
            self.read
                .get_attribute_value(&self.current_field)
                .ok_or_else(|| Error {
                    message: format!("missing char for field \"{}\"", self.current_field),
                })?
                .clone()
                .as_string()
                .ok_or_else(|| Error {
                    message: format!("missing char for field \"{}\"", self.current_field),
                })?
                .parse::<char>()
                .map_err(|_| Error {
                    message: format!("Expected char type for field \"{}\"", self.current_field),
                })?,
        )
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.as_key {
            match &self.current_field {
                Index::String(ref key) => visitor.visit_str(key),
                _ => visitor.visit_str(""),
            }
        } else if let Some(field) = self.read.get_attribute_value(&self.current_field) {
            field
                .clone()
                .as_string()
                .ok_or_else(|| Error {
                    message: format!("missing string for field \"{}\"", self.current_field),
                })
                .and_then(|string_field| visitor.visit_str(&string_field))
        } else {
            visitor.visit_str("")
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.read.get_attribute_value(&self.current_field).is_none() {
            return visitor.visit_none();
        }
        match self
            .read
            .get_attribute_value(&self.current_field)
            .ok_or_else(|| Error {
                message: format!("missing option for field \"{}\"", self.current_field),
            })? {
            Hocon::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self
            .read
            .get_attribute_value(&self.current_field)
            .ok_or_else(|| Error {
                message: format!("missing option for field \"{}\"", self.current_field),
            })? {
            Hocon::Null => visitor.visit_unit(),
            _ => visitor.visit_unit(),
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let list = self
            .read
            .get_attribute_value(&self.current_field)
            .ok_or_else(|| Error {
                message: format!("missing sequence for field \"{}\"", self.current_field),
            })?
            .clone();
        let read = match list {
            Hocon::Array(_) | Hocon::Hash(_) => HoconRead { hocon: list },
            _ => {
                return Err(Error {
                    message: format!(
                        "No sequence input found for field \"{}\"",
                        self.current_field
                    ),
                });
            }
        };
        let mut des = Deserializer::new(read);
        visitor.visit_seq(SeqAccess::new(&mut des))
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let list = self
            .read
            .get_attribute_value(&self.current_field)
            .ok_or_else(|| Error {
                message: format!("missing sequence for field \"{}\"", &self.current_field),
            })?
            .clone();
        let read = match list {
            Hocon::Array(_) | Hocon::Hash(_) => HoconRead { hocon: list },
            _ => {
                return Err(Error {
                    message: format!(
                        "No sequence input found for field \"{}\"",
                        self.current_field
                    ),
                });
            }
        };
        let mut des = Deserializer::new(read);
        visitor.visit_seq(SeqAccess::new(&mut des))
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.current_field {
            Index::None => visitor.visit_map(MapAccess::new(self, self.read.get_keys())),
            _ => {
                let hc = self
                    .read
                    .get_attribute_value(&self.current_field)
                    .ok_or_else(|| Error {
                        message: format!("missing struct for field \"{}\"", self.current_field),
                    })?
                    .clone();
                let keys = match &hc {
                    Hocon::Hash(hm) => hm.keys().cloned().collect(),
                    _ => {
                        return Err(Error {
                            message: format!("invalid type for field \"{}\"", self.current_field),
                        })
                    }
                };
                let mut des = Deserializer::new(HoconRead::new(hc));
                visitor.visit_map(MapAccess::new(&mut des, keys))
            }
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let hc = self
            .read
            .get_attribute_value(&self.current_field)
            .ok_or_else(|| Error {
                message: format!("missing struct for field \"{}\"", self.current_field),
            })?
            .clone();

        if let Index::String(ref s) = self.current_field {
            for v in variants {
                if s == v {
                    let reader = HoconRead::new(hc);
                    let deserializer = &mut Deserializer::new(reader);
                    deserializer.current_field = Index::String(String::from(s));
                    return visitor.visit_enum(UnitVariantAccess::new(deserializer));
                }
            }
        }

        match &hc {
            Hocon::String(name) => {
                let index = Index::String(String::from(name));
                let reader = HoconRead::new(hc);
                let deserializer = &mut Deserializer::new(reader);
                deserializer.current_field = index;
                visitor.visit_enum(UnitVariantAccess::new(deserializer))
            }
            Hocon::Hash(variant_map) => {
                let mut keys = variant_map.keys();
                let first_key = keys.next().ok_or_else(|| Error {
                    message: format!(
                        "non unit enum variant should have enum serialized for field \"{}\"",
                        self.current_field
                    ),
                })?;
                if let Some(_other_key) = keys.next() {
                    return Err(Error {
                        message: format!(
                            "non unit enum variant should have enum serialized for field \"{}\"",
                            self.current_field
                        ),
                    });
                }
                let index = Index::String(String::from(first_key));
                let reader = HoconRead::new(hc);
                let deserializer = &mut Deserializer::new(reader);
                deserializer.current_field = index;
                visitor.visit_enum(VariantAccess::new(deserializer))
            }
            _ => Err(Error {
                message: format!("invalid type for field \"{}\"", self.current_field),
            }),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

struct SeqAccess<'a, R: 'a> {
    de: &'a mut Deserializer<R>,
    current: usize,
}

impl<'a, R: 'a> SeqAccess<'a, R> {
    fn new(de: &'a mut Deserializer<R>) -> Self {
        SeqAccess { de, current: 0 }
    }
}

impl<'de, 'a, R: Read + 'a> serde::de::SeqAccess<'de> for SeqAccess<'a, R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        self.de.current_field = Index::Number(self.current);
        self.current += 1;
        if self
            .de
            .read
            .get_attribute_value(&self.de.current_field)
            .is_none()
        {
            return Ok(None);
        }
        seed.deserialize(&mut *self.de).map(Some)
    }
}

struct MapAccess<'a, R: 'a> {
    de: &'a mut Deserializer<R>,
    keys: Vec<String>,
    current: usize,
}

impl<'a, R: 'a> MapAccess<'a, R> {
    fn new(de: &'a mut Deserializer<R>, keys: Vec<String>) -> Self {
        de.as_key = true;
        MapAccess {
            de,
            keys,
            current: 0,
        }
    }
}

impl<'de, 'a, R: Read + 'a> serde::de::MapAccess<'de> for MapAccess<'a, R> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        if self.current >= self.keys.len() {
            Ok(None)
        } else {
            self.de.current_field = Index::String(self.keys[self.current].to_string());
            self.de.as_key = true;
            self.current += 1;
            seed.deserialize(&mut *self.de).map(Some)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        self.de.as_key = false;
        seed.deserialize(&mut *self.de)
    }
}

struct VariantAccess<'a, R: 'a> {
    de: &'a mut Deserializer<R>,
}

impl<'a, R: 'a> VariantAccess<'a, R> {
    fn new(de: &'a mut Deserializer<R>) -> Self {
        de.as_key = true;
        VariantAccess { de }
    }
}

impl<'de, 'a, R: Read + 'a> serde::de::EnumAccess<'de> for VariantAccess<'a, R> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self)>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let val = seed.deserialize(&mut *self.de)?;
        Ok((val, self))
    }
}

impl<'de, 'a, R: Read + 'a> serde::de::VariantAccess<'de> for VariantAccess<'a, R> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        serde::de::Deserialize::deserialize(self.de)
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        serde::de::Deserializer::deserialize_seq(self.de, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        serde::de::Deserializer::deserialize_struct(self.de, "", fields, visitor)
    }
}

struct UnitVariantAccess<'a, R: 'a> {
    de: &'a mut Deserializer<R>,
}

impl<'a, R: 'a> UnitVariantAccess<'a, R> {
    fn new(de: &'a mut Deserializer<R>) -> Self {
        de.as_key = true;
        UnitVariantAccess { de }
    }
}

impl<'de, 'a, R: Read + 'a> serde::de::EnumAccess<'de> for UnitVariantAccess<'a, R> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self)>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(&mut *self.de)?;
        Ok((variant, self))
    }
}

impl<'de, 'a, R: Read + 'a> serde::de::VariantAccess<'de> for UnitVariantAccess<'a, R> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        Err(serde::de::Error::invalid_type(
            serde::de::Unexpected::UnitVariant,
            &"newtype variant",
        ))
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(serde::de::Error::invalid_type(
            serde::de::Unexpected::UnitVariant,
            &"tuple variant",
        ))
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(serde::de::Error::invalid_type(
            serde::de::Unexpected::UnitVariant,
            &"struct variant",
        ))
    }
}

fn from_trait<'de, R, T>(read: R) -> Result<T>
where
    R: Read,
    T: serde::de::Deserialize<'de>,
{
    let mut de = Deserializer::new(read);
    let value = serde_path_to_error::deserialize(&mut de)?;

    Ok(value)
}

pub(crate) fn from_hocon<'de, T>(hocon: Hocon) -> Result<T>
where
    T: serde::de::Deserialize<'de>,
{
    from_trait(HoconRead::new(hocon))
}

/// Deserialize a HOCON string directly
pub fn from_str<'de, T>(hocon: &str) -> std::result::Result<T, crate::Error>
where
    T: serde::de::Deserialize<'de>,
{
    from_trait(HoconRead::new(
        crate::HoconLoader::new().load_str(hocon)?.hocon()?,
    ))
    .map_err(|err| crate::Error::Deserialization {
        message: err.message,
    })
}

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use crate::Hocon;
    use linked_hash_map::LinkedHashMap;
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Deserialize, Debug)]
    struct Simple {
        int: i64,
        float: f64,
        option_int: Option<u64>,
    }
    #[derive(Deserialize, Debug)]
    struct WithSubStruct {
        vec_sub: Vec<Simple>,
        int: i32,
        float: f32,
        boolean: bool,
        string: String,
    }

    #[test]
    fn can_deserialize_struct() {
        let mut hm = LinkedHashMap::new();
        hm.insert(String::from("int"), Hocon::Integer(56));
        hm.insert(String::from("float"), Hocon::Real(543.12));
        hm.insert(String::from("boolean"), Hocon::Boolean(false));
        hm.insert(String::from("string"), Hocon::String(String::from("test")));
        let mut vec_sub = vec![];
        let mut subhm = LinkedHashMap::new();
        subhm.insert(String::from("int"), Hocon::Integer(5));
        subhm.insert(String::from("float"), Hocon::Integer(6));
        subhm.insert(String::from("extra"), Hocon::Integer(10));
        let subdoc = Hocon::Hash(subhm);
        vec_sub.push(subdoc);
        let mut subhm = LinkedHashMap::new();
        subhm.insert(String::from("int"), Hocon::Integer(5));
        subhm.insert(String::from("float"), Hocon::Integer(6));
        let subdoc = Hocon::Hash(subhm);
        vec_sub.push(subdoc);
        let mut subhm = LinkedHashMap::new();
        subhm.insert(String::from("int"), Hocon::Integer(5));
        subhm.insert(String::from("float"), Hocon::Integer(6));
        subhm.insert(String::from("extra"), Hocon::Null);
        let subdoc = Hocon::Hash(subhm);
        vec_sub.push(subdoc);
        hm.insert(String::from("vec_sub"), Hocon::Array(vec_sub));
        let doc = Hocon::Hash(hm);

        let res: super::Result<WithSubStruct> = dbg!(super::from_hocon(dbg!(doc)));
        assert!(res.is_ok());
    }

    #[test]
    fn will_fail_on_missing_field() {
        let mut hm = LinkedHashMap::new();
        hm.insert(String::from("int"), Hocon::Integer(5));
        let doc = Hocon::Hash(hm);

        let res: super::Result<Simple> = dbg!(super::from_hocon(dbg!(doc)));
        assert!(res.is_err());
    }

    #[test]
    fn will_not_fail_on_extra_field() {
        let mut hm = LinkedHashMap::new();
        hm.insert(String::from("int"), Hocon::Integer(5));
        hm.insert(String::from("float"), Hocon::Integer(6));
        hm.insert(String::from("extra"), Hocon::Integer(10));
        let doc = Hocon::Hash(hm);

        let res: super::Result<Simple> = dbg!(super::from_hocon(dbg!(doc)));
        assert!(res.is_ok());
    }

    #[test]
    fn will_fail_on_wrong_type() {
        let mut hm = LinkedHashMap::new();
        hm.insert(String::from("int"), Hocon::Integer(5));
        hm.insert(String::from("float"), Hocon::String(String::from("wrong")));
        let doc = Hocon::Hash(hm);
        let res: super::Result<Simple> = dbg!(super::from_hocon(dbg!(doc)));
        assert!(res.is_err());

        let mut hm = LinkedHashMap::new();
        hm.insert(String::from("int"), Hocon::Integer(56));
        hm.insert(String::from("float"), Hocon::Real(543.12));
        hm.insert(String::from("boolean"), Hocon::Boolean(false));
        hm.insert(String::from("string"), Hocon::Array(vec![]));
        hm.insert(String::from("vec_sub"), Hocon::Array(vec![]));
        let doc = Hocon::Hash(hm);
        let res: super::Result<WithSubStruct> = dbg!(super::from_hocon(dbg!(doc)));
        assert!(res.is_err());

        let mut hm = LinkedHashMap::new();
        hm.insert(String::from("int"), Hocon::Integer(56));
        hm.insert(String::from("float"), Hocon::Real(543.12));
        hm.insert(String::from("boolean"), Hocon::Integer(1));
        hm.insert(String::from("string"), Hocon::String(String::from("test")));
        hm.insert(String::from("vec_sub"), Hocon::Array(vec![]));
        let doc = Hocon::Hash(hm);
        let res: super::Result<WithSubStruct> = dbg!(super::from_hocon(dbg!(doc)));
        assert!(res.is_err());
    }

    #[test]
    fn access_hash_as_array() {
        #[derive(Deserialize, Debug)]
        struct WithArray {
            a: Vec<i32>,
        }

        let mut array = LinkedHashMap::new();
        array.insert(String::from("0"), Hocon::Integer(5));
        array.insert(String::from("2"), Hocon::Integer(7));
        let mut hm = LinkedHashMap::new();
        hm.insert(String::from("a"), Hocon::Hash(array));
        let doc = Hocon::Hash(hm);

        let res: super::Result<WithArray> = dbg!(super::from_hocon(dbg!(doc)));
        assert!(res.is_ok());
        assert_eq!(res.expect("during test").a, vec![5, 7]);
    }

    #[test]
    fn hocon_and_serde_default() {
        #[derive(Deserialize, Debug)]
        struct MyStructWithDefaultField {
            #[serde(default)]
            size: f64,
        }

        // let s: MyStruct = HoconLoader::new().load_str("").unwrap().resolve().unwrap();
        let doc = Hocon::Hash(LinkedHashMap::new());

        let res: super::Result<MyStructWithDefaultField> = dbg!(super::from_hocon(dbg!(doc)));
        assert!(res.is_ok());
        assert_eq!(res.expect("during test").size, 0.);
    }

    #[test]
    fn unit_struct() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct UnitStruct;

        #[derive(Deserialize, Debug)]
        struct MyStruct {
            item: UnitStruct,
        }

        let mut hm = LinkedHashMap::new();
        hm.insert(String::from("item"), Hocon::Null);
        let doc = Hocon::Hash(hm);

        let res: super::Result<MyStruct> = dbg!(super::from_hocon(dbg!(doc)));
        assert!(res.is_ok());
        assert_eq!(res.expect("during test").item, UnitStruct);
    }

    #[test]
    fn tuple() {
        #[derive(Deserialize, Debug)]
        struct MyStruct {
            item: (u64, String),
        }

        let mut hm = LinkedHashMap::new();
        let mut vec_sub = vec![];
        vec_sub.push(Hocon::Integer(0));
        vec_sub.push(Hocon::String(String::from("Hello")));
        hm.insert(String::from("item"), Hocon::Array(vec_sub));
        let doc = Hocon::Hash(hm);

        let res: super::Result<MyStruct> = dbg!(super::from_hocon(dbg!(doc)));
        assert!(res.is_ok());
        assert_eq!(res.expect("during test").item, (0, "Hello".to_string()));
    }

    #[test]
    fn tuple_struct() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct TupleStruct(u64, String);

        #[derive(Deserialize, Debug)]
        struct MyStruct {
            item: TupleStruct,
        }

        let mut hm = LinkedHashMap::new();
        let mut vec_sub = vec![];
        vec_sub.push(Hocon::Integer(0));
        vec_sub.push(Hocon::String(String::from("Hello")));
        hm.insert(String::from("item"), Hocon::Array(vec_sub));
        let doc = Hocon::Hash(hm);

        let res: super::Result<MyStruct> = dbg!(super::from_hocon(dbg!(doc)));
        assert!(res.is_ok());
        assert_eq!(
            res.expect("during test").item,
            TupleStruct(0, "Hello".to_string())
        );
    }

    #[test]
    fn map() {
        #[derive(Deserialize, Debug)]
        struct MyStruct {
            item: HashMap<String, u64>,
        }

        let mut hm = LinkedHashMap::new();
        let mut hm_sub = LinkedHashMap::new();
        hm_sub.insert(String::from("Hello"), Hocon::Integer(7));
        hm.insert(String::from("item"), Hocon::Hash(hm_sub));
        let doc = Hocon::Hash(hm);

        let res: super::Result<MyStruct> = dbg!(super::from_hocon(dbg!(doc)));
        assert!(res.is_ok());
        assert_eq!(res.expect("during test").item.get("Hello"), Some(&7));
    }

    #[test]
    fn map_with_enum_keys() {
        #[derive(Deserialize, Debug, Hash, PartialEq, Eq)]
        enum E {
            A,
            B,
        }

        let mut hm = LinkedHashMap::new();
        hm.insert(String::from("A"), Hocon::Integer(1));
        hm.insert(String::from("B"), Hocon::Integer(2));
        let doc = Hocon::Hash(hm);

        let res: super::Result<HashMap<E, u8>> = dbg!(super::from_hocon(dbg!(doc)));
        assert!(res.is_ok());
        assert_eq!(res.expect("during test").get(&E::A), Some(&1));

        #[derive(Deserialize, Debug)]
        struct S {
            s: u8,
        }

        let mut hm = LinkedHashMap::new();
        let mut hm_sub = LinkedHashMap::new();
        hm_sub.insert(String::from("s"), Hocon::Integer(7));
        hm.insert(String::from("A"), Hocon::Hash(hm_sub));
        let doc = Hocon::Hash(hm);

        let res: super::Result<HashMap<E, S>> = dbg!(super::from_hocon(dbg!(doc)));
        assert!(res.is_ok());
        assert_eq!(res.expect("during test").get(&E::A).unwrap().s, 7);
    }

    #[derive(Deserialize, Debug, PartialEq)]
    enum MyEnum {
        UnitVariant,
        TupleVariant(u64, bool),
        StructVariant { u: u64, b: bool },
    }

    #[derive(Deserialize, Debug)]
    struct MyStructWithEnum {
        item: MyEnum,
    }

    #[test]
    fn deserialize_unit_enum() {
        let mut hm = LinkedHashMap::new();
        hm.insert(
            String::from("item"),
            Hocon::String(String::from("UnitVariant")),
        );
        let doc = Hocon::Hash(hm);

        let res: super::Result<MyStructWithEnum> = dbg!(super::from_hocon(dbg!(doc)));
        assert!(res.is_ok());
        assert_eq!(res.expect("during test").item, MyEnum::UnitVariant);
    }

    #[test]
    fn deserialize_tuple_enum() {
        let mut hm = LinkedHashMap::new();
        let mut sub_hm = LinkedHashMap::new();
        sub_hm.insert(String::from("u"), Hocon::Integer(12));
        sub_hm.insert(String::from("b"), Hocon::Boolean(true));
        let mut variant_map = LinkedHashMap::new();
        variant_map.insert(String::from("StructVariant"), Hocon::Hash(sub_hm));
        hm.insert(String::from("item"), Hocon::Hash(variant_map));
        let doc = Hocon::Hash(hm);

        let res: super::Result<MyStructWithEnum> = dbg!(super::from_hocon(dbg!(doc)));
        assert!(res.is_ok());
        assert_eq!(
            res.expect("during test").item,
            MyEnum::StructVariant { u: 12, b: true }
        );
    }

    #[test]
    fn deserialize_struct_enum() {
        let mut hm = LinkedHashMap::new();
        let mut sub_vec = vec![];
        sub_vec.push(Hocon::Integer(7));
        sub_vec.push(Hocon::Boolean(false));
        let mut variant_map = LinkedHashMap::new();
        variant_map.insert(String::from("TupleVariant"), Hocon::Array(sub_vec));
        hm.insert(String::from("item"), Hocon::Hash(variant_map));
        let doc = Hocon::Hash(hm);

        let res: super::Result<MyStructWithEnum> = dbg!(super::from_hocon(dbg!(doc)));
        assert!(res.is_ok());
        assert_eq!(
            res.expect("during test").item,
            MyEnum::TupleVariant(7, false)
        );
    }

    #[test]
    fn deserialize_tagged_enum() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Container {
            rp: RetryPolicy,
        }
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(tag = "type")]
        pub enum RetryPolicy {
            NoRetry,
            Asap { num_retries: u32 },
        }

        let mut hm = LinkedHashMap::new();
        let mut sub_hm = LinkedHashMap::new();
        // sub_hm.insert(String::from("type"), Hocon::String(String::from("NoRetry")));
        sub_hm.insert(String::from("type"), Hocon::String(String::from("Asap")));
        sub_hm.insert(String::from("num_retries"), Hocon::Integer(7));
        hm.insert(String::from("rp"), Hocon::Hash(sub_hm));
        let doc = Hocon::Hash(hm);

        let res: super::Result<Container> = dbg!(super::from_hocon(dbg!(doc)));
        assert!(res.is_ok());
        assert_eq!(
            res.expect("during test").rp,
            RetryPolicy::Asap { num_retries: 7 }
        );
    }
}
