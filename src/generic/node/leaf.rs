use crate::{
	generic::{
		map::M,
		node::{Balance, Item, Offset, WouldUnderflow},
	},
	utils::binary_search_min,
};
use smallvec::SmallVec;
use std::borrow::Borrow;

#[derive(Clone)]
pub struct Leaf<K, V> {
	parent: usize,
	items: SmallVec<[Item<K, V>; M + 1]>,
}

impl<K, V> Leaf<K, V> {
	#[inline]
	pub fn new(parent: Option<usize>, item: Item<K, V>) -> Leaf<K, V> {
		let mut items = SmallVec::new();
		items.push(item);

		Leaf {
			parent: parent.unwrap_or(std::usize::MAX),
			items,
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
		self.items.len()
	}

	#[inline]
	pub fn items(&self) -> &[Item<K, V>] {
		self.items.as_ref()
	}

	#[inline]
	pub fn iter(&self) -> std::slice::Iter<Item<K, V>> {
		self.items.as_ref().iter()
	}

	#[inline]
	pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V>
	where
		K: Borrow<Q>,
		Q: Ord,
	{
		match binary_search_min(&self.items, key) {
			Some(i) => {
				let item = &self.items[i];
				if item.key().borrow() == key {
					Some(item.value())
				} else {
					None
				}
			}
			_ => None,
		}
	}

	#[inline]
	pub fn get_mut<Q: ?Sized>(&mut self, key: &Q) -> Option<&mut V>
	where
		K: Borrow<Q>,
		Q: Ord,
	{
		match binary_search_min(&self.items, key) {
			Some(i) => {
				let item = &mut self.items[i];
				if item.key().borrow() == key {
					Some(item.value_mut())
				} else {
					None
				}
			}
			_ => None,
		}
	}

	/// Find the offset of the item matching the given key.
	#[inline]
	pub fn offset_of<Q: ?Sized>(&self, key: &Q) -> Result<Offset, Offset>
	where
		K: Borrow<Q>,
		Q: Ord,
	{
		match binary_search_min(&self.items, key) {
			Some(i) => {
				if self.items[i].key().borrow() == key {
					Ok(i.into())
				} else {
					Err((i + 1).into())
				}
			}
			None => Err(0.into()),
		}
	}

	#[inline]
	pub fn item(&self, offset: Offset) -> Option<&Item<K, V>> {
		match offset.value() {
			Some(offset) => self.items.get(offset),
			None => None,
		}
	}

	#[inline]
	pub fn item_mut(&mut self, offset: Offset) -> Option<&mut Item<K, V>> {
		match offset.value() {
			Some(offset) => self.items.get_mut(offset),
			None => None,
		}
	}

	#[inline]
	pub fn insert_by_key(&mut self, key: K, mut value: V) -> (Offset, Option<V>)
	where
		K: Ord,
	{
		match binary_search_min(&self.items, &key) {
			Some(i) => {
				if self.items[i].key() == &key {
					std::mem::swap(&mut value, self.items[i].value_mut());
					(i.into(), Some(value))
				} else {
					self.items.insert(i + 1, Item::new(key, value));
					((i + 1).into(), None)
				}
			}
			None => {
				self.items.insert(0, Item::new(key, value));
				(0.into(), None)
			}
		}
	}

	#[inline]
	pub fn split(&mut self) -> (usize, Item<K, V>, Leaf<K, V>) {
		assert!(self.is_overflowing());

		let median_i = (self.items.len() - 1) / 2;

		let right_items = self.items.drain(median_i + 1..).collect();
		let median = self.items.pop().unwrap();

		let right_leaf = Leaf {
			parent: self.parent,
			items: right_items,
		};

		assert!(!self.is_underflowing());
		assert!(!right_leaf.is_underflowing());

		(self.items.len(), median, right_leaf)
	}

	#[inline]
	pub fn append(&mut self, separator: Item<K, V>, mut other: Leaf<K, V>) -> Offset {
		let offset = self.items.len();
		self.items.push(separator);
		self.items.append(&mut other.items);
		offset.into()
	}

	#[inline]
	pub fn push_left(&mut self, item: Item<K, V>) {
		self.items.insert(0, item)
	}

	#[inline]
	pub fn pop_left(&mut self) -> Result<Item<K, V>, WouldUnderflow> {
		if self.item_count() < M / 2 {
			Err(WouldUnderflow)
		} else {
			Ok(self.items.remove(0))
		}
	}

	#[inline]
	pub fn push_right(&mut self, item: Item<K, V>) -> Offset {
		let offset = self.items.len();
		self.items.push(item);
		offset.into()
	}

	#[inline]
	pub fn pop_right(&mut self) -> Result<(Offset, Item<K, V>), WouldUnderflow> {
		if self.item_count() < M / 2 {
			Err(WouldUnderflow)
		} else {
			let offset = self.items.len();
			let item = self.items.pop().unwrap();
			Ok((offset.into(), item))
		}
	}

	#[inline]
	pub fn balance(&self) -> Balance {
		if self.is_overflowing() {
			Balance::Overflow
		} else if self.is_underflowing() {
			Balance::Underflow(self.items.is_empty())
		} else {
			Balance::Balanced
		}
	}

	#[inline]
	pub fn is_overflowing(&self) -> bool {
		self.item_count() > M
	}

	#[inline]
	pub fn is_underflowing(&self) -> bool {
		self.item_count() < M / 2 - 1
	}

	/// It is assumed that the leaf will not overflow.
	#[inline]
	pub fn insert(&mut self, offset: Offset, item: Item<K, V>) {
		match offset.value() {
			Some(offset) => self.items.insert(offset, item),
			None => panic!("Offset out of bounds"),
		}
	}

	/// Remove the item at the given offset.
	/// Return the new balance of the leaf.
	#[inline]
	pub fn remove(&mut self, offset: Offset) -> Item<K, V> {
		match offset.value() {
			Some(offset) => self.items.remove(offset),
			None => panic!("Offset out of bounds"),
		}
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
	pub fn dot_write_label<W: std::io::Write>(&self, f: &mut W) -> std::io::Result<()>
	where
		K: std::fmt::Display,
		V: std::fmt::Display,
	{
		for item in &self.items {
			write!(f, "{{{}|{}}}|", item.key(), item.value())?;
		}

		Ok(())
	}

	#[cfg(debug_assertions)]
	pub fn validate(&self, parent: Option<usize>, min: Option<&K>, max: Option<&K>)
	where
		K: Ord,
	{
		if self.parent() != parent {
			panic!("wrong parent")
		}

		if min.is_some() || max.is_some() {
			// not root
			match self.balance() {
				Balance::Overflow => panic!("leaf is overflowing"),
				Balance::Underflow(_) => panic!("leaf is underflowing"),
				_ => (),
			}
		}

		if !self.items.windows(2).all(|w| w[0] < w[1]) {
			panic!("leaf items are not sorted")
		}

		if let Some(min) = min {
			if let Some(item) = self.items.first() {
				if min >= item.key() {
					panic!("leaf item key is greater than right separator")
				}
			}
		}

		if let Some(max) = max {
			if let Some(item) = self.items.last() {
				if max <= item.key() {
					panic!("leaf item key is less than left separator")
				}
			}
		}
	}
}
