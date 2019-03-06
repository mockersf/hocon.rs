use std::rc::Rc;

use crate::{Hocon, HoconLoaderConfig};

use super::intermediate::{Child, HoconIntermediate, KeyType, Node};

#[derive(Clone, Debug)]
pub(crate) enum HoconValue {
    Real(f64),
    Integer(i64),
    String(String),
    UnquotedString(String),
    Boolean(bool),
    Concat(Vec<HoconValue>),
    PathSubstitution(Box<HoconValue>),
    PathSubstitutionInParent(Box<HoconValue>),
    ToConcatToArray {
        value: Box<HoconValue>,
        array_root: Option<Vec<HoconValue>>,
        original_path: Vec<HoconValue>,
    },
    Null,
    // Placeholder for a value that will be replaced before returning final document
    Temp,
    // Placeholder to mark an error when not processing document strictly
    BadValue(crate::Error),
    // Placeholder for an empty object
    EmptyObject,
    // Placeholder for an empty array
    EmptyArray,
    Included {
        value: Box<HoconValue>,
        include_root: Option<Vec<HoconValue>>,
        original_path: Vec<HoconValue>,
    },
}

impl HoconValue {
    pub(crate) fn maybe_concat(values: Vec<HoconValue>) -> HoconValue {
        let nb_values = values.len();
        let trimmed_values: Vec<HoconValue> = values
            .into_iter()
            .enumerate()
            .filter_map(|item| match item {
                (0, HoconValue::UnquotedString(ref s)) if s.trim() == "" => None,
                (i, HoconValue::UnquotedString(ref s)) if s.trim() == "" && i == nb_values - 1 => {
                    None
                }
                (_, v) => Some(v),
            })
            .collect();
        match trimmed_values {
            ref values if values.len() == 1 => {
                values.first().expect("unexpected empty values").clone()
            }
            values => HoconValue::Concat(values),
        }
    }

    fn to_path(&self) -> Vec<HoconValue> {
        match self {
            HoconValue::UnquotedString(s) if s == "." => vec![],
            HoconValue::UnquotedString(s) => s
                .trim()
                .split('.')
                .map(String::from)
                .map(HoconValue::String)
                .collect(),
            HoconValue::String(s) => vec![HoconValue::String(s.clone())],
            HoconValue::Concat(values) => values.iter().flat_map(HoconValue::to_path).collect(),
            _ => vec![self.clone()],
        }
    }

    pub(crate) fn finalize(
        self,
        root: &HoconIntermediate,
        config: &HoconLoaderConfig,
        in_concat: bool,
        included_path: Option<Vec<HoconValue>>,
    ) -> Result<Hocon, crate::Error> {
        match self {
            HoconValue::Null => Ok(Hocon::Null),
            HoconValue::BadValue(err) => Ok(public_bad_value_or_err!(config, err)),
            HoconValue::Boolean(b) => Ok(Hocon::Boolean(b)),
            HoconValue::Integer(i) => Ok(Hocon::Integer(i)),
            HoconValue::Real(f) => Ok(Hocon::Real(f)),
            HoconValue::String(s) => Ok(Hocon::String(s)),
            HoconValue::UnquotedString(ref s) if s == "null" => Ok(Hocon::Null),
            HoconValue::UnquotedString(s) => {
                if in_concat {
                    Ok(Hocon::String(s))
                } else {
                    Ok(Hocon::String(String::from(s.trim())))
                }
            }
            HoconValue::Concat(values) => Ok(Hocon::String({
                let nb_items = values.len();
                values
                    .into_iter()
                    .enumerate()
                    .map(|item| match item {
                        (0, HoconValue::UnquotedString(s)) => {
                            HoconValue::UnquotedString(String::from(s.trim_start()))
                        }
                        (i, HoconValue::UnquotedString(ref s)) if i == nb_items - 1 => {
                            HoconValue::UnquotedString(String::from(s.trim_end()))
                        }
                        (_, v) => v,
                    })
                    .map(|v| v.finalize(root, config, true, included_path.clone()))
                    .filter_map(|v| v.ok().and_then(|v| v.as_internal_string()))
                    .collect::<Vec<String>>()
                    .join("")
            })),
            HoconValue::PathSubstitution(v) => {
                // second pass for substitution
                let fixed_up_path = if let Some(included_path) = included_path.clone() {
                    let mut fixed_up_path = included_path
                        .iter()
                        .cloned()
                        .flat_map(|path_item| path_item.to_path())
                        .collect::<Vec<_>>();
                    fixed_up_path.append(&mut v.to_path());
                    fixed_up_path
                } else {
                    v.to_path()
                };
                match (
                    config.strict,
                    config.system,
                    root.tree
                        .find_key(config, fixed_up_path.clone())
                        .and_then(|v| v.finalize(root, config, included_path)),
                ) {
                    (_, true, Err(err)) | (_, true, Ok(Hocon::BadValue(err))) => {
                        match std::env::var(
                            v.to_path()
                                .into_iter()
                                .map(HoconValue::string_value)
                                .collect::<Vec<_>>()
                                .join("."),
                        ) {
                            Ok(val) => Ok(Hocon::String(val)),
                            Err(_) => Ok(public_bad_value_or_err!(config, err)),
                        }
                    }
                    (true, _, Err(err)) | (true, _, Ok(Hocon::BadValue(err))) => Err(err)?,
                    (_, _, v) => v,
                }
            }
            HoconValue::Included {
                value,
                include_root,
                ..
            } => value
                .clone()
                .finalize(root, config, in_concat, include_root),
            // These cases should have been replaced during substitution
            // and not exist anymore at this point
            HoconValue::Temp => unreachable!(),
            HoconValue::EmptyObject => unreachable!(),
            HoconValue::EmptyArray => unreachable!(),
            HoconValue::PathSubstitutionInParent(_) => unreachable!(),
            HoconValue::ToConcatToArray { .. } => unreachable!(),
        }
    }

