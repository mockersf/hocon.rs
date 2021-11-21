use std::{
    borrow::Cow,
    str::{self, FromStr},
};

use nom::{
    branch::alt,
    bytes::complete::{escaped, is_not, tag, take_till1, take_until},
    character::complete::{char, newline, none_of, one_of},
    combinator::{map, not, opt, value},
    error::ParseError,
    multi::{many0, many1, separated_list0},
    number::complete::recognize_float,
    sequence::{delimited, pair, separated_pair, tuple},
    IResult,
};

use crate::{
    internals::{unescape, Hash, HoconInternal, HoconValue, Include},
    loader_config::HoconLoaderConfig,
};

fn ws<'a, F: 'a, O, E: 'a + ParseError<&'a str>>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(space, inner, space)
}

fn space<'a, E: 'a + ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, (), E> {
    value(
        (),
        many0(alt((
            tag(" "),
            tag("\t"),
            tag("\u{feff}"),
            tag("\u{00a0}"),
            tag("\u{2007}"),
            tag("\u{202f}"),
        ))),
    )(input)
}

fn integer<'a, E: 'a + ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, i64, E> {
    recognize_float(input).and_then(|(r, v)| {
        FromStr::from_str(v)
            .map(|v| (r, v))
            .map_err(|_e| nom::Err::Error(E::from_error_kind(input, nom::error::ErrorKind::Verify)))
    })
}

fn float<'a, E: 'a + ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, f64, E> {
    recognize_float(input).and_then(|(r, v)| {
        FromStr::from_str(v)
            .map(|v: F64WithoutLeadingDot| (r, v.0))
            .map_err(|_e| nom::Err::Error(E::from_error_kind(input, nom::error::ErrorKind::Verify)))
    })
}

struct F64WithoutLeadingDot(f64);
impl FromStr for F64WithoutLeadingDot {
    type Err = ();
    fn from_str(v: &str) -> Result<Self, ()> {
        if let Some(".") = v.get(0..1) {
            return Err(());
        }
        v.parse::<f64>().map_err(|_| ()).map(F64WithoutLeadingDot)
    }
}

//FIXME: verify how json strings are formatted
fn string<'a, E: 'a + ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Cow<str>, E> {
    ws(delimited(
        char('"'),
        map(
            escaped(none_of("\\\"\n"), '\\', one_of(r#""\/bfnrtu"#)),
            unescape,
        ),
        char('"'),
    ))(input)
}

fn multiline_string<'a, E: 'a + ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, &'a str, E> {
    delimited(tag(r#"""""#), take_until(r#"""""#), tag(r#"""""#))(input)
}

fn boolean<'a, E: 'a + ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, bool, E> {
    alt((value(false, tag("false")), value(true, tag("true"))))(input)
}

// TODO: missing stopping unquoted string on '//'
fn unquoted_string<'a, E: 'a + ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &str, E> {
    take_till1(|c| "&\"{}[]:=,+#`^?!@*&'\\\t\n".contains(c))(input)
}

fn path_substitution<'a, E: 'a + ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, HoconValue, E> {
    delimited(tag("${"), complex_value, char('}'))(input)
}

fn optional_path_substitution<'a, E: 'a + ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, HoconValue, E> {
    delimited(tag("${?"), complex_value, char('}'))(input)
}

fn arrays<'a, E: 'a + ParseError<&'a str>>(
    input: &'a str,
    config: &'a HoconLoaderConfig,
) -> IResult<&'a str, Vec<HoconInternal>, E> {
    map(
        tuple((
            opt(path_substitution),
            |i| array(i, config),
            many0(|i| array(i, config)),
        )),
        |(maybe_substitution, mut first_array, remaining_arrays)| match (
            maybe_substitution,
            remaining_arrays.is_empty(),
        ) {
            (None, true) => first_array,
            (None, false) => {
                let mut values = first_array;
                remaining_arrays
                    .into_iter()
                    .for_each(|mut array| values.append(&mut array));
                values
            }
            (Some(subst), _) => {
                let mut values = vec![HoconInternal::from_value(
                    HoconValue::PathSubstitutionInParent(Box::new(subst)),
                )];
                values.append(&mut first_array);
                remaining_arrays
                    .into_iter()
                    .for_each(|mut array| values.append(&mut array));
                values
            }
        },
    )(input)
}

fn array<'a, E: 'a + ParseError<&'a str>>(
    input: &'a str,
    config: &'a HoconLoaderConfig,
) -> IResult<&'a str, Vec<HoconInternal>, E> {
    ws(delimited(
        tuple((char('['), many0(newline))),
        separated_list0(separators, move |i| wrapper(i, config)),
        tuple((opt(separators), char(']'))),
    ))(input)
}

