use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

use super::Hocon;

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
                                    match (Rc::deref(children.iter().next().unwrap()), path_item) {
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
            let mut leaf = current_node.value.borrow_mut();
            *leaf = Node::Leaf(item);
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
                _ => unreachable!(),
            },
        }
    }
}

#[derive(Debug)]
struct Child {
    key: HoconValue,
    value: RefCell<Node>,
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
    String(std::string::String),
    Boolean(bool),
    BadValue,
}

impl HoconValue {
    fn finalize(self) -> Hocon {
        match self {
            HoconValue::BadValue => Hocon::BadValue,
            HoconValue::Boolean(b) => Hocon::Boolean(b),
            HoconValue::Integer(i) => Hocon::Integer(i),
            HoconValue::Real(f) => Hocon::Real(f),
            HoconValue::String(s) => Hocon::String(s),
        }
    }

    fn string_value(self) -> String {
        match self {
            HoconValue::String(s) => s,
            _ => unreachable!(),
        }
    }
}

impl std::cmp::PartialEq for HoconValue {
    fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (HoconValue::Integer(left), HoconValue::Integer(right)) => left == right,
            (HoconValue::String(left), HoconValue::String(right)) => left == right,
            _ => false,
        }
    }
}
