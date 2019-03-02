use nom::*;

use std::str;

use crate::internals::{Hash, HoconInternal, HoconValue};
use crate::HoconLoaderConfig;

named!(
    space<()>,
    map!(
        many0!(alt!(
            tag!(" ")
                | tag!("\t")
                | tag!("\u{feff}")
                | tag!("\u{00a0}")
                | tag!("\u{2007}")
                | tag!("\u{202f}")
        )),
        |_| ()
    )
);

macro_rules! sp (
    ($i:expr, $($args:tt)*) => (
        {
            use nom::Convert;
            use nom::Err;

            match sep!($i, space, $($args)*) {
                Err(e)         => Err(e),
                Ok((i1, o))    => {
                    match space(i1) {
                        Err(e) => Err(Err::convert(e)),
                        Ok((i2, _))    => Ok((i2, o))
                    }
                }
            }
        }
    )
);

named!(possible_comment<Option<()>>, opt!(multiline_comment));
named!(
    multiline_comment<()>,
    do_parse!(many0!(newline) >> comment >> many0!(alt!(newline => { |_| () } | comment)) >> ())
);
named!(
    comment<()>,
    sp!(do_parse!(
        alt!(tag!("//") | tag!("#")) >> take_until_and_consume!("\n") >> ()
    ))
);

named!(integer<i64>, flat_map!(recognize_float, parse_to!(i64)));

named!(
    float<f64>,
    map!(
        flat_map!(recognize_float, parse_to!(F64WithoutLeadingDot)),
        |v| v.0
    )
);

struct F64WithoutLeadingDot(f64);
impl std::str::FromStr for F64WithoutLeadingDot {
    type Err = ();
    fn from_str(v: &str) -> Result<Self, ()> {
        if let Some(".") = v.get(0..1) {
            return Err(());
        }
        v.parse::<f64>().map_err(|_| ()).map(F64WithoutLeadingDot)
    }
}

named!(null, tag!("null"));

//FIXME: verify how json strings are formatted
named!(
    string<&str>,
    delimited!(
        char!('"'),
        map_res!(
            escaped!(none_of!("\"\n"), '\\', one_of!("\"n\\")),
            str::from_utf8
        ),
        char!('"')
    )
);

macro_rules! take_until_3_chr (
  ($i:expr, $chr:expr) => (
    {
      use nom::lib::std::result::Result::*;
      use nom::lib::std::option::Option::*;
      use nom::{Needed,IResult,need_more_err, ErrorKind};

      use nom::InputTake;
      let input = $i;

      let res: IResult<_,_> = match find_3_chr(input, $chr) {
        None => {
          need_more_err($i, Needed::Size(3), ErrorKind::TakeUntil::<u32>)
        },
        Some(index) => {
          Ok($i.take_split(index))
        },
      };
      res
    }
  );
);

fn find_3_chr(input: &[u8], chr: u8) -> Option<usize> {
    let substr_len = 3;
    let substr = [chr; 3];

    if substr_len > input.len() {
        None
    } else {
        let max = input.len() - substr_len;
        let mut offset = 0;
        let mut haystack = &input[..];

        while let Some(mut position) = memchr::memchr(chr, haystack) {
            offset += position;

            if offset > max {
                return None;
            }

            if haystack[position..position + substr_len] == substr {
                while offset + substr_len < input.len() && haystack[position + substr_len] == chr {
                    position += 1;
                    offset += 1;
                }
                return Some(offset);
            }

            haystack = &haystack[position + 1..];
            offset += 1;
        }

        None
    }
}

named!(
    multiline_string<&str>,
    delimited!(
        tag!("\"\"\""),
        map_res!(
            take_until_3_chr!(AsBytes::as_bytes("\"")[0]),
            str::from_utf8
        ),
        tag!("\"\"\"")
    )
);

named!(
    boolean<bool>,
    alt!(value!(false, tag!("false")) | value!(true, tag!("true")))
);

macro_rules! take_until_tag1 (
    ($input:expr, $arr:expr) => (
        {
            use nom::lib::std::result::Result::*;
            use nom::lib::std::option::Option::*;
            use nom::{Err,Needed,IResult,need_more_err,ErrorKind};

            use nom::InputIter;
            use nom::InputTake;

            let res: IResult<_, _> = match $input.iter_indices()
                .fold((0, None),
                    |(old_c, pos), (i, c)| {
                        if pos.is_some() {
                            (old_c, pos)
                        } else if $arr.contains(&str::from_utf8(&[c]).unwrap()) {
                            (c, Some(i))
                        } else {
                            if $arr.contains(&str::from_utf8(&[old_c, c]).unwrap()) {
                                (c, Some(i - 1))
                            } else {
                                (c, None)
                            }
                        }
                    }
                )
            {
                (_, Some(0)) => Err(Err::Error(error_position!($input, ErrorKind::TakeUntilEither::<u32>))),
                (_, Some(n)) => Ok($input.take_split(n)),
                (_, None)    => need_more_err($input, Needed::Size(1), ErrorKind::TakeUntilEither::<u32>)
            };
            res
        }
    );
);

