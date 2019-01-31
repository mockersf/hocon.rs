use nom::*;

use std::str;

use crate::internals::{Hash, HoconInternal, HoconValue};

named!(integer<i64>, flat_map!(recognize_float, parse_to!(i64)));

named!(float<f64>, flat_map!(recognize_float, parse_to!(f64)));

//FIXME: verify how json strings are formatted
named!(
    string<&str>,
    delimited!(
        char!('\"'),
        map_res!(
            escaped!(call!(alphanumeric), '\\', one_of!("\"n\\")),
            str::from_utf8
        ),
        //map_res!(escaped!(take_while1!(is_alphanumeric), '\\', one_of!("\"n\\")), str::from_utf8),
        char!('\"')
    )
);

named!(
    boolean<bool>,
    alt!(value!(false, tag!("false")) | value!(true, tag!("true")))
);

named!(
    array<Vec<HoconInternal>>,
    ws!(delimited!(
        char!('['),
        separated_list!(alt!(char!(',') | char!('\n')), wrapper),
        alt!(char!(']') => { |_| () } | tag!(",]") => { |_| () } )
    ))
);

named!(
    key_value<Hash>,
    ws!(alt!(
        separated_pair!(string, alt!(char!(':') | char!('=')), wrapper)
            => { |(s, h): (&str, HoconInternal)|
                HoconInternal::from_object(h.internal)
                    .add_to_path(vec![HoconValue::String(String::from(s))]).internal
            } |
        pair!(string, hash)
            => { |(s, h): (&str, Hash)|
                HoconInternal::from_object(h)
                    .add_to_path(vec![HoconValue::String(String::from(s))]).internal
            }
    ))
);

named!(
    hash<Hash>,
    ws!(map!(
        delimited!(
            char!('{'),
            separated_list!(char!(','), key_value),
            alt!(char!('}') => { |_| () } | tag!(",}") => { |_| () } )
        ),
        |tuple_vec| tuple_vec.into_iter().flat_map(|h| h.into_iter()).collect()
    ))
);

named!(
    value<HoconValue>,
    ws!(alt!(
    //   hash    => { |h| HoconValue::Object(h)        } |
    //   array   => { |v| HoconValue::Array(v)                } |
      string  => { |s| HoconValue::String(String::from(s)) } |
      integer => { |i| HoconValue::Integer(i)              } |
      float   => { |f| HoconValue::Real(f)                 } |
      boolean => { |b| HoconValue::Boolean(b)              }
    ))
);

named!(
    pub(crate) wrapper<HoconInternal>,
    ws!(alt!(
        hash  => { |h| HoconInternal::from_object(h) } |
        array => { |a| HoconInternal::from_array(a)  } |
        value => { |v| HoconInternal::from_value(v)  }
    ))
);
