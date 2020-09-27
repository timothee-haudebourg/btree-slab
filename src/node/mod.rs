use crate::{
	Item
};

mod leaf;
mod internal;

use leaf::Leaf as LeafNode;
use internal::Internal as InternalNode;

#[derive(Debug)]
pub enum Balance {
	Balanced,
	Overflow,
	Underflow(bool) // true if the node is empty.
}

pub struct WouldUnderflow;

/// B-tree node.
pub enum Node<K, V> {
	/// Internal node.
	Internal(InternalNode<K, V>),

	/// Leaf node.
	Leaf(LeafNode<K, V>)
}

impl<K, V> Node<K, V> {
	#[inline]
	pub fn binary(parent: Option<usize>, left_id: usize, median: Item<K, V>, right_id: usize) -> Node<K, V> {
		Node::Internal(InternalNode::binary(parent, left_id, median, right_id))
	}

	#[inline]
	pub fn leaf(parent: Option<usize>, item: Item<K, V>) -> Node<K, V> {
		Node::Leaf(LeafNode::new(parent, item))
	}

	#[inline]
	pub fn balance(&self) -> Balance {
		match self {
			Node::Internal(node) => node.balance(),
			Node::Leaf(leaf) => leaf.balance()
		}
	}

	#[inline]
	pub fn parent(&self) -> Option<usize> {
		match self {
			Node::Internal(node) => node.parent(),
			Node::Leaf(leaf) => leaf.parent()
		}
	}

	#[inline]
	pub fn set_parent(&mut self, p: Option<usize>) {
		match self {
			Node::Internal(node) => node.set_parent(p),
			Node::Leaf(leaf) => leaf.set_parent(p)
		}
	}

	#[inline]
	pub fn child_count(&self) -> usize {
		match self {
			Node::Internal(node) => node.child_count(),
			Node::Leaf(_) => 0
		}
	}

	#[inline]
	pub fn child_index(&self, id: usize) -> Option<usize> {
		match self {
			Node::Internal(node) => node.child_index(id),
			_ => None
		}
	}

	#[inline]
	pub fn child_id(&self, index: usize) -> usize {
		match self {
			Node::Internal(node) => node.child_id(index),
			_ => panic!("only internal nodes can be indexed")
		}
	}

	#[inline]
	pub fn child_id_opt(&self, index: usize) -> Option<usize> {
		match self {
			Node::Internal(node) => node.child_id_opt(index),
			Node::Leaf(_) => None
		}
	}

	#[inline]
	pub fn get(&self, key: &K) -> Result<Option<&V>, usize> where K: Ord {
		match self {
			Node::Leaf(leaf) => Ok(leaf.get(key)),
			Node::Internal(node) => match node.get(key) {
				Ok(value) => Ok(Some(value)),
				Err(e) => Err(e)
			}
		}
	}

	#[inline]
	pub fn get_mut(&mut self, key: &K) -> Result<Option<&mut V>, usize> where K: Ord {
		match self {
			Node::Leaf(leaf) => Ok(leaf.get_mut(key)),
			Node::Internal(node) => match node.get_mut(key) {
				Ok(value) => Ok(Some(value)),
				Err(e) => Err(e)
			}
		}
	}

	/// Find the offset of the item matching the given key.
	///
	/// If the key matches no item in this node,
	/// this funtion returns the index and id of the child that may match the key,
	/// or `Err(None)` if it is a leaf.
	#[inline]
	pub fn offset_of(&self, key: &K) -> Result<usize, (usize, Option<usize>)> where K: Ord {
		match self {
			Node::Internal(node) => match node.offset_of(key) {
				Ok(i) => Ok(i),
				Err((offset, child_id)) => Err((offset, Some(child_id)))
			},
			Node::Leaf(leaf) => match leaf.offset_of(key) {
				Ok(i) => Ok(i),
				Err(offset) =>  Err((offset, None))
			}
		}
	}

	#[inline]
	pub fn item_at_mut(&mut self, offset: usize) -> &mut Item<K, V> {
		match self {
			Node::Internal(node) => node.item_at_mut(offset),
			Node::Leaf(leaf) => leaf.item_at_mut(offset)
		}
	}

