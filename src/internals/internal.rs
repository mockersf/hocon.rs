use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

use crate::HoconLoaderConfig;

use super::intermediate::{Child, HoconIntermediate, Node};
use super::value::HoconValue;

pub(crate) enum Include<'a> {
    File(Cow<'a, str>),
    Url(Cow<'a, str>),
}
impl<'a> Include<'a> {
    fn included(&self) -> &Cow<'a, str> {
        match self {
            Include::File(s) => s,
            Include::Url(s) => s,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct HoconInternal {
    pub(crate) internal: Hash,
}

impl HoconInternal {
    pub(crate) fn empty() -> Self {
        Self { internal: vec![] }
    }

    pub(crate) fn add(self, mut other: HoconInternal) -> Self {
        let mut elems = self.internal;
        elems.append(&mut other.internal);
        Self { internal: elems }
    }

    pub(crate) fn from_properties(properties: HashMap<String, String>) -> Self {
        Self {
            internal: properties
                .into_iter()
                .map(|(path, value)| {
                    (
                        path.split('.')
                            .map(|s| HoconValue::String(String::from(s)))
                            .collect(),
                        HoconValue::String(value),
                    )
                })
                .collect(),
        }
    }

    pub(crate) fn from_value(v: HoconValue) -> Self {
        Self {
            internal: vec![(vec![], v)],
        }
    }

    pub(crate) fn from_object(h: Hash) -> Self {
        if h.is_empty() {
            Self {
                internal: vec![(vec![], HoconValue::EmptyObject)],
            }
        } else {
            Self {
                internal: h
                    .into_iter()
                    .map(|(k, v)| Self::add_root_to_includes(k, v))
                    .collect(),
            }
        }
    }

    fn add_root_to_includes(k: Vec<HoconValue>, v: HoconValue) -> (Vec<HoconValue>, HoconValue) {
        match v {
            HoconValue::Included {
                value,
                original_path,
                ..
            } => {
                let root = k
                    .iter()
                    .take(k.len() - original_path.len())
                    .cloned()
                    .collect();
                (
                    k,
                    HoconValue::Included {
                        value,
                        include_root: Some(root),
                        original_path,
                    },
                )
            }
            HoconValue::ToConcatToArray {
                value,
                original_path,
                item_id,
                ..
            } => (
                k,
                HoconValue::ToConcatToArray {
                    value,
                    original_path,
                    item_id,
                },
            ),
            _ => (k, v),
        }
    }

    pub(crate) fn from_array(a: Vec<HoconInternal>) -> Self {
        let mut indexer: Box<dyn Fn(i64) -> HoconValue> = Box::new(HoconValue::Integer);
        if !a.is_empty() && a[0].internal.len() == 1 {
            if let HoconValue::PathSubstitutionInParent(_) = a[0].internal[0].1 {
                let index_prefix = uuid::Uuid::new_v4().to_hyphenated().to_string();
                indexer = Box::new(move |i| HoconValue::Null(format!("{}-{}", index_prefix, i)));
            }
        }
        if a.is_empty() {
            Self {
                internal: vec![(vec![], HoconValue::EmptyArray)],
            }
        } else {
            Self {
                internal: a
                    .into_iter()
                    .enumerate()
                    .flat_map(|(i, hw)| {
                        Self {
                            internal: hw.internal,
                        }
                        .add_to_path(vec![indexer(i as i64)])
                        .internal
                        .into_iter()
                    })
                    .map(|(k, v)| Self::add_root_to_includes(k, v))
                    .collect(),
            }
        }
    }

    pub(crate) fn from_include(
        included: Include,
        config: &HoconLoaderConfig,
    ) -> Result<Self, crate::Error> {
        if config.include_depth > config.max_include_depth {
            Ok(Self {
                internal: vec![(
                    vec![HoconValue::String(included.included().to_string())],
                    bad_value_or_err!(config, crate::Error::TooManyIncludes),
                )],
            })
        } else if config.file_meta.is_none() {
            Ok(Self {
                internal: vec![(
                    vec![HoconValue::String(included.included().to_string())],
                    bad_value_or_err!(config, crate::Error::IncludeNotAllowedFromStr),
                )],
            })
        } else {
            let included_parsed = match included {
                Include::File(ref path) => {
                    let include_config = config
                        .included_from()
                        .with_file(std::path::Path::new(path.as_ref()).to_path_buf());
                    include_config
                        .read_file()
                        .map_err(|_| crate::error::Error::Include {
                            path: path.to_string(),
                        })
                        .and_then(|s| include_config.parse_str_to_internal(s))
                }
                #[cfg(feature = "url-support")]
                Include::Url(ref url) => {
                    config
                        .load_url(url)
                        .map_err(|_| crate::error::Error::Include {
                            path: url.to_string(),
                        })
                }
                #[cfg(not(feature = "url-support"))]
                _ => Err(crate::error::Error::DisabledExternalUrl),
            };

            match included_parsed {
                Ok(included) => Ok(Self {
                    internal: included
                        .internal
                        .into_iter()
                        .map(|(path, value)| {
                            (
                                path.clone(),
                                HoconValue::Included {
                                    value: Box::new(value),
                                    original_path: path,
                                    include_root: None,
                                },
                            )
                        })
                        .collect(),
                }),
                Err(error) => Ok(Self {
                    internal: vec![(
                        vec![HoconValue::String(included.included().to_string())],
                        bad_value_or_err!(config, error),
                    )],
                }),
            }
        }
    }

    pub(crate) fn add_include(
        &mut self,
        included: Include,
        config: &HoconLoaderConfig,
    ) -> Result<Self, crate::Error> {
        let mut included = Self::from_include(included, config)?;

        included.internal.append(&mut self.internal);

        Ok(included)
    }

    pub(crate) fn add_to_path(self, p: Path) -> Self {
        self.transform(|mut k, v| {
            let mut new_path = p.clone();
            new_path.append(&mut k);
            (new_path, v)
        })
    }

    pub(crate) fn transform(
        self,
        transform: impl Fn(Vec<HoconValue>, HoconValue) -> (Vec<HoconValue>, HoconValue),
    ) -> Self {
        Self {
            internal: self
                .internal
                .into_iter()
                .map(|(k, v)| (transform(k, v)))
                .collect(),
        }
    }

    pub(crate) fn merge(
        self,
        config: &HoconLoaderConfig,
    ) -> Result<HoconIntermediate, crate::Error> {
        let root = Rc::new(Child {
            key: HoconValue::Temp,
            value: RefCell::new(Node::Node {
                children: vec![],
                key_hint: None,
            }),
        });

        let mut concatenated_arrays: HashMap<Path, HashMap<HoconValue, i64>> = HashMap::new();

        let mut last_path_encoutered = vec![];
        for (raw_path, item) in self.internal {
            if raw_path.is_empty() {
                continue;
            }

            let full_path = raw_path
                .clone()
                .into_iter()
                .flat_map(|path_item| match path_item {
                    HoconValue::UnquotedString(s) => s
                        .trim()
                        .split('.')
                        .map(|s| HoconValue::String(String::from(s)))
                        .collect(),
                    _ => vec![path_item],
                })
                .collect::<Vec<_>>();

            let (leaf_value, path) = match item {
                HoconValue::PathSubstitutionInParent(v) => {
                    let subst = HoconValue::PathSubstitution {
                        target: v,
                        optional: false,
                        original: None,
                    }
                    .substitute(config, &root, &full_path);
                    (subst, full_path.into_iter().rev().skip(1).rev().collect())
                }
                HoconValue::ToConcatToArray {
                    value,
                    original_path,
                    item_id,
                    ..
                } => {
                    let concat_root: Path = full_path
                        .iter()
                        .rev()
                        .skip(original_path.len())
                        .rev()
                        .cloned()
                        .collect();
                    let existing_array = concatenated_arrays
                        .entry(concat_root.clone())
                        .or_insert_with(HashMap::new);
                    let nb_elems = existing_array.keys().len();
                    let idx = existing_array
                        .entry(HoconValue::String(item_id.clone()))
                        .or_insert(nb_elems as i64);
                    (
                        value.substitute(config, &root, &full_path),
                        concat_root
                            .into_iter()
                            .chain(std::iter::once(HoconValue::Integer(*idx)))
                            .chain(original_path.into_iter().flat_map(|path_item| {
                                match path_item {
                                    HoconValue::UnquotedString(s) => s
                                        .trim()
                                        .split('.')
                                        .map(|s| HoconValue::String(String::from(s)))
                                        .collect(),
                                    _ => vec![path_item],
                                }
                            }))
                            .collect(),
                    )
                }
                HoconValue::PathSubstitution { ref target, .. } => {
                    let value = concatenated_arrays
                        .get(&target.to_path())
                        .cloned()
                        .unwrap_or_default();
                    concatenated_arrays
                        .entry(full_path.clone())
                        .or_insert(value);
                    (item.substitute(config, &root, &full_path), full_path)
                }
                v => {
                    let mut checked_path: Path = vec![];
                    for item in full_path.clone() {
                        if let HoconValue::Integer(idx) = item {
                            concatenated_arrays
                                .entry(checked_path.clone())
                                .or_insert_with(HashMap::new)
                                .entry(HoconValue::Integer(idx))
                                .or_insert(idx);
                        }
                        checked_path.push(item);
                    }
                    (v.substitute(config, &root, &full_path), full_path)
                }
            };

            let mut current_path = vec![];
            let mut current_node = Rc::clone(&root);
            let mut old_node_value_for_optional_substitution = None;
            for path_item in path {
                current_path.push(path_item.clone());
                let (target_child, child_list) = match current_node.value.borrow().deref() {
                    Node::Leaf(old_value) => {
                        let new_child = Rc::new(Child {
                            key: path_item,
                            value: RefCell::new(Node::Leaf(HoconValue::Temp)),
                        });

                        old_node_value_for_optional_substitution = Some(old_value.clone());

                        (Rc::clone(&new_child), vec![Rc::clone(&new_child)])
                    }
                    Node::Node { children, .. } => {
                        let exist = children.iter().find(|child| child.key == path_item);
                        let first_key = children.iter().next().map(|v| Rc::deref(v).key.clone());
                        match (exist, first_key) {
                            (_, Some(HoconValue::Integer(0)))
                                if path_item == HoconValue::Integer(0)
                                    && last_path_encoutered.len() >= current_path.len()
                                    && current_path.as_slice()
                                        != &last_path_encoutered[0..current_path.len()] =>
                            {
                                let mut new_children = vec![];
                                let new_child = Rc::new(Child {
                                    key: path_item.clone(),
                                    value: RefCell::new(Node::Leaf(HoconValue::Temp)),
                                });
                                new_children.push(Rc::clone(&new_child));

                                (new_child, new_children)
                            }
                            (Some(child), _) => {
                                if let Node::Leaf(old_val) = child.value.borrow().deref() {
                                    old_node_value_for_optional_substitution =
                                        Some(old_val.clone());
                                }
                                (Rc::clone(child), children.clone())
                            }
                            (None, _) => {
                                let new_child = Rc::new(Child {
                                    key: path_item.clone(),
                                    value: RefCell::new(Node::Leaf(HoconValue::Null(
                                        String::from("0"),
                                    ))),
                                });
                                let mut new_children = if children.is_empty() {
                                    children.clone()
                                } else {
                                    match (
                                        Rc::deref(
                                            children.iter().next().expect("got an empty iterator"),
                                        ),
                                        path_item,
                                    ) {
                                        (_, HoconValue::Integer(0)) => vec![],
                                        (
                                            Child {
                                                key: HoconValue::Integer(_),
                                                ..
                                            },
                                            HoconValue::String(_),
                                        ) => vec![],
                                        (
                                            Child {
                                                key: HoconValue::String(_),
                                                ..
                                            },
                                            HoconValue::Integer(_),
                                        ) => vec![],
                                        _ => children.clone(),
                                    }
                                };

                                new_children.push(Rc::clone(&new_child));
                                (new_child, new_children)
                            }
                        }
                    }
                };
                current_node.value.replace(Node::Node {
                    children: child_list,
                    key_hint: None,
                });

                current_node = target_child;
            }
            let mut leaf = current_node.value.borrow_mut();

            *leaf = match leaf_value? {
                Node::Leaf(HoconValue::PathSubstitution {
                    target,
                    optional,
                    original: previously_set_original,
                }) => Node::Leaf(HoconValue::PathSubstitution {
                    target,
                    optional,
                    original: previously_set_original
                        .or_else(|| old_node_value_for_optional_substitution.map(Box::new)),
                }),
                v => v,
            };
            last_path_encoutered = current_path;
        }

        Ok(HoconIntermediate {
            tree: Rc::try_unwrap(root)
                .expect("error getting Rc")
                .value
                .into_inner(),
        })
    }
}

pub(crate) type Path = Vec<HoconValue>;
pub(crate) type Hash = Vec<(Path, HoconValue)>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_depth_of_include() {
        let val = dbg!(HoconInternal::from_include(
            Include::File(Cow::from("file.conf")),
            &HoconLoaderConfig {
                include_depth: 15,
                file_meta: Some(crate::ConfFileMeta::from_path(
                    std::path::Path::new("file.conf").to_path_buf()
                )),
                ..Default::default()
            }
        ))
        .expect("during test");

        assert_eq!(
            val,
            HoconInternal {
                internal: vec![(
                    vec![HoconValue::String(String::from("file.conf"))],
                    HoconValue::BadValue(crate::Error::TooManyIncludes)
                )]
            }
        );
    }

    #[test]
    fn missing_file_included() {
        let val = dbg!(HoconInternal::from_include(
            Include::File(Cow::from("file.conf")),
            &HoconLoaderConfig {
                include_depth: 5,
                file_meta: Some(crate::ConfFileMeta::from_path(
                    std::path::Path::new("file.conf").to_path_buf()
                )),
                ..Default::default()
            }
        ))
        .expect("during test");

        assert_eq!(
            val,
            HoconInternal {
                internal: vec![(
                    vec![HoconValue::String(String::from("file.conf"))],
                    HoconValue::BadValue(crate::Error::Include {
                        path: String::from("file.conf")
                    })
                )]
            }
        );
    }
}
