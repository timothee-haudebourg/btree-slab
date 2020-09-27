use std::cmp::{
	PartialOrd,
	Ord,
	Ordering
};

#[derive(Clone, Copy)]
pub struct ItemAddr {
	pub id: usize,
	pub offset: usize
}

#[derive(Clone, Copy)]
pub struct Item<K, V> {
	pub key: K,
	pub value: V
}

impl<K, V> Item<K, V> {
	pub fn new(key: K, value: V) -> Item<K, V> {
		Item {
			key,
			value
		}
	}
}

impl<K: PartialEq, V> PartialEq<K> for Item<K, V> {
	fn eq(&self, key: &K) -> bool {
		self.key.eq(key)
	}
}

impl<K: Ord + PartialEq, V> PartialOrd<K> for Item<K, V> {
	fn partial_cmp(&self, key: &K) -> Option<Ordering> {
		Some(self.key.cmp(key))
	}
}

impl<K: PartialEq, V> PartialEq for Item<K, V> {
	fn eq(&self, other: &Item<K, V>) -> bool {
		self.key.eq(&other.key)
	}
}

impl<K: Ord + PartialEq, V> PartialOrd for Item<K, V> {
	fn partial_cmp(&self, other: &Item<K, V>) -> Option<Ordering> {
		Some(self.key.cmp(&other.key))
	}
}
