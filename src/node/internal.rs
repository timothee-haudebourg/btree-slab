use std::cmp::{
	PartialOrd,
	Ord,
	Ordering
};
use staticvec::StaticVec;
use crate::{
	M,
	Item,
	Children,
	ChildrenWithSeparators,
	Balance,
	WouldUnderflow,
	utils::binary_search_min
};

pub struct Branch<K, V> {
	pub item: Item<K, V>,
	pub child: usize
}

impl<K: PartialEq, V> PartialEq<K> for Branch<K, V> {
	fn eq(&self, key: &K) -> bool {
		self.item.key().eq(key)
	}
}

impl<K: Ord + PartialEq, V> PartialOrd<K> for Branch<K, V> {
	fn partial_cmp(&self, key: &K) -> Option<Ordering> {
		Some(self.item.key().cmp(key))
	}
}

impl<K: PartialEq, V> PartialEq for Branch<K, V> {
	fn eq(&self, other: &Branch<K, V>) -> bool {
		self.item.key().eq(other.item.key())
	}
}

impl<K: Ord + PartialEq, V> PartialOrd for Branch<K, V> {
	fn partial_cmp(&self, other: &Branch<K, V>) -> Option<Ordering> {
		Some(self.item.key().cmp(other.item.key()))
	}
}

pub struct Internal<K, V> {
	parent: usize,
	first_child: usize,
	other_children: StaticVec<Branch<K, V>, M>
}

impl<K, V> Internal<K, V> {
	#[inline]
	pub fn binary(parent: Option<usize>, left_id: usize, median: Item<K, V>, right_id: usize) -> Internal<K, V> {
		let mut other_children = StaticVec::new();
		other_children.push(Branch {
			item: median,
			child: right_id
		});

		Internal {
			parent: parent.unwrap_or(std::usize::MAX),
			first_child: left_id,
			other_children
		}
	}

	#[inline]
	pub fn balance(&self) -> Balance {
		if self.item_count() == M {
			Balance::Overflow
		} else if self.item_count() < M/2 {
			Balance::Underflow(self.other_children.is_empty())
		} else {
			Balance::Balanced
		}
	}

	#[inline]
	pub fn parent(&self) -> Option<usize> {
		if self.parent == std::usize::MAX {
			None
		} else {
			Some(self.parent)
		}
	}

	#[inline]
	pub fn set_parent(&mut self, p: Option<usize>) {
		self.parent = p.unwrap_or(std::usize::MAX);
	}

	#[inline]
	pub fn item_count(&self) -> usize {
		self.other_children.len()
	}

	#[inline]
	pub fn child_count(&self) -> usize {
		1usize + self.item_count()
	}

	#[inline]
	pub fn first_child_id(&self) -> usize {
		self.first_child
	}

	#[inline]
	pub fn branches(&self) -> &[Branch<K, V>] {
		self.other_children.as_ref()
	}

	#[inline]
	pub fn child_index(&self, id: usize) -> Option<usize> {
		if self.first_child == id {
			Some(0)
		} else {
			for i in 0..self.other_children.len() {
				if self.other_children[i].child == id {
					return Some(i+1)
				}
			}

			None
		}
	}

	#[inline]
	pub fn child_id(&self, index: usize) -> usize {
		if index == 0 {
			self.first_child
		} else {
			self.other_children[index - 1].child
		}
	}

	#[inline]
	pub fn child_id_opt(&self, index: usize) -> Option<usize> {
		if index == 0 {
			Some(self.first_child)
		} else {
			self.other_children.get(index - 1).map(|b| b.child)
		}
	}

	#[inline]
	pub fn separators(&self, index: usize) -> (Option<&K>, Option<&K>) {
		let min = if index > 0 {
			Some(self.other_children[index - 1].item.key())
		} else {
			None
		};

		let max = if index < self.other_children.len() {
			Some(self.other_children[index].item.key())
		} else {
			None
		};

		(min, max)
	}

	#[inline]
	pub fn get(&self, key: &K) -> Result<&V, usize> where K: Ord {
		match binary_search_min(&self.other_children, key) {
			Some(offset) => {
				let b = &self.other_children[offset];
				if b.item.key() == key {
					Ok(b.item.value())
				} else {
					Err(b.child)
				}
			},
			None => Err(self.first_child)
		}
	}