	#[inline]
	pub fn item_at_mut_opt(&mut self, offset: usize) -> Option<&mut Item<K, V>> {
		match self {
			Node::Internal(node) => node.item_at_mut_opt(offset),
			Node::Leaf(leaf) => leaf.item_at_mut_opt(offset)
		}
	}

	/// Insert by key.
	///
	/// It is assumed that the node is not free.
	/// If it is a leaf node, there must be a free space in it for the inserted value.
	#[inline]
	pub fn insert_by_key(&mut self, key: K, value: V) -> Result<(usize, Option<V>), (K, V, usize, usize)> where K: Ord {
		match self {
			Node::Internal(node) => match node.insert_by_key(key, value) {
				Ok((offset, value)) => Ok((offset, Some(value))),
				Err(e) => Err(e)
			},
			Node::Leaf(leaf) => Ok(leaf.insert_by_key(key, value))
		}
	}

	/// Split the node.
	/// Return the length of the node after split, the median item and the right node.
	#[inline]
	pub fn split(&mut self) -> (usize, Item<K, V>, Node<K, V>) {
		match self {
			Node::Internal(node) => {
				let (len, item, right_node) = node.split();
				(len, item, Node::Internal(right_node))
			},
			Node::Leaf(leaf) => {
				let (len, item, right_leaf) = leaf.split();
				(len, item, Node::Leaf(right_leaf))
			}
		}
	}

	#[inline]
	pub fn merge(&mut self, left_index: usize, right_index: usize) -> (usize, usize, usize, Item<K, V>, Balance) {
		match self {
			Node::Internal(node) => node.merge(left_index, right_index),
			_ => panic!("only internal nodes can merge children")
		}
	}

	/// Return the offset of the separator.
	#[inline]
	pub fn append(&mut self, separator: Item<K, V>, other: Node<K, V>) -> usize {
		match (self, other) {
			(Node::Internal(node), Node::Internal(other)) => node.append(separator, other),
			(Node::Leaf(leaf), Node::Leaf(other)) => leaf.append(separator, other),
			_ => panic!("incompatibles nodes")
		}
	}

	#[inline]
	pub fn push_left(&mut self, item: Item<K, V>, opt_child_id: Option<usize>) {
		match self {
			Node::Internal(node) => node.push_left(item, opt_child_id.unwrap()),
			Node::Leaf(leaf) => leaf.push_left(item)
		}
	}

	#[inline]
	pub fn pop_left(&mut self) -> Result<(Item<K, V>, Option<usize>), WouldUnderflow> {
		match self {
			Node::Internal(node) => {
				let (item, child_id) = node.pop_left()?;
				Ok((item, Some(child_id)))
			},
			Node::Leaf(leaf) => Ok((leaf.pop_left()?, None))
		}
	}

	#[inline]
	pub fn push_right(&mut self, item: Item<K, V>, opt_child_id: Option<usize>) -> usize {
		match self {
			Node::Internal(node) => node.push_right(item, opt_child_id.unwrap()),
			Node::Leaf(leaf) => leaf.push_right(item)
		}
	}

	#[inline]
	pub fn pop_right(&mut self) -> Result<(usize, Item<K, V>, Option<usize>), WouldUnderflow> {
		match self {
			Node::Internal(node) => {
				let (offset, item, child_id) = node.pop_right()?;
				Ok((offset, item, Some(child_id)))
			},
			Node::Leaf(leaf) => {
				let (offset, item) = leaf.pop_right()?;
				Ok((offset, item, None))
			}
		}
	}

	#[inline]
	pub fn leaf_remove(&mut self, offset: usize) -> Result<Item<K, V>, usize> {
		match self {
			Node::Internal(node) => {
				let left_child_index = offset;
				Err(node.child_id(left_child_index))
			},
			Node::Leaf(leaf) => Ok(leaf.remove(offset))
		}
	}

	/// Remove the item at the given offset.
	#[inline]
	pub fn remove(&mut self, offset: usize) -> Result<Item<K, V>, (usize, Item<K, V>, usize)> {
		match self {
			Node::Internal(node) => Err(node.remove(offset)),
			Node::Leaf(leaf) => Ok(leaf.remove(offset))
		}
	}