fn key_value<'a, E: 'a + ParseError<&'a str>>(
    input: &'a str,
    config: &'a HoconLoaderConfig,
) -> IResult<&'a str, Hash, E> {
    map(
        tuple((
            ws(maybe_comments),
            ws(alt((
                map(include, move |path| {
                    HoconInternal::from_include(path, config).unwrap().internal
                }),
                map(
                    separated_pair(ws(string), ws(alt((char(':'), char('=')))), move |i| {
                        wrapper(i, config)
                    }),
                    |(s, h)| {
                        HoconInternal::from_object(h.internal)
                            .add_to_path(vec![HoconValue::String(s.to_string())])
                            .internal
                    },
                ),
                map(pair(ws(string), move |i| hashes(i, config)), |(s, h)| {
                    HoconInternal::from_object(h)
                        .add_to_path(vec![HoconValue::String(s.to_string())])
                        .internal
                }),
                map(
                    separated_pair(ws(string), ws(tag("+=")), move |i| wrapper(i, config)),
                    |(s, h)| {
                        let item_id = uuid::Uuid::new_v4().to_hyphenated().to_string();
                        HoconInternal::from_object(h.internal)
                            .transform(|k, v| {
                                (
                                    k.clone(),
                                    HoconValue::ToConcatToArray {
                                        value: Box::new(v),
                                        original_path: k,
                                        item_id: item_id.clone(),
                                    },
                                )
                            })
                            .add_to_path(vec![HoconValue::String(s.to_string())])
                            .internal
                    },
                ),
                map(
                    separated_pair(
                        ws(unquoted_string),
                        ws(alt((char(':'), char('=')))),
                        move |i| wrapper(i, config),
                    ),
                    |(s, h)| {
                        HoconInternal::from_object(h.internal)
                            .add_to_path(vec![HoconValue::UnquotedString(s.to_string())])
                            .internal
                    },
                ),
                map(
                    pair(ws(unquoted_string), move |i| hashes(i, config)),
                    |(s, h)| {
                        HoconInternal::from_object(h)
                            .add_to_path(vec![HoconValue::UnquotedString(s.to_string())])
                            .internal
                    },
                ),
                map(
                    separated_pair(ws(unquoted_string), ws(tag("+=")), move |i| {
                        wrapper(i, config)
                    }),
                    |(s, h)| {
                        let item_id = uuid::Uuid::new_v4().to_hyphenated().to_string();
                        HoconInternal::from_object(h.internal)
                            .transform(|k, v| {
                                (
                                    k.clone(),
                                    HoconValue::ToConcatToArray {
                                        value: Box::new(v),
                                        original_path: k,
                                        item_id: item_id.clone(),
                                    },
                                )
                            })
                            .add_to_path(vec![HoconValue::UnquotedString(s.to_string())])
                            .internal
                    },
                ),
            ))),
        )),
        |p| p.1,
    )(input)
}

fn separators<'a, E: 'a + ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, (), E> {
    alt((
        multiline_comments,
        value((), many1(newline)),
        value((), tuple((char(','), maybe_comments))),
    ))(input)
}

fn separated_hashlist<'a, E: 'a + ParseError<&'a str>>(
    input: &'a str,
    config: &'a HoconLoaderConfig,
) -> IResult<&'a str, Vec<Hash>, E> {
    separated_list0(separators, |i| key_value(i, config))(input)
}