	#[inline]
	pub fn get_mut(&mut self, key: &K) -> Result<&mut V, usize> where K: Ord {
		match binary_search_min(&self.other_children, key) {
			Some(offset) => {
				let b = &mut self.other_children[offset];
				if b.item.key() == key {
					Ok(b.item.value_mut())
				} else {
					Err(b.child)
				}
			},
			None => Err(self.first_child)
		}
	}

	/// Find the offset of the item matching the given key.
	///
	/// If the key matches no item in this node,
	/// this funtion returns the index and id of the child that may match the key.
	#[inline]
	pub fn offset_of(&self, key: &K) -> Result<usize, (usize, usize)> where K: Ord {
		match binary_search_min(&self.other_children, key) {
			Some(offset) => {
				if self.other_children[offset].item.key() == key {
					Ok(offset)
				} else {
					let id = self.other_children[offset].child;
					Err((offset+1, id))
				}
			},
			None => Err((0, self.first_child))
		}
	}

	#[inline]
	pub fn children(&self) -> Children<K, V> {
		Children::Internal(Some(self.first_child), self.other_children.as_ref().iter())
	}

	#[inline]
	pub fn children_with_separators(&self) -> ChildrenWithSeparators<K, V> {
		ChildrenWithSeparators::Internal(Some(self.first_child), None, self.other_children.as_ref().iter().peekable())
	}

	#[inline]
	pub fn item_at_mut(&mut self, offset: usize) -> &mut Item<K, V> {
		&mut self.other_children[offset].item
	}

	#[inline]
	pub fn item_at_mut_opt(&mut self, offset: usize) -> Option<&mut Item<K, V>> {
		match self.other_children.get_mut(offset) {
			Some(b) => Some(&mut b.item),
			None => None
		}
	}

	/// Insert by key
	#[inline]
	pub fn insert_by_key(&mut self, key: K, mut value: V) -> Result<(usize, V), (K, V, usize, usize)> where K: Ord {
		match binary_search_min(&self.other_children, &key) {
			Some(i) => {
				if self.other_children[i].item.key() == &key {
					std::mem::swap(&mut value, self.other_children[i].item.value_mut());
					Ok((i, value))
				} else {
					Err((key, value, i+1, self.other_children[i].child))
				}
			},
			None => {
				Err((key, value, 0, self.first_child))
			}
		}
	}

	// /// Get the offset of the item with the given key.
	// #[inline]
	// pub fn key_offset(&self, key: &K) -> Result<usize, (usize, usize)> {
	// 	match binary_search_min(&self.other_children, key) {
	// 		Some(i) => {
	// 			if self.other_children[i].item.key() == key {
	// 				Ok(i)
	// 			} else {
	// 				Err((i+1, self.other_children[i].child))
	// 			}
	// 		},
	// 		None => {
	// 			Err((0, self.first_child))
	// 		}
	// 	}
	// }

	/// Insert item at the given offset.
	#[inline]
	pub fn insert(&mut self, offset: usize, item: Item<K, V>, right_node_id: usize) {
		self.other_children.insert(offset, Branch {
			item,
			child: right_node_id
		});
	}

	/// Replace the item at the given offset.
	#[inline]
	pub fn replace(&mut self, offset: usize, mut item: Item<K, V>) -> Item<K, V> {
		std::mem::swap(&mut item, &mut self.other_children[offset].item);
		item
	}

	/// Remove the item at the given offset.
	/// Return the child id on the left of the item, the item, and the child id on the right
	/// (which is also removed).
	#[inline]
	pub fn remove(&mut self, offset: usize) -> (usize, Item<K, V>, usize) {
		let left_child_id = self.child_id(offset);
		let b = self.other_children.remove(offset);
		(left_child_id, b.item, b.child)
	}

	#[inline]
	pub fn split(&mut self) -> (usize, Item<K, V>, Internal<K, V>) {
		// Index of the median-key item in `other_children`.
		let median_i = self.other_children.len() / 2; // Since M is at least 3, `median_i` is at least 1.

		let right_other_children = self.other_children.drain(median_i+1..);
		let median = self.other_children.pop().unwrap();

		let right_node = Internal {
			parent: self.parent,
			first_child: median.child,
			other_children: right_other_children
		};

		(self.other_children.len(), median.item, right_node)
	}

