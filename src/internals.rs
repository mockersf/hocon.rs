use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

use super::Hocon;

#[derive(Debug, PartialEq)]
pub(crate) struct HoconInternal {
    pub(crate) internal: Hash,
}

impl HoconInternal {
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
            Self { internal: h }
        }
    }

    pub(crate) fn from_array(a: Vec<HoconInternal>) -> Self {
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
                        .add_to_path(vec![HoconValue::Integer(i as i64)])
                        .internal
                        .into_iter()
                    })
                    .collect(),
            }
        }
    }

    pub(crate) fn from_include(file_root: Option<&str>, file_path: &str, depth: usize) -> Self {
        if depth > 10 || file_root.is_none() {
            Self {
                internal: vec![(
                    vec![HoconValue::String(String::from(file_path))],
                    HoconValue::BadValue,
                )],
            }
        } else if let Ok(included) =
            Hocon::load_file(file_root.expect("file_root is present"), file_path)
                .and_then(|(cf, s)| Hocon::parse_str_to_internal(Some(&cf.path), &s, depth + 1))
        {
            Self {
                internal: included
                    .internal
                    .into_iter()
                    .map(|(path, value)| {
                        (
                            path.clone(),
                            HoconValue::Included {
                                value: Box::new(value),
                                original_path: path,
                            },
                        )
                    })
                    .collect(),
            }
        } else {
            Self {
                internal: vec![(
                    vec![HoconValue::String(String::from(file_path))],
                    HoconValue::BadValue,
                )],
            }
        }
    }

    pub(crate) fn add_include(&mut self, file_root: &str, file_path: &str, depth: usize) -> Self {
        let mut included = Self::from_include(Some(file_root), file_path, depth);

        included.internal.append(&mut self.internal);

        included
    }

    pub(crate) fn add_to_path(self, p: Path) -> Self {
        Self {
            internal: self
                .internal
                .into_iter()
                .map(|(mut k, v)| {
                    let mut new_path = p.clone();
                    new_path.append(&mut k);
                    (new_path, v)
                })
                .collect(),
        }
    }

    pub(crate) fn merge(self) -> Result<HoconIntermediate, ()> {
        let root = Rc::new(Child {
            key: HoconValue::BadValue,
            value: RefCell::new(Node::Node {
                children: vec![],
                key_hint: None,
            }),
        });

        for (path, item) in self.internal {
            let mut current_node = Rc::clone(&root);

            for path_item in path.clone() {
                for path_item in match path_item {
                    HoconValue::UnquotedString(s) => s
                        .split('.')
                        .map(|s| HoconValue::String(String::from(s)))
                        .collect(),
                    _ => vec![path_item],
                } {
                    let (target_child, child_list) = match current_node.value.borrow().deref() {
                        Node::Leaf(_v) => {
                            let new_child = Rc::new(Child {
                                key: path_item.clone(),
                                value: RefCell::new(Node::Leaf(HoconValue::BadValue)),
                            });

                            (Rc::clone(&new_child), vec![Rc::clone(&new_child)])
                        }
                        Node::Node { children, .. } => {
                            let exist = children.iter().find(|child| child.key == path_item);
                            match exist {
                                Some(child) => (Rc::clone(child), children.clone()),
                                None => {
                                    let new_child = Rc::new(Child {
                                        key: path_item.clone(),
                                        value: RefCell::new(Node::Leaf(HoconValue::BadValue)),
                                    });
                                    let mut new_children = if children.is_empty() {
                                        children.clone()
                                    } else {
                                        match (
                                            Rc::deref(children.iter().next().unwrap()),
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
            }
            let mut leaf = current_node.value.borrow_mut();
            *leaf = item.substitute(&root, &path);
        }

        Ok(HoconIntermediate {
            tree: Rc::try_unwrap(root).unwrap().value.into_inner(),
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

    fn finalize(self) -> Hocon {
        match self {
            Node::Leaf(v) => v.finalize(),
            Node::Node {
                ref children,
                ref key_hint,
            } => children
                .first()
                .map(|ref first| match first.key {
                    HoconValue::Integer(_) => Hocon::Array(
                        children
                            .iter()
                            .map(|c| c.value.clone().into_inner().finalize())
                            .collect(),
                    ),
                    HoconValue::String(_) => Hocon::Hash(
                        children
                            .iter()
                            .map(|c| {
                                (
                                    c.key.clone().string_value(),
                                    c.value.clone().into_inner().finalize(),
                                )
                            })
                            .collect(),
                    ),
                    // Keys should only be integer or strings
                    _ => unreachable!(),
                })
                .unwrap_or_else(|| match key_hint {
                    Some(KeyType::Int) => Hocon::Array(vec![]),
                    Some(KeyType::String) | None => Hocon::Hash(HashMap::new()),
                }),
        }
    }

    fn find_key(&self, path: Vec<HoconValue>) -> Node {
        match (self, &path) {
            (Node::Leaf(_), ref path) if path.is_empty() => self.clone(),
            (Node::Node { children, .. }, _) => {
                let mut iter = path.clone().into_iter();
                let first = iter.nth(0);
                let remaining = iter.collect();

                match first {
                    None => self.clone(),
                    Some(first) => children
                        .iter()
                        .find(|child| child.key == first)
                        .map(|child| child.find_key(remaining))
                        .unwrap_or(Node::Leaf(HoconValue::BadValue)),
                }
            }
            _ => Node::Leaf(HoconValue::BadValue),
        }
    }
}

#[derive(Debug)]
struct Child {
    key: HoconValue,
    value: RefCell<Node>,
}

impl Child {
    fn find_key(&self, path: Vec<HoconValue>) -> Node {
        self.value.clone().into_inner().find_key(path)
    }

    fn deep_clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            value: RefCell::new(self.value.clone().into_inner().deep_clone()),
        }
    }
}

pub(crate) struct HoconIntermediate {
    tree: Node,
}

impl HoconIntermediate {
    pub(crate) fn finalize(self) -> Hocon {
        self.tree.finalize()
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
    PathSubstitution(String),
    Null,
    BadValue,
    EmptyObject,
    EmptyArray,
    Included {
        value: Box<HoconValue>,
        original_path: Vec<HoconValue>,
    },
}

impl HoconValue {
    fn finalize(self) -> Hocon {
        match self {
            HoconValue::Null => Hocon::Null,
            HoconValue::BadValue => Hocon::BadValue,
            HoconValue::Boolean(b) => Hocon::Boolean(b),
            HoconValue::Integer(i) => Hocon::Integer(i),
            HoconValue::Real(f) => Hocon::Real(f),
            HoconValue::String(s) => Hocon::String(s),
            HoconValue::UnquotedString(s) => Hocon::String(s),
            HoconValue::Concat(ref value) if value.len() == 1 => {
                value.clone().pop().unwrap().finalize()
            }
            HoconValue::Concat(values) => Hocon::String(
                values
                    .into_iter()
                    .map(HoconValue::finalize)
                    .filter_map(|v| v.as_string())
                    .collect::<Vec<String>>()
                    .join(""),
            ),
            // This cases should have been replaced during substitution
            // and not exist anymore at this point
            HoconValue::PathSubstitution(_) => unreachable!(),
            HoconValue::EmptyObject => unreachable!(),
            HoconValue::EmptyArray => unreachable!(),
            HoconValue::Included { .. } => unreachable!(),
        }
    }

    fn string_value(self) -> String {
        match self {
            HoconValue::String(s) => s,
            _ => unreachable!(),
        }
    }

    fn substitute(self, current_tree: &Rc<Child>, at_path: &[HoconValue]) -> Node {
        match self {
            HoconValue::PathSubstitution(path) => current_tree
                .find_key(
                    path.split('.')
                        .map(String::from)
                        .map(HoconValue::String)
                        .collect(),
                )
                .deep_clone(),
            HoconValue::Concat(values) => Node::Leaf(HoconValue::Concat(
                values
                    .into_iter()
                    .map(|v| v.substitute(&current_tree, at_path))
                    .map(|v| {
                        if let Node::Leaf(value) = v {
                            value
                        } else {
                            HoconValue::BadValue
                        }
                    })
                    .collect::<Vec<_>>(),
            )),
            HoconValue::EmptyObject => Node::Node {
                children: vec![],
                key_hint: Some(KeyType::String),
            },
            HoconValue::EmptyArray => Node::Node {
                children: vec![],
                key_hint: Some(KeyType::Int),
            },
            HoconValue::Included {
                value,
                original_path,
            } => {
                if let HoconValue::PathSubstitution(path) = *value.clone() {
                    let mut fixed_up_path = at_path
                        .iter()
                        .take(at_path.len() - original_path.len())
                        .cloned()
                        .flat_map(|path_item| match path_item {
                            HoconValue::String(_) => vec![path_item],
                            HoconValue::Integer(_) => vec![path_item],
                            HoconValue::UnquotedString(v) => v
                                .split('.')
                                .map(String::from)
                                .map(HoconValue::String)
                                .collect(),
                            _ => vec![HoconValue::BadValue],
                        })
                        .collect::<Vec<_>>();
                    fixed_up_path.append(
                        &mut path
                            .split('.')
                            .map(String::from)
                            .map(HoconValue::String)
                            .collect(),
                    );
                    match current_tree.find_key(fixed_up_path) {
                        Node::Leaf(HoconValue::BadValue) => (),
                        new_value => {
                            return new_value.deep_clone();
                        }
                    }
                }
                value.substitute(current_tree, &at_path)
            }
            v => Node::Leaf(v),
        }
    }
}

impl PartialEq for HoconValue {
    fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (HoconValue::Integer(left), HoconValue::Integer(right)) => left == right,
            (HoconValue::String(left), HoconValue::String(right)) => left == right,
            (HoconValue::BadValue, HoconValue::BadValue) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_depth_of_include() {
        let val = dbg!(HoconInternal::from_include(Some("./"), "file.conf", 15));
        assert_eq!(
            val,
            HoconInternal {
                internal: vec![(
                    vec![HoconValue::String(String::from("file.conf"))],
                    HoconValue::BadValue
                )]
            }
        );
    }

    #[test]
    fn missing_file_included() {
        let val = dbg!(HoconInternal::from_include(Some("./"), "file.conf", 1));
        assert_eq!(
            val,
            HoconInternal {
                internal: vec![(
                    vec![HoconValue::String(String::from("file.conf"))],
                    HoconValue::BadValue
                )]
            }
        );
    }

}
