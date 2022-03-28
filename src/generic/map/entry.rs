use crate::generic::{
	map::{BTreeExt, BTreeExtMut, BTreeMap},
	node::{Address, Item, Node},
};
use cc_traits::{Slab, SlabMut};
use std::fmt;

/// A view into a single entry in a map, which may either be vacant or occupied.
///
/// This enum is constructed from the [`entry`](`BTreeMap#entry`) method on [`BTreeMap`].
pub enum Entry<'a, K, V, C = slab::Slab<Node<K, V>>> {
	Vacant(VacantEntry<'a, K, V, C>),
	Occupied(OccupiedEntry<'a, K, V, C>),
}

use Entry::*;

impl<'a, K, V, C: Slab<Node<K, V>>> Entry<'a, K, V, C>
where
	for<'r> C::ItemRef<'r>: Into<&'r Node<K, V>>,
{
	/// Gets the address of the entry in the B-Tree.
	#[inline]
	pub fn address(&self) -> Address {
		match self {
			Occupied(entry) => entry.address(),
			Vacant(entry) => entry.address(),
		}
	}

	/// Returns a reference to this entry's key.
	///
	/// # Examples
	///
	/// ```
	/// use btree_slab::BTreeMap;
	///
	/// let mut map: BTreeMap<&str, usize> = BTreeMap::new();
	/// assert_eq!(map.entry("poneyland").key(), &"poneyland");
	/// ```
	#[inline]
	pub fn key(&self) -> &K {
		match *self {
			Occupied(ref entry) => entry.key(),
			Vacant(ref entry) => entry.key(),
		}
	}
}

impl<'a, K, V, C: SlabMut<Node<K, V>>> Entry<'a, K, V, C>
where
	for<'r> C::ItemRef<'r>: Into<&'r Node<K, V>>,
	for<'r> C::ItemMut<'r>: Into<&'r mut Node<K, V>>,
{
	/// Ensures a value is in the entry by inserting the default if empty, and returns
	/// a mutable reference to the value in the entry.
	///
	/// # Examples
	///
	/// ```
	/// use btree_slab::BTreeMap;
	///
	/// let mut map: BTreeMap<&str, usize> = BTreeMap::new();
	/// map.entry("poneyland").or_insert(12);
	///
	/// assert_eq!(map["poneyland"], 12);
	/// ```
	#[inline]
	pub fn or_insert(self, default: V) -> &'a mut V {
		match self {
			Occupied(entry) => entry.into_mut(),
			Vacant(entry) => entry.insert(default),
		}
	}

	/// Ensures a value is in the entry by inserting the result of the default function if empty,
	/// and returns a mutable reference to the value in the entry.
	///
	/// # Examples
	///
	/// ```
	/// use btree_slab::BTreeMap;
	///
	/// let mut map: BTreeMap<&str, String> = BTreeMap::new();
	/// let s = "hoho".to_string();
	///
	/// map.entry("poneyland").or_insert_with(|| s);
	///
	/// assert_eq!(map["poneyland"], "hoho".to_string());
	/// ```
	#[inline]
	pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V {
		match self {
			Occupied(entry) => entry.into_mut(),
			Vacant(entry) => entry.insert(default()),
		}
	}

	/// Ensures a value is in the entry by inserting, if empty, the result of the default function,
	/// which takes the key as its argument, and returns a mutable reference to the value in the
	/// entry.
	///
	/// # Examples
	///
	/// ```
	/// #![feature(or_insert_with_key)]
	/// use btree_slab::BTreeMap;
	///
	/// let mut map: BTreeMap<&str, usize> = BTreeMap::new();
	///
	/// map.entry("poneyland").or_insert_with_key(|key| key.chars().count());
	///
	/// assert_eq!(map["poneyland"], 9);
	/// ```
	#[inline]
	pub fn or_insert_with_key<F: FnOnce(&K) -> V>(self, default: F) -> &'a mut V {
		match self {
			Occupied(entry) => entry.into_mut(),
			Vacant(entry) => {
				let value = default(entry.key());
				entry.insert(value)
			}
		}
	}

	/// Provides in-place mutable access to an occupied entry before any
	/// potential inserts into the map.
	///
	/// # Examples
	///
	/// ```
	/// use btree_slab::BTreeMap;
	///
	/// let mut map: BTreeMap<&str, usize> = BTreeMap::new();
	///
	/// map.entry("poneyland")
	///    .and_modify(|e| { *e += 1 })
	///    .or_insert(42);
	/// assert_eq!(map["poneyland"], 42);
	///
	/// map.entry("poneyland")
	///    .and_modify(|e| { *e += 1 })
	///    .or_insert(42);
	/// assert_eq!(map["poneyland"], 43);
	/// ```
	#[inline]
	pub fn and_modify<F>(self, f: F) -> Self
	where
		F: FnOnce(&mut V),
	{
		match self {
			Occupied(mut entry) => {
				f(entry.get_mut());
				Occupied(entry)
			}
			Vacant(entry) => Vacant(entry),
		}
	}

	/// Ensures a value is in the entry by inserting the default value if empty,
	/// and returns a mutable reference to the value in the entry.
	///
	/// # Examples
	///
	/// ```
	/// use btree_slab::BTreeMap;
	///
	/// let mut map: BTreeMap<&str, Option<usize>> = BTreeMap::new();
	/// map.entry("poneyland").or_default();
	///
	/// assert_eq!(map["poneyland"], None);
	/// ```
	#[inline]
	pub fn or_default(self) -> &'a mut V
	where
		V: Default,
	{
		match self {
			Occupied(entry) => entry.into_mut(),
			Vacant(entry) => entry.insert(Default::default()),
		}
	}
}