    pub(crate) fn string_value(self) -> String {
        match self {
            HoconValue::String(s) => s,
            HoconValue::Null => String::from("null"),
            _ => unreachable!(),
        }
    }

    pub(crate) fn substitute(
        self,
        config: &HoconLoaderConfig,
        current_tree: &Rc<Child>,
        at_path: &[HoconValue],
    ) -> Result<Node, crate::Error> {
        match self {
            HoconValue::PathSubstitution(path) => {
                match current_tree.find_key(config, path.to_path()) {
                    Err(_) | Ok(Node::Leaf(HoconValue::BadValue(_))) => {
                        // If node is not found, keep substitution to try again on second pass
                        Ok(Node::Leaf(HoconValue::PathSubstitution(path)))
                    }
                    Ok(v) => Ok(v.deep_clone()),
                }
            }
            HoconValue::Concat(values) => {
                let substituted = crate::helper::extract_result(
                    values
                        .into_iter()
                        .map(|v| v.substitute(config, &current_tree, at_path))
                        .map(|v| match v {
                            Ok(Node::Leaf(value)) => Ok(value),
                            Err(err) => Ok(bad_value_or_err!(config, err)),
                            _ => Ok(bad_value_or_err!(config, crate::Error::Parse)),
                        })
                        .collect::<Vec<_>>(),
                )?;

                Ok(Node::Leaf(HoconValue::Concat(substituted)))
            }
            HoconValue::EmptyObject => Ok(Node::Node {
                children: vec![],
                key_hint: Some(KeyType::String),
            }),
            HoconValue::EmptyArray => Ok(Node::Node {
                children: vec![],
                key_hint: Some(KeyType::Int),
            }),
            HoconValue::Included {
                value,
                original_path,
                include_root,
            } => {
                match *value.clone() {
                    HoconValue::PathSubstitution(path) => {
                        let root_path = at_path
                            .iter()
                            .take(at_path.len() - original_path.len())
                            .cloned()
                            .flat_map(|path_item| path_item.to_path())
                            .collect::<Vec<_>>();
                        let mut fixed_up_path = root_path.clone();
                        fixed_up_path.append(&mut path.to_path());
                        match current_tree.find_key(config, fixed_up_path.clone()) {
                            Ok(Node::Leaf(HoconValue::BadValue(_))) | Err(_) => (),
                            Ok(new_value) => {
                                return Ok(new_value.deep_clone());
                            }
                        }
                    }
                    HoconValue::Concat(values) => {
                        return HoconValue::Concat(
                            values
                                .into_iter()
                                .map(|value| HoconValue::Included {
                                    value: Box::new(value),
                                    original_path: original_path.clone(),
                                    include_root: include_root.clone(),
                                })
                                .collect(),
                        )
                        .substitute(config, current_tree, &at_path);
                    }
                    _ => (),
                }

                match value.substitute(config, current_tree, &at_path) {
                    Ok(Node::Leaf(value_found)) => {
                        // remember leaf was found inside an include
                        Ok(Node::Leaf(HoconValue::Included {
                            value: Box::new(value_found),
                            original_path,
                            include_root,
                        }))
                    }
                    v => v,
                }
            }
            v => Ok(Node::Leaf(v)),
        }
    }
}

impl PartialEq for HoconValue {
    fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (HoconValue::Integer(left), HoconValue::Integer(right)) => left == right,
            (HoconValue::String(left), HoconValue::String(right)) => left == right,
            (HoconValue::BadValue(left), HoconValue::BadValue(right)) => left == right,
            _ => false,
        }
    }
}