	#[inline]
	pub fn remove_rightmost_leaf(&mut self) -> Result<Item<K, V>, usize> {
		match self {
			Node::Internal(node) => {
				let child_index = node.child_count() - 1;
				let child_id = node.child_id(child_index);
				Err(child_id)
			},
			Node::Leaf(leaf) => Ok(leaf.remove_last())
		}
	}

	/// Put an item in a node.
	///
	/// It is assumed that the node will not overflow.
	#[inline]
	pub fn insert(&mut self, offset: usize, item: Item<K, V>, opt_right_child_id: Option<usize>) {
		match self {
			Node::Internal(node) => node.insert(offset, item, opt_right_child_id.unwrap()),
			Node::Leaf(leaf) => leaf.insert(offset, item)
		}
	}

	#[inline]
	pub fn replace(&mut self, offset: usize, item: Item<K, V>) -> Item<K, V> {
		match self {
			Node::Internal(node) => node.replace(offset, item),
			_ => panic!("can only replace in internal nodes")
		}
	}

	#[inline]
	pub fn separators(&self, i: usize) -> (Option<&K>, Option<&K>) {
		match self {
			Node::Leaf(_) => (None, None),
			Node::Internal(node) => node.separators(i)
		}
	}

	#[inline]
	pub fn children(&self) -> Children<K, V> {
		match self {
			Node::Leaf(_) => Children::Leaf,
			Node::Internal(node) => node.children()
		}
	}

	#[inline]
	pub fn children_with_separators(&self) -> ChildrenWithSeparators<K, V> {
		match self {
			Node::Leaf(_) => ChildrenWithSeparators::Leaf,
			Node::Internal(node) => node.children_with_separators()
		}
	}

	/// Write the label of the node in the DOT format.
	///
	/// Requires the `dot` feature.
	#[cfg(feature = "dot")]
	#[inline]
	pub fn dot_write_label<W: std::io::Write>(&self, f: &mut W) -> std::io::Result<()> where K: std::fmt::Display, V: std::fmt::Display {
		match self {
			Node::Leaf(leaf) => leaf.dot_write_label(f),
			Node::Internal(node) => node.dot_write_label(f)
		}
	}

	#[cfg(debug_assertions)]
	pub fn validate(&self, parent: Option<usize>, min: Option<&K>, max: Option<&K>) where K: Ord {
		match self {
			Node::Leaf(leaf) => leaf.validate(parent, min, max),
			Node::Internal(node) => node.validate(parent, min, max)
		}
	}
}

pub enum Children<'a, K, V> {
	Leaf,
	Internal(Option<usize>, std::slice::Iter<'a, internal::Branch<K, V>>)
}

impl<'a, K, V> Iterator for Children<'a, K, V> {
	type Item = usize;

	#[inline]
	fn next(&mut self) -> Option<usize> {
		match self {
			Children::Leaf => None,
			Children::Internal(first, rest) => {
				match first.take() {
					Some(child) => Some(child),
					None => {
						match rest.next() {
							Some(branch) => Some(branch.child),
							None => None
						}
					}
				}
			}
		}
	}
}

pub enum ChildrenWithSeparators<'a, K, V> {
	Leaf,
	Internal(Option<usize>, Option<&'a Item<K, V>>, std::iter::Peekable<std::slice::Iter<'a, internal::Branch<K, V>>>)
}

impl<'a, K, V> Iterator for ChildrenWithSeparators<'a, K, V> {
	type Item = (Option<&'a Item<K, V>>, usize, Option<&'a Item<K, V>>);

	#[inline]
	fn next(&mut self) -> Option<Self::Item> {
		match self {
			ChildrenWithSeparators::Leaf => None,
			ChildrenWithSeparators::Internal(first, left_sep, rest) => {
				match first.take() {
					Some(child) => {
						let right_sep = match rest.peek() {
							Some(right) => Some(&right.item),
							None => None
						};
						*left_sep = right_sep;
						Some((None, child, right_sep))
					},
					None => {
						match rest.next() {
							Some(branch) => {
								let right_sep = match rest.peek() {
									Some(right) => Some(&right.item),
									None => None
								};
								let result = Some((*left_sep, branch.child, right_sep));
								*left_sep = right_sep;
								result
							},
							None => None
						}
					}
				}
			}
		}
	}
}