fn hashes<'a, E: 'a + ParseError<&'a str>>(
    input: &'a str,
    config: &'a HoconLoaderConfig,
) -> IResult<&'a str, Hash, E> {
    map(
        tuple((
            opt(path_substitution),
            |i| hash(i, config),
            many0(|i| hash(i, config)),
        )),
        |(maybe_substitution, mut first_hash, remaining_hashes)| match (
            maybe_substitution,
            remaining_hashes.is_empty(),
        ) {
            (None, true) => first_hash,
            (None, false) => {
                let mut values = first_hash;
                remaining_hashes
                    .into_iter()
                    .for_each(|mut hash| values.append(&mut hash));
                values
            }
            (Some(subst), _) => {
                let mut values = vec![(
                    vec![],
                    HoconValue::PathSubstitution {
                        target: Box::new(subst),
                        optional: false,
                        original: None,
                    },
                )];
                values.append(&mut first_hash);
                remaining_hashes
                    .into_iter()
                    .for_each(|mut hash| values.append(&mut hash));
                values
            }
        },
    )(input)
}

fn hash<'a, E: 'a + ParseError<&'a str>>(
    input: &'a str,
    config: &'a HoconLoaderConfig,
) -> IResult<&'a str, Hash, E> {
    ws(map(
        delimited(
            tuple((char('{'), many0(newline))),
            move |i| separated_hashlist(i, config),
            tuple((opt(separators), char('}'))),
        ),
        |tuple_vec| {
            tuple_vec
                .into_iter()
                .flat_map(std::iter::IntoIterator::into_iter)
                .collect()
        },
    ))(input)
}

fn root_hash<'a, E: 'a + ParseError<&'a str>>(
    input: &'a str,
    config: &'a HoconLoaderConfig,
) -> IResult<&'a str, Hash, E> {
    map(
        tuple((not(char('{')), |i| separated_hashlist(i, config))),
        |(_, tuple_vec)| {
            tuple_vec
                .into_iter()
                .flat_map(std::iter::IntoIterator::into_iter)
                .collect()
        },
    )(input)
}

fn single_value<'a, E: 'a + ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, HoconValue, E> {
    alt((
        map(multiline_string, |s| HoconValue::String(String::from(s))),
        map(string, |s| HoconValue::String(String::from(s))),
        map(integer, HoconValue::Integer),
        map(float, HoconValue::Real),
        map(boolean, HoconValue::Boolean),
        map(optional_path_substitution, |p| {
            HoconValue::PathSubstitution {
                target: Box::new(p),
                optional: true,
                original: None,
            }
        }),
        map(path_substitution, |p| HoconValue::PathSubstitution {
            target: Box::new(p),
            optional: false,
            original: None,
        }),
        map(unquoted_string, |s| {
            HoconValue::UnquotedString(String::from(s))
        }),
    ))(input)
}

fn complex_value<'a, E: 'a + ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, HoconValue, E> {
    map(
        tuple((maybe_comments, single_value, many0(single_value))),
        |(_, first_value, mut remaining_values)| {
            if remaining_values.is_empty() {
                first_value
            } else {
                let mut values = vec![first_value];
                values.append(&mut &mut remaining_values);
                HoconValue::maybe_concat(values)
            }
        },
    )(input)
}

fn wrapper<'a, E: 'a + ParseError<&'a str>>(
    input: &'a str,
    config: &'a HoconLoaderConfig,
) -> IResult<&'a str, HoconInternal, E> {
    map(
        tuple((
            maybe_comments,
            alt((
                map(|i| hashes(i, config), HoconInternal::from_object),
                map(|i| arrays(i, config), HoconInternal::from_array),
                map(include, |i| HoconInternal::from_include(i, config).unwrap()),
                map(complex_value, HoconInternal::from_value),
            )),
        )),
        |p| p.1,
    )(input)
}

fn maybe_comments<'a, E: 'a + ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, (), E> {
    Ok((
        many0(|i| {
            value(
                (),
                tuple((space, alt((tag("//"), tag("#"))), is_not("\n"), char('\n'))),
            )(i)
        })(input)?
        .0,
        (),
    ))
}

fn multiline_comments<'a, E: 'a + ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, (), E> {
    Ok((
        many1(|i| {
            value(
                (),
                tuple((space, alt((tag("//"), tag("#"))), is_not("\n"), char('\n'))),
            )(i)
        })(input)?
        .0,
        (),
    ))
}