	/// Merge the children at the given indexes.
	///
	/// It is supposed that `left_index` is `right_index-1`.
	/// This method returns the identifier of the left node in the tree, the identifier of the right node,
	/// the item removed from this node to be merged with the merged children and
	/// the balance status of this node after the merging operation.
	#[inline]
	pub fn merge(&mut self, left_index: usize, right_index: usize) -> (usize, usize, usize, Item<K, V>, Balance) {
		let left_id = self.child_id(left_index);
		let right_id = self.child_id(right_index);

		// We remove the right child (the one of index `right_index`).
		// Since left_index = right_index-1, it is indexed by `left_index` in `other_children`.
		let item = self.other_children.remove(left_index).item;

		let item_count = self.item_count();
		let balancing = if item_count >= M/2 {
			Balance::Balanced
		} else {
			Balance::Underflow(item_count == 0)
		};

		(left_index, left_id, right_id, item, balancing)
	}

	#[inline]
	pub fn push_left(&mut self, item: Item<K, V>, child_id: usize) {
		self.other_children.insert(0, Branch {
			item,
			child: self.first_child
		});
		self.first_child = child_id
	}

	#[inline]
	pub fn pop_left(&mut self) -> Result<(Item<K, V>, usize), WouldUnderflow> {
		if self.item_count() < M/2 {
			Err(WouldUnderflow)
		} else {
			let child_id = self.first_child;
			let first = self.other_children.remove(0);
			self.first_child = first.child;
			Ok((first.item, child_id))
		}
	}

	#[inline]
	pub fn push_right(&mut self, item: Item<K, V>, child_id: usize) -> usize {
		let offset = self.other_children.len();
		self.other_children.push(Branch {
			item,
			child: child_id
		});
		offset
	}

	#[inline]
	pub fn pop_right(&mut self) -> Result<(usize, Item<K, V>, usize), WouldUnderflow> {
		if self.item_count() < M/2 {
			Err(WouldUnderflow)
		} else {
			let offset = self.other_children.len();
			let last = self.other_children.pop().unwrap();
			Ok((offset, last.item, last.child))
		}
	}

	#[inline]
	pub fn append(&mut self, separator: Item<K, V>, mut other: Internal<K, V>) -> usize {
		let offset = self.other_children.len();
		self.other_children.push(Branch {
			item: separator,
			child: other.first_child
		});

		self.other_children.append(&mut other.other_children);
		offset
	}

	/// Write the label of the internal node in the DOT format.
	///
	/// Requires the `dot` feature.
	#[cfg(feature = "dot")]
	#[inline]
	pub fn dot_write_label<W: std::io::Write>(&self, f: &mut W) -> std::io::Result<()> where K: std::fmt::Display, V: std::fmt::Display {
		write!(f, "<c0> |")?;
		let mut i = 1;
		for branch in &self.other_children {
			write!(f, "{{{}|<c{}> {}}} |", branch.item.key(), i, branch.item.value())?;
			i += 1;
		}

		Ok(())
	}

	#[cfg(debug_assertions)]
	pub fn validate(&self, parent: Option<usize>, min: Option<&K>, max: Option<&K>) where K: Ord {
		if self.parent() != parent {
			panic!("wrong parent")
		}

		if min.is_some() || max.is_some() { // not root
			if self.item_count() < (M/2 - 1) {
				panic!("internal node is underflowing")
			}

			if self.item_count() == M {
				panic!("internal node is overflowing")
			}
		} else {
			if self.item_count() == 0 {
				panic!("root node is empty")
			}
		}

		if !self.other_children.is_sorted() {
			panic!("internal node items are not sorted")
		}

		if let Some(min) = min {
			if let Some(b) = self.other_children.first() {
				if min >= b.item.key() {
					panic!("internal node item key is greater than right separator")
				}
			}
		}

		if let Some(max) = max {
			if let Some(b) = self.other_children.last() {
				if max <= b.item.key() {
					panic!("internal node item key is less than left separator")
				}
			}
		}
	}
}
