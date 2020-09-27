use staticvec::StaticVec;
use crate::{
	M,
	Item,
	Balance,
	WouldUnderflow,
	utils::binary_search_min
};

pub struct Leaf<K, V> {
	parent: usize,
	items: StaticVec<Item<K, V>, {M+1}>
}

impl<K, V> Leaf<K, V> {
	#[inline]
	pub fn new(parent: Option<usize>, item: Item<K, V>) -> Leaf<K, V> {
		let mut items = StaticVec::new();
		items.push(item);

		Leaf {
			parent: parent.unwrap_or(std::usize::MAX),
			items
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
		let mut len = self.items.len();
		len
	}

	#[inline]
	pub fn items(&self) -> std::slice::Iter<Item<K, V>> {
		self.items.as_ref().iter()
	}

	#[inline]
	pub fn get(&self, key: &K) -> Option<&V> where K: Ord {
		match binary_search_min(&self.items, key) {
			Some(i) => {
				let item = &self.items[i];
				if &item.key == key {
					Some(&item.value)
				} else {
					None
				}
			},
			_ => None
		}
	}

	#[inline]
	pub fn get_mut(&mut self, key: &K) -> Option<&mut V> where K: Ord {
		match binary_search_min(&self.items, key) {
			Some(i) => {
				let item = &mut self.items[i];
				if &item.key == key {
					Some(&mut item.value)
				} else {
					None
				}
			},
			_ => None
		}
	}

	/// Find the offset of the item matching the given key.
	#[inline]
	pub fn offset_of(&self, key: &K) -> Option<usize> where K: Ord {
		match binary_search_min(&self.items, key) {
			Some(i) if &self.items[i].key == key => Some(i),
			_ => None
		}
	}

	#[inline]
	pub fn item_at_mut(&mut self, offset: usize) -> &mut Item<K, V> {
		&mut self.items[offset]
	}

	#[inline]
	pub fn item_at_mut_opt(&mut self, offset: usize) -> Option<&mut Item<K, V>> {
		self.items.get_mut(offset)
	}

	#[inline]
	pub fn insert_by_key(&mut self, key: K, mut value: V) -> (usize, Option<V>) where K: Ord {
		match binary_search_min(&self.items, &key) {
			Some(i) => {
				if self.items[i].key == key {
					std::mem::swap(&mut value, &mut self.items[i].value);
					(i, Some(value))
				} else {
					self.items.insert(i+1, Item { key, value });
					(i+1, None)
				}
			},
			None => {
				self.items.insert(0, Item { key, value });
				(0, None)
			}
		}
	}

	#[inline]
	pub fn split(&mut self) -> (usize, Item<K, V>, Leaf<K, V>) {
		let median_i = M / 2;

		let right_items = self.items.drain(median_i+1..);
		let median = self.items.pop().unwrap();

		let right_leaf = Leaf {
			parent: self.parent,
			items: right_items
		};

		(self.items.len(), median, right_leaf)
	}

	#[inline]
	pub fn append(&mut self, separator: Item<K, V>, mut other: Leaf<K, V>) {
		self.items.push(separator);
		self.items.append(&mut other.items);
	}

	#[inline]
	pub fn push_left(&mut self, item: Item<K, V>) {
		self.items.insert(0, item)
	}

	#[inline]
	pub fn pop_left(&mut self) -> Result<Item<K, V>, WouldUnderflow> {
		if self.item_count() < M/2 {
			Err(WouldUnderflow)
		} else {
			Ok(self.items.remove(0))
		}
	}

	#[inline]
	pub fn push_right(&mut self, item: Item<K, V>) {
		self.items.push(item)
	}

	#[inline]
	pub fn pop_right(&mut self) -> Result<Item<K, V>, WouldUnderflow> {
		if self.item_count() < M/2 {
			Err(WouldUnderflow)
		} else {
			Ok(self.items.pop().unwrap())
		}
	}

	#[inline]
	pub fn balance(&self) -> Balance {
		if self.item_count() > M {
			Balance::Overflow
		} else if self.item_count() < M/2 - 1 {
			Balance::Underflow(self.items.is_empty())
		} else {
			Balance::Balanced
		}
	}

	/// It is assumed that the leaf will not overflow.
	#[inline]
	pub fn insert(&mut self, offset: usize, item: Item<K, V>) {
		self.items.insert(offset, item)
	}

	/// Remove the item at the given offset.
	/// Return the new balance of the leaf.
	#[inline]
	pub fn remove(&mut self, offset: usize) -> Item<K, V> {
		self.items.remove(offset)
	}

	#[inline]
	pub fn remove_last(&mut self) -> Item<K, V> {
		self.items.pop().unwrap()
	}

	/// Write the label of the leaf in the DOT language.
	///
	/// Requires the `dot` feature.
	#[cfg(feature = "dot")]
	#[inline]
	pub fn dot_write_label<W: std::io::Write>(&self, f: &mut W) -> std::io::Result<()> where K: std::fmt::Display, V: std::fmt::Display {
		for item in &self.items {
			write!(f, "{{{}|{}}}|", item.key, item.value)?;
		}

		Ok(())
	}

	#[cfg(debug_assertions)]
	pub fn validate(&self, parent: Option<usize>, min: Option<&K>, max: Option<&K>) where K: Ord {
		if self.parent() != parent {
			panic!("wrong parent")
		}

		if min.is_some() || max.is_some() { // not root
			match self.balance() {
				Balance::Overflow => panic!("leaf is overflowing"),
				Balance::Underflow(_) => panic!("leaf is underflowing"),
				_ => ()
			}
		}

		if !self.items.is_sorted() {
			panic!("leaf items are not sorted")
		}

		if let Some(min) = min {
			if let Some(item) = self.items.first() {
				if min >= &item.key {
					panic!("leaf item key is greater than right separator")
				}
			}
		}

		if let Some(max) = max {
			if let Some(item) = self.items.last() {
				if max <= &item.key {
					panic!("leaf item key is less than left separator")
				}
			}
		}
	}
}
