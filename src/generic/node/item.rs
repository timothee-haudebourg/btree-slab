use super::Keyed;
use std::{cmp::Ordering, mem::MaybeUninit};

pub struct Item<K, V> {
	/// # Safety
	///
	/// This field must always be initialized when the item is accessed and/or dropped.
	key: MaybeUninit<K>,

	/// # Safety
	///
	/// This field must always be initialized when the item is accessed and/or dropped.
	value: MaybeUninit<V>,
}

impl<K: Clone, V: Clone> Clone for Item<K, V> {
	fn clone(&self) -> Self {
		unsafe {
			Self::new(
				self.key.assume_init_ref().clone(),
				self.value.assume_init_ref().clone(),
			)
		}
	}
}

impl<K, V> AsRef<Item<K, V>> for Item<K, V> {
	fn as_ref(&self) -> &Self {
		self
	}
}

impl<K, V> Item<K, V> {
	pub fn new(key: K, value: V) -> Item<K, V> {
		Item {
			key: MaybeUninit::new(key),
			value: MaybeUninit::new(value),
		}
	}

	#[inline]
	pub fn key(&self) -> &K {
		unsafe { self.key.assume_init_ref() }
	}

	/// Modifying a key in such a way that its order with regard to other keys changes is a logical error.
	#[inline]
	pub fn key_mut(&mut self) -> &mut K {
		unsafe { self.key.assume_init_mut() }
	}

	#[inline]
	pub fn value(&self) -> &V {
		unsafe { self.value.assume_init_ref() }
	}

	#[inline]
	pub fn value_mut(&mut self) -> &mut V {
		unsafe { self.value.assume_init_mut() }
	}

	/// Modifying a key in such a way that its order with regard to other keys changes is a logical error.
	#[inline]
	pub fn set(&mut self, key: K, value: V) -> (K, V) {
		let mut old_key = MaybeUninit::new(key);
		let mut old_value = MaybeUninit::new(value);
		std::mem::swap(&mut old_key, &mut self.key);
		std::mem::swap(&mut old_value, &mut self.value);
		unsafe { (old_key.assume_init(), old_value.assume_init()) }
	}

	/// Modifying a key in such a way that its order with regard to other keys changes is a logical error.
	#[inline]
	pub fn set_key(&mut self, key: K) -> K {
		let mut old_key = MaybeUninit::new(key);
		std::mem::swap(&mut old_key, &mut self.key);
		unsafe { old_key.assume_init() }
	}

	#[inline]
	pub fn set_value(&mut self, value: V) -> V {
		let mut old_value = MaybeUninit::new(value);
		std::mem::swap(&mut old_value, &mut self.value);
		unsafe { old_value.assume_init() }
	}

	#[inline]
	pub fn maybe_uninit_value_mut(&mut self) -> &mut MaybeUninit<V> {
		&mut self.value
	}

	#[inline]
	pub fn into_key(self) -> K {
		let (key, value) = self.into_inner();
		unsafe {
			std::mem::drop(value.assume_init());
			key.assume_init()
		}
	}

	#[inline]
	pub fn into_value(self) -> V {
		let (key, value) = self.into_inner();
		unsafe {
			std::mem::drop(key.assume_init());
			value.assume_init()
		}
	}

	#[inline]
	pub fn as_pair(&self) -> (&K, &V) {
		unsafe { (self.key.assume_init_ref(), self.value.assume_init_ref()) }
	}

	#[inline]
	pub fn as_pair_mut(&mut self) -> (&mut K, &mut V) {
		unsafe { (self.key.assume_init_mut(), self.value.assume_init_mut()) }
	}

	#[inline]
	pub fn into_pair(self) -> (K, V) {
		let (key, value) = self.into_inner();
		unsafe { (key.assume_init(), value.assume_init()) }
	}

	/// Drop the key but not the value which is assumed uninitialized.
	/// 
	/// # Safety
	/// 
	/// The value must be uninitialized.
	#[inline]
	pub unsafe fn forget_value(self) {
		let (key, value) = self.into_inner();
		std::mem::drop(key.assume_init());
		std::mem::forget(value);
	}

	#[inline]
	pub fn into_inner(mut self) -> (MaybeUninit<K>, MaybeUninit<V>) {
		let mut key = MaybeUninit::uninit();
		let mut value = MaybeUninit::uninit();
		std::mem::swap(&mut key, &mut self.key);
		std::mem::swap(&mut value, &mut self.value);
		std::mem::forget(self);
		(key, value)
	}
}

impl<K, V> Drop for Item<K, V> {
	fn drop(&mut self) {
		unsafe {
			std::ptr::drop_in_place(self.key.assume_init_mut());
			std::ptr::drop_in_place(self.value.assume_init_mut());
		}
	}
}

impl<K, V> Keyed for Item<K, V> {
	type Key = K;

	#[inline]
	fn key(&self) -> &K {
		self.key()
	}
}

// impl<K, V, T: PartialEq<K>> PartialEq<T> for Item<K, V> {
// 	fn eq(&self, other: &T) -> bool {
// 		other.eq(self.key())
// 	}
// }

// impl<K, V, T: PartialOrd<K>> PartialOrd<T> for Item<K, V> {
// 	fn partial_cmp(&self, other: &T) -> Option<Ordering> {
// 		match other.partial_cmp(self.key()) {
// 			Some(Ordering::Greater) => Some(Ordering::Less),
// 			Some(Ordering::Less) => Some(Ordering::Greater),
// 			Some(Ordering::Equal) => Some(Ordering::Equal),
// 			None => None
// 		}
// 	}
// }

impl<K: PartialEq, V> PartialEq for Item<K, V> {
	fn eq(&self, other: &Item<K, V>) -> bool {
		self.key().eq(other.key())
	}
}

impl<K: Ord + PartialEq, V> PartialOrd for Item<K, V> {
	fn partial_cmp(&self, other: &Item<K, V>) -> Option<Ordering> {
		Some(self.key().cmp(other.key()))
	}
}
