use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::{Hocon, HoconLoaderConfig};

use super::value::HoconValue;

use crate::internals::value;

#[derive(Clone, Debug)]
pub(crate) enum KeyType {
    Int,
    String,
}

#[derive(Clone, Debug)]
pub(crate) enum Node {
    Leaf(HoconValue),
    Node {
        children: Vec<Rc<Child>>,
        key_hint: Option<KeyType>,
    },
}

const STATIC_TRUE: bool = true;
const STATIC_FALSE: bool = false;
impl Child {
    fn is_array_leaf_included(&self) -> bool {
        *std::cell::Ref::map(self.value.borrow(), |v| {
            if let Node::Node { children, .. } = v {
                let is_included_leaf = children
                    .get(0)
                    .map(|child| {
                        *std::cell::Ref::map(child.value.borrow(), |v| {
                            if let Node::Leaf(HoconValue::Included { .. }) = v {
                                &true
                            } else {
                                &false
                            }
                        })
                    })
                    .unwrap_or(false);
                if is_included_leaf {
                    &STATIC_TRUE
                } else {
                    &STATIC_FALSE
                }
            } else {
                &STATIC_FALSE
            }
        })
    }
}

impl Node {
    pub(crate) fn deep_clone(&self) -> Self {
        match self {
            Node::Leaf(v) => Node::Leaf(v.clone()),
            Node::Node { children, key_hint } => Node::Node {
                children: children.iter().map(|v| Rc::new(v.deep_clone())).collect(),
                key_hint: key_hint.clone(),
            },
        }
    }

    pub(crate) fn finalize(
        self,
        root: &HoconIntermediate,
        config: &HoconLoaderConfig,
        included_path: Option<Vec<HoconValue>>,
        substituting_path: Option<Vec<HoconValue>>,
    ) -> Result<Hocon, crate::Error> {
        match self {
            Node::Leaf(v) => v.finalize(root, config, false, included_path, substituting_path),
            Node::Node {
                ref children,
                ref key_hint,
            } => children
                .first()
                .map(
                    |ref first| match (&first.key, &first.is_array_leaf_included()) {
                        (HoconValue::Null(_), true) => Ok(Hocon::Array(
                            crate::helper::extract_result(
                                children
                                    .iter()
                                    .map(|c| {
                                        c.value.clone().into_inner().finalize(
                                            root,
                                            config,
                                            included_path.clone(),
                                            substituting_path.clone(),
                                        )
                                    })
                                    .collect(),
                            )?
                            .into_iter()
                            .map(|v| {
                                if let Hocon::Array(vs) = v {
                                    vs.into_iter()
                                } else {
                                    vec![v].into_iter()
                                }
                            })
                            .flatten()
                            .collect(),
                        )),
                        (HoconValue::Integer(_), _) | (HoconValue::Null(_), _) => {
                            Ok(Hocon::Array(crate::helper::extract_result(
                                children
                                    .iter()
                                    .map(|c| {
                                        c.value.clone().into_inner().finalize(
                                            root,
                                            config,
                                            included_path.clone(),
                                            substituting_path.clone(),
                                        )
                                    })
                                    .collect(),
                            )?))
                        }

                        (HoconValue::String(_), _) => Ok(Hocon::Hash(
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
                                                substituting_path.clone(),
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
                    },
                )
                .unwrap_or_else(|| match key_hint {
                    Some(KeyType::Int) => Ok(Hocon::Array(vec![])),
                    Some(KeyType::String) | None => Ok(Hocon::Hash(HashMap::new())),
                }),
        }
    }

    pub(crate) fn find_key(
        &self,
        config: &HoconLoaderConfig,
        path: Vec<HoconValue>,
    ) -> Result<Node, crate::Error> {
        match (self, &path) {
            (Node::Leaf(_), ref path) if path.is_empty() => Ok(self.clone()),
            (Node::Node { children, .. }, _) => {
                let mut iter = path.clone().into_iter();
                let first = iter.next();
                let remaining = iter.collect();

                match first {
                    None => Ok(self.clone()),
                    Some(first) => Ok(
                        match children
                            .iter()
                            .find(|child| child.key == first)
                            .ok_or(crate::Error::KeyNotFound {
                                key: path
                                    .into_iter()
                                    .map(value::HoconValue::string_value)
                                    .collect::<Vec<_>>()
                                    .join("."),
                            })
                            .and_then(|child| child.find_key(config, remaining))
                        {
                            Ok(n) => n,
                            Err(err) => Node::Leaf(bad_value_or_err!(config, err)),
                        },
                    ),
                }
            }
            _ => Ok(Node::Leaf(bad_value_or_err!(
                config,
                crate::Error::KeyNotFound {
                    key: path
                        .into_iter()
                        .map(value::HoconValue::string_value)
                        .collect::<Vec<_>>()
                        .join(".")
                }
            ))),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Child {
    pub(crate) key: HoconValue,
    pub(crate) value: RefCell<Node>,
}

impl Child {
    pub(crate) fn find_key(
        &self,
        config: &HoconLoaderConfig,
        path: Vec<HoconValue>,
    ) -> Result<Node, crate::Error> {
        self.value.clone().into_inner().find_key(config, path)
    }

    pub(crate) fn deep_clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            value: RefCell::new(self.value.clone().into_inner().deep_clone()),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct HoconIntermediate {
    pub(crate) tree: Node,
}

impl HoconIntermediate {
    pub(crate) fn finalize(self, config: &HoconLoaderConfig) -> Result<Hocon, crate::Error> {
        #[allow(clippy::redundant_clone)]
        // looks like https://github.com/rust-lang/rust-clippy/issues/5707
        let refself = &self.clone();
        self.tree.finalize(refself, config, None, None)
    }
}