named!(
    unquoted_string<&str>,
    map_res!(
        complete!(take_until_tag1!([
            "$", "\"", "{", "}", "[", "]", ":", "=", ",", "+", "#", "`", "^", "?", "!", "@", "*",
            "&", "'", "\\", "\t", "\n", "//"
        ])),
        str::from_utf8
    )
);

named!(
    path_substitution<HoconValue>,
    delimited!(alt!(tag!("${?") | tag!("${")), value, char!('}'))
);

named_args!(
    arrays<'a>(config: &HoconLoaderConfig)<Vec<HoconInternal>>,
    map!(
        do_parse!(
            maybe_substitution: opt!(path_substitution)
                >> first_array: call!(array, config)
                >> remaining_arrays: many0!(call!(array, config))
                >> (maybe_substitution, first_array, remaining_arrays)
        ),
        |(maybe_substitution, mut first_array, remaining_arrays)| match (maybe_substitution, remaining_arrays.is_empty()) {
            (None, true) => first_array,
            (None, false) => {
                let mut values = first_array;
                remaining_arrays.into_iter().for_each(|mut array| values.append(&mut array));
                values
            }
            (Some(subst), _) => {
                let mut values = vec![HoconInternal::from_value(HoconValue::PathSubstitutionInParent(Box::new(subst)))];
                values.append(&mut first_array);
                remaining_arrays.into_iter().for_each(|mut array| values.append(&mut array));
                values
            }
        }
    )
);

named_args!(
    array<'a>(config: &HoconLoaderConfig)<Vec<HoconInternal>>,
    sp!(delimited!(
        do_parse!(char!('[') >> many0!(newline) >> ()),
        separated_list!(separators, call!(wrapper, config)),
        call!(closing, ']')
    ))
);

named_args!(
    key_value<'a>(config: &HoconLoaderConfig)<Hash>,
    do_parse!(
        ws!(possible_comment)
            >> pair: sp!(alt!(
                call!(include) => { |path| HoconInternal::from_include(path, config).internal } |
                separated_pair!(ws!(string), ws!(alt!(char!(':') | char!('='))), call!(wrapper, config))
                    => { |(s, h): (&str, HoconInternal)|
                        HoconInternal::from_object(h.internal)
                            .add_to_path(vec![HoconValue::String(String::from(s))]).internal
                    } |
                pair!(ws!(string), call!(hashes, config))
                    => { |(s, h): (&str, Hash)|
                        HoconInternal::from_object(h)
                            .add_to_path(vec![HoconValue::String(String::from(s))]).internal
                    } |
                // to concat to an array
                separated_pair!(ws!(string), ws!(tag!("+=")), call!(wrapper, config))
                    => { |(s, h): (&str, HoconInternal)|
                        HoconInternal::from_object(h.internal)
                            .transform(|k, v| (
                                k.clone(),
                                HoconValue::ToConcatToArray {
                                    value: Box::new(v),
                                    array_root: None,
                                    original_path: k
                                }
                            ))
                            .add_to_path(vec![HoconValue::String(String::from(s))]).internal
                    } |
                separated_pair!(ws!(unquoted_string), ws!(alt!(char!(':') | char!('='))), call!(wrapper, config))
                    => { |(s, h): (&str, HoconInternal)|
                        HoconInternal::from_object(h.internal)
                            .add_to_path(vec![HoconValue::UnquotedString(String::from(s))]).internal
                    } |
                pair!(ws!(unquoted_string), call!(hashes, config))
                    => { |(s, h): (&str, Hash)|
                        HoconInternal::from_object(h)
                            .add_to_path(vec![HoconValue::UnquotedString(String::from(s))]).internal
                    } |
                // to concat to an array
                separated_pair!(ws!(unquoted_string), ws!(tag!("+=")), call!(wrapper, config))
                    => { |(s, h): (&str, HoconInternal)|
                        HoconInternal::from_object(h.internal)
                            .transform(|k, v| (
                                k.clone(),
                                HoconValue::ToConcatToArray {
                                    value: Box::new(v),
                                    array_root: None,
                                    original_path: k
                                }
                            ))
                            .add_to_path(vec![HoconValue::UnquotedString(String::from(s))]).internal
                    }
            ))
            >> (pair)
    )
);

named!(
    separators<()>,
    alt!(
        sp!(multiline_comment) => { |_| () } |
        sp!(many1!(newline)) => { |_| () } |
        ws!(do_parse!(char!(',') >> possible_comment>> ())) => { |_| () }
    )
);