impl<'a, K: fmt::Debug, V: fmt::Debug, C: Slab<Node<K, V>>> fmt::Debug for Entry<'a, K, V, C>
where
	for<'r> C::ItemRef<'r>: Into<&'r Node<K, V>>,
{
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Occupied(entry) => entry.fmt(f),
			Vacant(entry) => entry.fmt(f),
		}
	}
}

/// A view into a vacant entry in a [`BTreeMap`].
/// It is part of the [`Entry`] enum.
pub struct VacantEntry<'a, K, V, C = slab::Slab<Node<K, V>>> {
	pub(crate) map: &'a mut BTreeMap<K, V, C>,
	pub(crate) key: K,
	pub(crate) addr: Address,
}

impl<'a, K, V, C: Slab<Node<K, V>>> VacantEntry<'a, K, V, C> {
	/// Gets the address of the vacant entry in the B-Tree.
	#[inline]
	pub fn address(&self) -> Address {
		self.addr
	}

	/// Gets a reference to the keys that would be used when inserting a value through the `VacantEntry`.
	///
	/// ## Example
	/// ```
	/// use btree_slab::BTreeMap;
	///
	/// let mut map: BTreeMap<&str, usize> = BTreeMap::new();
	/// assert_eq!(map.entry("poneyland").key(), &"poneyland");
	/// ```
	#[inline]
	pub fn key(&self) -> &K {
		&self.key
	}

	/// Take ownership of the key.
	///
	/// ## Example
	/// ```
	/// use btree_slab::BTreeMap;
	/// use btree_slab::generic::map::Entry;
	///
	/// let mut map: BTreeMap<&str, usize> = BTreeMap::new();
	///
	/// if let Entry::Vacant(v) = map.entry("poneyland") {
	///     v.into_key();
	/// }
	/// ```
	#[inline]
	pub fn into_key(self) -> K {
		self.key
	}
}

impl<'a, K, V, C: SlabMut<Node<K, V>>> VacantEntry<'a, K, V, C>
where
	for<'r> C::ItemRef<'r>: Into<&'r Node<K, V>>,
	for<'r> C::ItemMut<'r>: Into<&'r mut Node<K, V>>,
{
	/// Sets the value of the entry with the `VacantEntry`'s key,
	/// and returns a mutable reference to it.
	///
	/// ## Example
	/// ```
	/// use btree_slab::BTreeMap;
	/// use btree_slab::generic::map::Entry;
	///
	/// let mut map: BTreeMap<&str, u32> = BTreeMap::new();
	///
	/// if let Entry::Vacant(o) = map.entry("poneyland") {
	///     o.insert(37);
	/// }
	/// assert_eq!(map["poneyland"], 37);
	/// ```
	#[inline]
	pub fn insert(self, value: V) -> &'a mut V {
		let addr = self.map.insert_at(self.addr, Item::new(self.key, value));
		self.map.item_mut(addr).unwrap().value_mut()
	}
}

impl<'a, K: fmt::Debug, V, C: Slab<Node<K, V>>> fmt::Debug for VacantEntry<'a, K, V, C> {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_tuple("VacantEntry").field(self.key()).finish()
	}
}

/// A view into an occupied entry in a [`BTreeMap`].
/// It is part of the [`Entry`] enum.
pub struct OccupiedEntry<'a, K, V, C = slab::Slab<Node<K, V>>> {
	pub(crate) map: &'a mut BTreeMap<K, V, C>,
	pub(crate) addr: Address,
}

impl<'a, K, V, C: Slab<Node<K, V>>> OccupiedEntry<'a, K, V, C>
where
	for<'r> C::ItemRef<'r>: Into<&'r Node<K, V>>,
{
	/// Gets the address of the occupied entry in the B-Tree.
	#[inline]
	pub fn address(&self) -> Address {
		self.addr
	}

	/// Gets a reference to the value in the entry.
	///
	/// # Example
	/// ```
	/// use btree_slab::BTreeMap;
	/// use btree_slab::generic::map::Entry;
	///
	/// let mut map: BTreeMap<&str, usize> = BTreeMap::new();
	/// map.entry("poneyland").or_insert(12);
	///
	/// if let Entry::Occupied(o) = map.entry("poneyland") {
	///     assert_eq!(o.get(), &12);
	/// }
	/// ```
	#[inline]
	pub fn get(&self) -> &V {
		self.map.item(self.addr).unwrap().value()
	}

	/// Gets a reference to the key in the entry.
	///
	/// # Example
	/// ```
	/// use btree_slab::BTreeMap;
	///
	/// let mut map: BTreeMap<&str, usize> = BTreeMap::new();
	/// map.entry("poneyland").or_insert(12);
	/// assert_eq!(map.entry("poneyland").key(), &"poneyland");
	/// ```
	#[inline]
	pub fn key(&self) -> &K {
		self.map.item(self.addr).unwrap().key()
	}
}

