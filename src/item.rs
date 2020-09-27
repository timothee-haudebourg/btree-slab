use std::{
	mem::MaybeUninit,
	cmp::{
		PartialOrd,
		Ord,
		Ordering
	}
};

#[derive(Clone, Copy)]
pub struct ItemAddr {
	pub id: usize,
	pub offset: usize
}

pub struct Item<K, V> {
	key: MaybeUninit<K>,

	/// # Safety
	///
	/// This field must always be initialized when the item is accessed and/or dropped.
	value: MaybeUninit<V>
}

impl<K, V> Item<K, V> {
	pub fn new(key: K, value: V) -> Item<K, V> {
		Item {
			key: MaybeUninit::new(key),
			value: MaybeUninit::new(value)
		}
	}

	#[inline]
	pub fn key(&self) -> &K {
		unsafe { self.key.assume_init_ref() }
	}

	#[inline]
	pub fn value(&self) -> &V {
		unsafe { self.value.assume_init_ref() }
	}

	#[inline]
	pub fn value_mut(&mut self) -> &mut V {
		unsafe { self.value.assume_init_mut() }
	}

	#[inline]
	pub unsafe fn maybe_uninit_value_mut(&mut self) -> &mut MaybeUninit<V> {
		&mut self.value
	}

	#[inline]
	pub fn into_value(self) -> V {
		let (key, value) = self.into_inner();
		unsafe {
			std::mem::drop(key.assume_init());
			value.assume_init()
		}
	}

	/// Drop the key but not the value which is assumed uninitialized.
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

impl<K: PartialEq, V> PartialEq<K> for Item<K, V> {
	fn eq(&self, key: &K) -> bool {
		self.key().eq(key)
	}
}

impl<K: Ord + PartialEq, V> PartialOrd<K> for Item<K, V> {
	fn partial_cmp(&self, key: &K) -> Option<Ordering> {
		Some(self.key().cmp(key))
	}
}

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
