use nom::*;

use std::str;

use crate::internals::{Hash, HoconInternal, HoconValue};

named!(pub space, eat_separator!(&b" \t"[..]));

macro_rules! sp (
  ($i:expr, $($args:tt)*) => (
    {
      use nom::Convert;
      use nom::Err;

      match sep!($i, space, $($args)*) {
        Err(e) => Err(e),
        Ok((i1,o))    => {
          match space(i1) {
            Err(e) => Err(Err::convert(e)),
            Ok((i2,_))    => Ok((i2, o))
          }
        }
      }
    }
  )
);

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
    sp!(delimited!(
        do_parse!(char!('[') >> many0!(newline) >> ()),
        separated_list!(separators, wrapper),
        call!(closing, ']')
    ))
);

named!(
    key_value<Hash>,
    sp!(alt!(
        separated_pair!(ws!(string), ws!(alt!(char!(':') | char!('='))), wrapper)
            => { |(s, h): (&str, HoconInternal)|
                HoconInternal::from_object(h.internal)
                    .add_to_path(vec![HoconValue::String(String::from(s))]).internal
            } |
        pair!(ws!(string), hash)
            => { |(s, h): (&str, Hash)|
                HoconInternal::from_object(h)
                    .add_to_path(vec![HoconValue::String(String::from(s))]).internal
            }
    ))
);

named!(
    separators<()>,
    alt!(sp!(many1!(newline)) => { |_| () } | ws!(char!(',')) => { |_| () })
);

named!(
    separated_hashlist<Vec<Hash>>,
    separated_list!(separators, key_value)
);

named_args!(
    closing(closing_char: char)<()>,
    do_parse!(opt!(separators) >> eat_separator!(&b" \t\n"[..]) >> char!(closing_char) >> ())
);

named!(
    hash<Hash>,
    sp!(map!(
        delimited!(char!('{'), separated_hashlist, call!(closing, '}')),
        |tuple_vec| tuple_vec.into_iter().flat_map(|h| h.into_iter()).collect()
    ))
);

named!(
    value<HoconValue>,
    alt!(
      string  => { |s| HoconValue::String(String::from(s)) } |
      integer => { |i| HoconValue::Integer(i)              } |
      float   => { |f| HoconValue::Real(f)                 } |
      boolean => { |b| HoconValue::Boolean(b)              }
    )
);

named!(
    pub(crate) wrapper<HoconInternal>,
    alt!(
        hash  => { |h| HoconInternal::from_object(h) } |
        array => { |a| HoconInternal::from_array(a)  } |
        value => { |v| HoconInternal::from_value(v)  }
    )
);
