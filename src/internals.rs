use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

use super::Hocon;

#[derive(Debug)]
pub(crate) struct HoconInternal {
    pub(crate) internal: Hash,
}

impl HoconInternal {
    pub(crate) fn from_value(v: HoconValue) -> Self {
        Self {
            internal: vec![(vec![], v)],
        }
    }

    pub(crate) fn from_object(h: Hash) -> Self {
        Self { internal: h }
    }

    pub(crate) fn from_array(a: Vec<HoconInternal>) -> Self {
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
                .and_then(|(p, s)| Hocon::parse_str_to_internal(Some(&p), &s, depth + 1))
        {
            included
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
            value: RefCell::new(Node::Node(vec![])),
        });

        for (path, item) in self.internal {
            let mut current_node = Rc::clone(&root);

            for path_item in path {
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
                        Node::Node(children) => {
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
                    current_node.value.replace(Node::Node(child_list));

                    current_node = target_child;
                }
            }
            let mut leaf = current_node.value.borrow_mut();
            *leaf = item.substitute(&root);
        }

        Ok(HoconIntermediate {
            tree: Rc::try_unwrap(root).unwrap().value.into_inner(),
        })
    }
}

pub(crate) type Path = Vec<HoconValue>;
pub(crate) type Hash = Vec<(Path, HoconValue)>;

#[derive(Clone, Debug)]
enum Node {
    Leaf(HoconValue),
    Node(Vec<Rc<Child>>),
}

impl Node {
    fn finalize(self) -> Hocon {
        match self {
            Node::Leaf(v) => v.finalize(),
            Node::Node(vec) => match vec.first().unwrap().key {
                HoconValue::Integer(_) => Hocon::Array(
                    vec.into_iter()
                        .map(|c| c.value.clone().into_inner().finalize())
                        .collect(),
                ),
                HoconValue::String(_) => Hocon::Hash(
                    vec.into_iter()
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
            },
        }
    }

    fn find_key(&self, path: Vec<HoconValue>) -> Node {
        match (self, &path) {
            (Node::Leaf(_), ref path) if path.is_empty() => self.clone(),
            (Node::Node(children), _) => {
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
            // This case should have been replaced during substitution
            // and not exist anymore at this point
            HoconValue::PathSubstitution(_) => unreachable!(),
        }
    }

    fn string_value(self) -> String {
        match self {
            HoconValue::String(s) => s,
            _ => unreachable!(),
        }
    }

    fn substitute(self, current_tree: &Rc<Child>) -> Node {
        match self {
            HoconValue::PathSubstitution(path) => current_tree.find_key(
                path.split('.')
                    .map(String::from)
                    .map(HoconValue::String)
                    .collect(),
            ),
            HoconValue::Concat(values) => dbg!(Node::Leaf(HoconValue::Concat(
                values
                    .into_iter()
                    .map(|v| v.substitute(&current_tree))
                    .map(|v| if let Node::Leaf(value) = v {
                        value
                    } else {
                        HoconValue::BadValue
                    })
                    .collect::<Vec<_>>()
            ))),
            v => Node::Leaf(v),
        }
    }
}

impl PartialEq for HoconValue {
    fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (HoconValue::Integer(left), HoconValue::Integer(right)) => left == right,
            (HoconValue::String(left), HoconValue::String(right)) => left == right,
            _ => false,
        }
    }
}
