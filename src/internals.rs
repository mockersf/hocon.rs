use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

use super::{Hocon, HoconLoaderConfig};

macro_rules! bad_value_or_err {
    ( $config:expr, $err:expr ) => {
        if $config.strict {
            Err($err)?
        } else {
            HoconValue::BadValue($err)
        }
    };
}

macro_rules! public_bad_value_or_err {
    ( $config:expr, $err:expr ) => {
        if $config.strict {
            Err($err)?
        } else {
            Hocon::BadValue($err)
        }
    };
}

pub(crate) enum Include<'a> {
    File(&'a str),
    Url(&'a str),
}
impl<'a> Include<'a> {
    fn included(&self) -> &'a str {
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
                ..
            } => {
                let root = k
                    .iter()
                    .take(k.len() - original_path.len())
                    .cloned()
                    .collect();
                (
                    k,
                    HoconValue::ToConcatToArray {
                        value,
                        array_root: Some(root),
                        original_path,
                    },
                )
            }
            _ => (k, v),
        }
    }

    pub(crate) fn from_array(a: Vec<HoconInternal>) -> Self {
        let mut indexer: Box<dyn Fn(i64) -> HoconValue> = Box::new(HoconValue::Integer);
        if !a.is_empty() && a[0].internal.len() == 1 {
            if let HoconValue::PathSubstitutionInParent(_) = a[0].internal[0].1 {
                indexer = Box::new(|_| HoconValue::Null);
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
    ) -> Result<Self, crate::HoconError> {
        if config.include_depth > 10 || config.file_meta.is_none() {
            Ok(Self {
                internal: vec![(
                    vec![HoconValue::String(String::from(included.included()))],
                    bad_value_or_err!(config, crate::HoconError::TooManyIncludesError),
                )],
            })
        } else {
            let included_parsed = match included {
                Include::File(path) => {
                    let include_config = config
                        .included_from()
                        .with_file(std::path::Path::new(path).to_path_buf());
                    include_config
                        .read_file()
                        .map_err(|_| crate::error::HoconError::IncludeError {
                            path: String::from(path),
                        })
                        .and_then(|s| include_config.parse_str_to_internal(s))
                }
                #[cfg(feature = "url-support")]
                Include::Url(url) => {
                    config
                        .load_url(url)
                        .map_err(|_| crate::error::HoconError::IncludeError {
                            path: String::from(url),
                        })
                }
                #[cfg(not(feature = "url-support"))]
                _ => Err(crate::error::HoconError::DisabledExternalUrlError),
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
                        vec![HoconValue::String(String::from(included.included()))],
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
    ) -> Result<Self, crate::HoconError> {
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
    ) -> Result<HoconIntermediate, crate::HoconError> {
        let root = Rc::new(Child {
            key: HoconValue::Temp,
            value: RefCell::new(Node::Node {
                children: vec![],
                key_hint: None,
            }),
        });

        let mut last_path_encoutered = vec![];
        for (path, item) in self.internal {
            if path.is_empty() {
                continue;
            }

            let path = path
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
                HoconValue::PathSubstitutionInParent(v) => (
                    HoconValue::PathSubstitution(v).substitute(config, &root, &path),
                    path.into_iter().rev().skip(1).rev().collect(),
                ),
                HoconValue::ToConcatToArray {
                    value,
                    original_path,
                    ..
                } => (
                    value.substitute(config, &root, &path),
                    path.into_iter()
                        .rev()
                        .skip(original_path.len())
                        .rev()
                        .chain(std::iter::once(HoconValue::Null))
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
                ),
                v => (v.substitute(config, &root, &path), path),
            };

            let mut current_path = vec![];
            let mut current_node = Rc::clone(&root);
            for path_item in path {
                current_path.push(path_item.clone());
                let (target_child, child_list) = match current_node.value.borrow().deref() {
                    Node::Leaf(_) => {
                        let new_child = Rc::new(Child {
                            key: path_item,
                            value: RefCell::new(Node::Leaf(HoconValue::Temp)),
                        });

                        (Rc::clone(&new_child), vec![Rc::clone(&new_child)])
                    }
                    Node::Node { children, .. } => {
                        let exist = children.iter().find(|child| child.key == path_item);
                        let first_key = children.iter().next().map(|v| Rc::deref(v).key.clone());
                        match (exist, first_key) {
                            (_, Some(HoconValue::Integer(0)))
                                if path_item == HoconValue::Integer(0)
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
                            (Some(child), _) => (Rc::clone(child), children.clone()),
                            (None, _) => {
                                let new_child = Rc::new(Child {
                                    key: path_item.clone(),
                                    value: RefCell::new(Node::Leaf(HoconValue::Null)),
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
            *leaf = leaf_value?;
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

#[derive(Clone, Debug)]
enum KeyType {
    Int,
    String,
}

#[derive(Clone, Debug)]
enum Node {
    Leaf(HoconValue),
    Node {
        children: Vec<Rc<Child>>,
        key_hint: Option<KeyType>,
    },
}

impl Node {
    fn deep_clone(&self) -> Self {
        match self {
            Node::Leaf(v) => Node::Leaf(v.clone()),
            Node::Node { children, key_hint } => Node::Node {
                children: children.iter().map(|v| Rc::new(v.deep_clone())).collect(),
                key_hint: key_hint.clone(),
            },
        }
    }

    fn finalize(
        self,
        root: &HoconIntermediate,
        config: &HoconLoaderConfig,
        included_path: Option<Vec<HoconValue>>,
    ) -> Result<Hocon, crate::HoconError> {
        match self {
            Node::Leaf(v) => v.finalize(root, config, false, included_path),
            Node::Node {
                ref children,
                ref key_hint,
            } => children
                .first()
                .map(|ref first| match first.key {
                    HoconValue::Integer(_) | HoconValue::Null => {
                        Ok(Hocon::Array(crate::helper::extract_result(
                            children
                                .iter()
                                .map(|c| {
                                    c.value.clone().into_inner().finalize(
                                        root,
                                        config,
                                        included_path.clone(),
                                    )
                                })
                                .collect(),
                        )?))
                    }
                    HoconValue::String(_) => Ok(Hocon::Hash(
                        crate::helper::extract_result(
                            children
                                .iter()
                                .map(|c| {
                                    (
                                        c.key.clone().string_value(),
                                        c.value.clone().into_inner().finalize(
                                            root,
                                            config,
                                            included_path.clone(),
                                        ),
                                    )
                                })
                                .map(|(k, v)| v.map(|v| (k, v)))
                                .collect(),
                        )?
                        .into_iter()
                        .collect(),
                    )),
                    // Keys should only be integer or strings
                    _ => unreachable!(),
                })
                .unwrap_or_else(|| match key_hint {
                    Some(KeyType::Int) => Ok(Hocon::Array(vec![])),
                    Some(KeyType::String) | None => Ok(Hocon::Hash(HashMap::new())),
                }),
        }
    }

    fn find_key(
        &self,
        config: &HoconLoaderConfig,
        path: Vec<HoconValue>,
    ) -> Result<Node, crate::HoconError> {
        match (self, &path) {
            (Node::Leaf(_), ref path) if path.is_empty() => Ok(self.clone()),
            (Node::Node { children, .. }, _) => {
                let mut iter = path.clone().into_iter();
                let first = iter.nth(0);
                let remaining = iter.collect();

                match first {
                    None => Ok(self.clone()),
                    Some(first) => Ok(children
                        .iter()
                        .find(|child| child.key == first)
                        .and_then(|child| child.find_key(config, remaining).ok())
                        .unwrap_or(Node::Leaf(bad_value_or_err!(
                            config,
                            crate::HoconError::KeyNotFoundError
                        )))),
                }
            }
            _ => Ok(Node::Leaf(bad_value_or_err!(
                config,
                crate::HoconError::KeyNotFoundError
            ))),
        }
    }
}

#[derive(Debug)]
struct Child {
    key: HoconValue,
    value: RefCell<Node>,
}

impl Child {
    fn find_key(
        &self,
        config: &HoconLoaderConfig,
        path: Vec<HoconValue>,
    ) -> Result<Node, crate::HoconError> {
        self.value.clone().into_inner().find_key(config, path)
    }

    fn deep_clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            value: RefCell::new(self.value.clone().into_inner().deep_clone()),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct HoconIntermediate {
    tree: Node,
}

impl HoconIntermediate {
    pub(crate) fn finalize(self, config: &HoconLoaderConfig) -> Result<Hocon, crate::HoconError> {
        let refself = &self.clone();
        self.tree.finalize(refself, config, None)
    }
}

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
    BadValue(crate::HoconError),
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

    fn finalize(
        self,
        root: &HoconIntermediate,
        config: &HoconLoaderConfig,
        in_concat: bool,
        included_path: Option<Vec<HoconValue>>,
    ) -> Result<Hocon, crate::HoconError> {
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

    fn string_value(self) -> String {
        match self {
            HoconValue::String(s) => s,
            HoconValue::Null => String::from("null"),
            _ => unreachable!(),
        }
    }

    fn substitute(
        self,
        config: &HoconLoaderConfig,
        current_tree: &Rc<Child>,
        at_path: &[HoconValue],
    ) -> Result<Node, crate::HoconError> {
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
                            _ => Ok(bad_value_or_err!(config, crate::HoconError::ParseError)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_depth_of_include() {
        let val = dbg!(HoconInternal::from_include(
            Include::File("file.conf"),
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
                    HoconValue::BadValue(crate::HoconError::TooManyIncludesError)
                )]
            }
        );
    }

    #[test]
    fn missing_file_included() {
        let val = dbg!(HoconInternal::from_include(
            Include::File("file.conf"),
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
                    HoconValue::BadValue(crate::HoconError::IncludeError {
                        path: String::from("file.conf")
                    })
                )]
            }
        );
    }

}
