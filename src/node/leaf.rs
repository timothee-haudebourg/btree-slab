use staticvec::StaticVec;
use crate::{
	Item,
	Balance,
	WouldUnderflow,
	utils::binary_search_min
};

pub struct Leaf<K, V, const M: usize> {
	items: StaticVec<Item<K, V>, M>
}

impl<K, V, const M: usize> Leaf<K, V, M> {
	#[inline]
	pub fn new(item: Item<K, V>) -> Leaf<K, V, M> {
		let mut items = StaticVec::new();
		items.push(item);

		Leaf {
			items
		}
	}

	#[inline]
	pub fn item_count(&self) -> usize {
		self.items.len()
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
	pub fn insert(&mut self, key: K, mut value: V) -> Option<V> where K: Ord {
		match binary_search_min(&self.items, &key) {
			Some(i) => {
				if self.items[i].key == key {
					std::mem::swap(&mut value, &mut self.items[i].value);
					Some(value)
				} else {
					self.items.insert(i + 1, Item { key, value });
					None
				}
			},
			None => {
				self.items.insert(0, Item { key, value });
				None
			}
		}
	}

	#[inline]
	pub fn split(&mut self) -> Result<(Item<K, V>, Leaf<K, V, M>), ()> {
		if self.items.len() < M {
			Err(()) // We don't need to split.
		} else {
			let median_i = M / 2;

			let right_items = self.items.drain(median_i+1..);
			let median = self.items.pop().unwrap();

			let right_leaf = Leaf {
				items: right_items
			};

			Ok((median, right_leaf))
		}
	}

	#[inline]
	pub fn append(&mut self, separator: Item<K, V>, mut other: Leaf<K, V, M>) {
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
	fn balance(&self) -> Balance {
		if self.item_count() < M/2 - 1 {
			Balance::Underflow(self.items.is_empty())
		} else {
			Balance::Balanced
		}
	}

	#[inline]
	pub fn take(&mut self, offset: usize) -> (Item<K, V>, Balance) {
		let item = self.items.remove(offset);
		(item, self.balance())
	}

	#[inline]
	pub fn take_last(&mut self) -> (Item<K, V>, Balance) {
		let item = self.items.pop().unwrap();
		(item, self.balance())
	}
}