fn include<'a, E: 'a + ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Include, E> {
    map(
        ws(tuple((
            tag("include "),
            many0(alt((space, value((), newline)))),
            alt((
                map(string, Include::File),
                map(tuple((tag("file("), string, tag(")"))), |p| {
                    Include::File(p.1)
                }),
                map(tuple((tag("url("), string, tag(")"))), |p| {
                    Include::Url(p.1)
                }),
            )),
        ))),
        |p| p.2,
    )(input)
}

fn root_include<'a, E: 'a + ParseError<&'a str>>(
    input: &'a str,
    config: &'a HoconLoaderConfig,
) -> IResult<&'a str, HoconInternal, E> {
    map(pair(include, |i| root(i, config)), |(included, mut doc)| {
        doc.add_include(included, config).unwrap()
    })(input)
}

pub(crate) fn root<'a, E: 'a + ParseError<&'a str>>(
    input: &'a str,
    config: &'a HoconLoaderConfig,
) -> IResult<&'a str, HoconInternal, E> {
    map(
        tuple((
            maybe_comments,
            alt((
                |i| root_include(i, config),
                map(|i| root_hash(i, config), HoconInternal::from_object),
                map(|i| hash(i, config), HoconInternal::from_object),
                map(|i| array(i, config), HoconInternal::from_array),
            )),
            maybe_comments,
        )),
        |p| p.1,
    )(input)
}

#[cfg(test)]
mod tests {
    use crate::{internals::HoconValue, loader_config::HoconLoaderConfig};

    use super::*;

    #[test]
    fn can_parse_comments() {
        assert_eq!(
            maybe_comments::<nom::error::VerboseError<&str>>("input")
                .unwrap()
                .0,
            "input"
        );
        assert_eq!(
            maybe_comments::<nom::error::VerboseError<&str>>("//input\n")
                .unwrap()
                .0,
            ""
        );
        assert_eq!(
            maybe_comments::<nom::error::VerboseError<&str>>("  //input\n")
                .unwrap()
                .0,
            ""
        );
        assert_eq!(
            maybe_comments::<()>("//input\n//input2\nremaining")
                .unwrap()
                .0,
            "remaining"
        );
    }

    #[test]
    fn can_parse_keyvalue() {
        let config = HoconLoaderConfig::default();
        assert_eq!(
            key_value::<nom::error::VerboseError<&str>>(r#""int":56"#, &config)
                .unwrap()
                .1,
            vec![(
                vec![HoconValue::String("int".to_string())],
                HoconValue::Integer(56)
            )]
        );
        assert_eq!(
            key_value::<nom::error::VerboseError<&str>>(r#"int:56"#, &config)
                .unwrap()
                .1,
            vec![(
                vec![HoconValue::UnquotedString("int".to_string())],
                HoconValue::Integer(56)
            )]
        );
    }

    #[test]
    fn can_parse_hash() {
        let config = HoconLoaderConfig::default();
        assert_eq!(
            hash::<nom::error::VerboseError<&str>>(r#"{"int":56}"#, &config)
                .unwrap()
                .1,
            vec![(
                vec![HoconValue::String("int".to_string())],
                HoconValue::Integer(56)
            )]
        );
        assert_eq!(
            hash::<nom::error::VerboseError<&str>>(r#"{"int":56, bool: true}"#, &config)
                .unwrap()
                .1,
            vec![
                (
                    vec![HoconValue::String("int".to_string())],
                    HoconValue::Integer(56)
                ),
                (
                    vec![HoconValue::UnquotedString("bool".to_string())],
                    HoconValue::Boolean(true)
                )
            ]
        );
    }

    #[test]
    fn can_parse_array() {
        let config = HoconLoaderConfig::default();
        assert_eq!(
            array::<nom::error::VerboseError<&str>>(r#"[5,4]"#, &config)
                .unwrap()
                .1,
            vec![
                HoconInternal {
                    internal: vec![(vec![], HoconValue::Integer(5))]
                },
                HoconInternal {
                    internal: vec![(vec![], HoconValue::Integer(4))]
                }
            ]
        );
    }
}
