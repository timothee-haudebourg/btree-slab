use std::cmp::{
	PartialOrd,
	Ord,
	Ordering
};

pub struct Item<K, V> {
	pub key: K,
	pub value: V
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