impl<'a, K, V, C: SlabMut<Node<K, V>>> OccupiedEntry<'a, K, V, C>
where
	for<'r> C::ItemRef<'r>: Into<&'r Node<K, V>>,
	for<'r> C::ItemMut<'r>: Into<&'r mut Node<K, V>>,
{
	/// Gets a mutable reference to the value in the entry.
	///
	/// If you need a reference to the OccupiedEntry that may outlive
	/// the destruction of the Entry value, see into_mut.
	///
	/// # Example
	/// ```
	/// use btree_slab::BTreeMap;
	/// use btree_slab::generic::map::Entry;
	///
	/// let mut map: BTreeMap<&str, usize> = BTreeMap::new();
	/// map.entry("poneyland").or_insert(12);
	///
	/// assert_eq!(map["poneyland"], 12);
	/// if let Entry::Occupied(mut o) = map.entry("poneyland") {
	///     *o.get_mut() += 10;
	///     assert_eq!(*o.get(), 22);
	///
	///     // We can use the same Entry multiple times.
	///     *o.get_mut() += 2;
	/// }
	/// assert_eq!(map["poneyland"], 24);
	/// ```
	#[inline]
	pub fn get_mut(&mut self) -> &mut V {
		self.map.item_mut(self.addr).unwrap().value_mut()
	}

	/// Sets the value of the entry with the OccupiedEntry's key,
	/// and returns the entry's old value.
	///
	/// # Example
	/// ```
	/// use btree_slab::BTreeMap;
	/// use btree_slab::generic::map::Entry;
	///
	/// let mut map: BTreeMap<&str, usize> = BTreeMap::new();
	/// map.entry("poneyland").or_insert(12);
	///
	/// if let Entry::Occupied(mut o) = map.entry("poneyland") {
	///     assert_eq!(o.insert(15), 12);
	/// }
	/// assert_eq!(map["poneyland"], 15);
	/// ```
	#[inline]
	pub fn insert(&mut self, value: V) -> V {
		self.map.item_mut(self.addr).unwrap().set_value(value)
	}

	/// Converts the entry into a mutable reference to its value.
	///
	/// If you need multiple references to the `OccupiedEntry`, see [`get_mut`].
	///
	/// [`get_mut`]: #method.get_mut
	///
	/// # Example
	///
	/// ```
	/// use btree_slab::BTreeMap;
	/// use btree_slab::generic::map::Entry;
	///
	/// let mut map: BTreeMap<&str, usize> = BTreeMap::new();
	/// map.entry("poneyland").or_insert(12);
	///
	/// assert_eq!(map["poneyland"], 12);
	/// if let Entry::Occupied(o) = map.entry("poneyland") {
	///     *o.into_mut() += 10;
	/// }
	/// assert_eq!(map["poneyland"], 22);
	/// ```
	#[inline]
	pub fn into_mut(self) -> &'a mut V {
		self.map.item_mut(self.addr).unwrap().value_mut()
	}

	/// Takes the value of the entry out of the map, and returns it.
	///
	/// # Examples
	///
	/// ```
	/// use btree_slab::BTreeMap;
	/// use btree_slab::generic::map::Entry;
	///
	/// let mut map: BTreeMap<&str, usize> = BTreeMap::new();
	/// map.entry("poneyland").or_insert(12);
	///
	/// if let Entry::Occupied(o) = map.entry("poneyland") {
	///     assert_eq!(o.remove(), 12);
	/// }
	/// // If we try to get "poneyland"'s value, it'll panic:
	/// // println!("{}", map["poneyland"]);
	/// ```
	#[inline]
	pub fn remove(self) -> V {
		self.map.remove_at(self.addr).unwrap().0.into_value()
	}

	/// Take ownership of the key and value from the map.
	///
	/// # Example
	/// ```
	/// use btree_slab::BTreeMap;
	/// use btree_slab::generic::map::Entry;
	///
	/// let mut map: BTreeMap<&str, usize> = BTreeMap::new();
	/// map.entry("poneyland").or_insert(12);
	///
	/// if let Entry::Occupied(o) = map.entry("poneyland") {
	///     // We delete the entry from the map.
	///     o.remove_entry();
	/// }
	///
	/// // If now try to get the value, it will panic:
	/// // println!("{}", map["poneyland"]);
	/// ```
	#[inline]
	pub fn remove_entry(self) -> (K, V) {
		self.map.remove_at(self.addr).unwrap().0.into_pair()
	}
}

impl<'a, K: fmt::Debug, V: fmt::Debug, C: Slab<Node<K, V>>> fmt::Debug
	for OccupiedEntry<'a, K, V, C>
where
	for<'r> C::ItemRef<'r>: Into<&'r Node<K, V>>,
{
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("OccupiedEntry")
			.field("key", self.key())
			.field("value", self.get())
			.finish()
	}
}