named_args!(
    separated_hashlist<'a>(config: &HoconLoaderConfig)<Vec<Hash>>,
    separated_list!(separators, call!(key_value, config))
);

named_args!(
    closing(closing_char: char)<()>,
    do_parse!(opt!(separators) >> char!(closing_char) >> ())
);

named_args!(
    hashes<'a>(config: &HoconLoaderConfig)<Hash>,
    map!(
        do_parse!(
            maybe_substitution: opt!(path_substitution)
                >> first_hash: call!(hash, config)
                >> remaining_hashes: many0!(call!(hash, config))
                >> (maybe_substitution, first_hash, remaining_hashes)
        ),
        |(maybe_substitution, mut first_hash, remaining_hashes)| match (maybe_substitution, remaining_hashes.is_empty()) {
            (None, true) => first_hash,
            (None, false) => {
                let mut values = first_hash;
                remaining_hashes.into_iter().for_each(|mut hash| values.append(&mut hash));
                values
            }
            (Some(subst), _) => {
                let mut values = vec![(vec![], HoconValue::PathSubstitution(Box::new(subst)))];
                values.append(&mut first_hash);
                remaining_hashes.into_iter().for_each(|mut hash| values.append(&mut hash));
                values
            }
        }
    )
);

named_args!(
    hash<'a>(config: &HoconLoaderConfig)<Hash>,
    sp!(map!(
        delimited!(char!('{'), call!(separated_hashlist, config), call!(closing, '}')),
        |tuple_vec| tuple_vec.into_iter().flat_map(|h| h.into_iter()).collect()
    ))
);

named_args!(
    root_hash<'a>(config: &HoconLoaderConfig)<Hash>,
    sp!(map!(
        do_parse!(not!(char!('{')) >> list: call!(separated_hashlist, config) >> (list)),
        |tuple_vec| tuple_vec.into_iter().flat_map(|h| h.into_iter()).collect()
    ))
);

named!(
    single_value<HoconValue>,
    alt!(
        multiline_string =>  { |s| HoconValue::String(String::from(s))         } |
        string  =>           { |s| HoconValue::String(String::from(s))         } |
        integer =>           { |i| HoconValue::Integer(i)                      } |
        float   =>           { |f| HoconValue::Real(f)                         } |
        boolean =>           { |b| HoconValue::Boolean(b)                      } |
        path_substitution => { |p| HoconValue::PathSubstitution(Box::new(p))   } |
        unquoted_string =>   { |s| HoconValue::UnquotedString(String::from(s)) } |
        null =>              { |_| HoconValue::Null                            }
    )
);

named!(
    value<HoconValue>,
    map!(
        do_parse!(
            possible_comment
                >> first_value: single_value
                >> remaining_values: many0!(single_value)
                >> (first_value, remaining_values)
        ),
        |(first_value, mut remaining_values)| if remaining_values.is_empty() {
            first_value
        } else {
            let mut values = vec![first_value];
            values.append(&mut remaining_values);
            HoconValue::maybe_concat(values)
        }
    )
);

named!(
    include<&str>,
    do_parse!(
        tag!("include ")
            >> ws!(many0!(newline))
            >> file_name:
                sp!(alt!(
                    call!(string)
                        | do_parse!(tag!("file(") >> file_name: string >> tag!(")") >> (file_name))
                ))
            >> (file_name)
    )
);

named_args!(
    root_include<'a>(config: &HoconLoaderConfig)<HoconInternal>,
    map!(
        do_parse!(file_name: ws!(include) >> doc: call!(root, config) >> ((file_name, doc))),
        |(file_name, mut doc)| doc.add_include(file_name, config)
    )
);

named_args!(
    wrapper<'a>(config: &HoconLoaderConfig)<HoconInternal>,
    do_parse!(
        possible_comment
            >> wrapped:
                alt!(
                    call!(hashes, config) => { |h| HoconInternal::from_object(h)          } |
                    call!(arrays, config) => { |a| HoconInternal::from_array(a)           } |
                    include               => { |f| HoconInternal::from_include(f, config) } |
                    value                 => { |v| HoconInternal::from_value(v)           }
                )
            >> (wrapped)
    )
);

named_args!(
    pub(crate) root<'a>(config: &HoconLoaderConfig)<HoconInternal>,
    do_parse!(
        possible_comment
            >> wrapped:
                alt!(
                    call!(root_include, config) => { |d| d                             } |
                    call!(root_hash, config)    => { |h| HoconInternal::from_object(h) } |
                    call!(hash, config)         => { |h| HoconInternal::from_object(h) } |
                    call!(array, config)        => { |a| HoconInternal::from_array(a)  } |
                    value                       => { |v| HoconInternal::from_value(v)  }
                )
            >> (wrapped)
    )
);
