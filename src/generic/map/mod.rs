use std::{
	borrow::Borrow,
	marker::PhantomData,
	ops::{
		Index,
		Bound,
		RangeBounds
	},
	cmp::Ordering,
	hash::{
		Hash,
		Hasher
	}
};
use crate::{
	generic::{
		node::{
			Item,
			ItemAddr,
			Node,
			Balance,
			WouldUnderflow
		}
	},
	Container,
	ContainerMut
};

mod ext;
mod entry;

pub use ext::*;
pub use entry::*;

/// Knuth order of the B-Trees.
///
/// Must be at least 4.
pub const M: usize = 8;

/// A map based on a B-Tree.
///
/// This offers an alternative over the standard implementation of B-Trees where nodes are
/// allocated in a contiguous array of [`Node`]s, reducing the cost of tree nodes allocations.
/// In addition the crate provides advanced functions to iterate through and update the map
/// efficiently.
///
/// # Basic usage
/// Basic usage is similar to the map data structures offered by the standard library.
/// ```
/// use local_btree::BTreeMap;
///
/// // type inference lets us omit an explicit type signature (which
/// // would be `BTreeMap<&str, &str>` in this example).
/// let mut movie_reviews = BTreeMap::new();
///
/// // review some movies.
/// movie_reviews.insert("Office Space",       "Deals with real issues in the workplace.");
/// movie_reviews.insert("Pulp Fiction",       "Masterpiece.");
/// movie_reviews.insert("The Godfather",      "Very enjoyable.");
/// movie_reviews.insert("The Blues Brothers", "Eye lyked it a lot.");
///
/// // check for a specific one.
/// if !movie_reviews.contains_key("Les Misérables") {
///     println!("We've got {} reviews, but Les Misérables ain't one.",
///              movie_reviews.len());
/// }
///
/// // oops, this review has a lot of spelling mistakes, let's delete it.
/// movie_reviews.remove("The Blues Brothers");
///
/// // look up the values associated with some keys.
/// let to_find = ["Up!", "Office Space"];
/// for movie in &to_find {
///     match movie_reviews.get(movie) {
///        Some(review) => println!("{}: {}", movie, review),
///        None => println!("{} is unreviewed.", movie)
///     }
/// }
///
/// // Look up the value for a key (will panic if the key is not found).
/// println!("Movie review: {}", movie_reviews["Office Space"]);
///
/// // iterate over everything.
/// for (movie, review) in &movie_reviews {
///     println!("{}: \"{}\"", movie, review);
/// }
/// ```
///
/// # Entry API
/// This crate also reproduces the Entry API defined by the standard library,
/// which allows for more complex methods of getting, setting, updating and removing keys and
/// their values:
/// ```
/// use local_btree::BTreeMap;
///
/// // type inference lets us omit an explicit type signature (which
/// // would be `BTreeMap<&str, u8>` in this example).
/// let mut player_stats: BTreeMap<&str, u8> = BTreeMap::new();
///
/// fn random_stat_buff() -> u8 {
///     // could actually return some random value here - let's just return
///     // some fixed value for now
///     42
/// }
///
/// // insert a key only if it doesn't already exist
/// player_stats.entry("health").or_insert(100);
///
/// // insert a key using a function that provides a new value only if it
/// // doesn't already exist
/// player_stats.entry("defence").or_insert_with(random_stat_buff);
///
/// // update a key, guarding against the key possibly not being set
/// let stat = player_stats.entry("attack").or_insert(100);
/// *stat += random_stat_buff();
/// ```
///
/// # Inplace iterator modification
/// Bar.
///
/// # Correctness
/// It is a logic error for a key to be modified in such a way that the key's ordering relative
/// to any other key, as determined by the [`Ord`] trait, changes while it is in the map.
/// This is normally only possible through [`Cell`](`std::cell::Cell`),
/// [`RefCell`](`std::cell::RefCell`), global state, I/O, or unsafe code.
#[derive(Clone)]
pub struct BTreeMap<K, V, C> {
	/// Allocated and free nodes.
	nodes: C,

	/// Root node id.
	root: Option<usize>,

	/// Number of items in the tree.
	len: usize,

	k: PhantomData<K>,
	v: PhantomData<V>
}

impl<K, V, C> BTreeMap<K, V, C> {
	/// Create a new empty B-tree.
	pub fn new() -> BTreeMap<K, V, C> where C: Default {
		assert!(M >= 4);
		BTreeMap {
			nodes: Default::default(),
			root: None,
			len: 0,
			k: PhantomData,
			v: PhantomData
		}
	}

	#[inline]
	pub fn is_empty(&self) -> bool {
		self.root.is_none()
	}

	#[inline]
	pub fn len(&self) -> usize {
		self.len
	}
}

impl<K, V, C: Container<Node<K, V>>> BTreeMap<K, V, C> {
	/// Returns the key-value pair corresponding to the supplied key.
	///
	/// The supplied key may be any borrowed form of the map's key type, but the ordering
	/// on the borrowed form *must* match the ordering on the key type.
	///
	/// # Example
	///
	/// ```
	/// use local_btree::BTreeMap;
	///
	/// let mut map: BTreeMap<i32, &str> = BTreeMap::new();
	/// map.insert(1, "a");
	/// assert_eq!(map.get_key_value(&1), Some((&1, &"a")));
	/// assert_eq!(map.get_key_value(&2), None);
	/// ```
	#[inline]
	pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V> where K: Borrow<Q>, Q: Ord {
		match self.root {
			Some(id) => self.get_in(key, id),
			None => None
		}
	}

	/// Returns the key-value pair corresponding to the supplied key.
	///
	/// The supplied key may be any borrowed form of the map's key type, but the ordering
	/// on the borrowed form *must* match the ordering on the key type.
	///
	/// # Examples
	///
	/// ```
	/// use local_btree::BTreeMap;
	///
	/// let mut map = BTreeMap::new();
	/// map.insert(1, "a");
	/// assert_eq!(map.get_key_value(&1), Some((&1, &"a")));
	/// assert_eq!(map.get_key_value(&2), None);
	/// ```
	pub fn get_key_value<Q: ?Sized>(&self, k: &Q) -> Option<(&K, &V)>
	where
		K: Borrow<Q>,
		Q: Ord,
	{
		match self.address_of(k) {
			Ok(addr) => {
				let item = self.item(addr).unwrap();
				Some((item.key(), item.value()))
			},
			Err(_) => None
		}
	}

	/// Returns the first key-value pair in the map.
	/// The key in this pair is the minimum key in the map.
	///
	/// # Example
	///
	/// ```
	/// use local_btree::BTreeMap;
	///
	/// let mut map = BTreeMap::new();
	/// assert_eq!(map.first_key_value(), None);
	/// map.insert(1, "b");
	/// map.insert(2, "a");
	/// assert_eq!(map.first_key_value(), Some((&1, &"b")));
	/// ```
	pub fn first_key_value(&self) -> Option<(&K, &V)> {
		match self.first_item_address() {
			Some(addr) => {
				let item = self.item(addr).unwrap();
				Some((item.key(), item.value()))
			},
			None => None
		}
	}

	/// Returns the last key-value pair in the map.
	/// The key in this pair is the maximum key in the map.
	///
	/// # Examples
	///
	/// Basic usage:
	///
	/// ```
	/// use local_btree::BTreeMap;
	///
	/// let mut map = BTreeMap::new();
	/// map.insert(1, "b");
	/// map.insert(2, "a");
	/// assert_eq!(map.last_key_value(), Some((&2, &"a")));
	/// ```
	pub fn last_key_value(&self) -> Option<(&K, &V)> {
		match self.last_item_address() {
			Some(addr) => {
				let item = self.item(addr).unwrap();
				Some((item.key(), item.value()))
			},
			None => None
		}
	}

	#[inline]
	pub fn iter(&self) -> Iter<K, V, C> {
		Iter::new(self)
	}

	/// Constructs a double-ended iterator over a sub-range of elements in the map.
	/// The simplest way is to use the range syntax `min..max`, thus `range(min..max)` will
	/// yield elements from min (inclusive) to max (exclusive).
	/// The range may also be entered as `(Bound<T>, Bound<T>)`, so for example
	/// `range((Excluded(4), Included(10)))` will yield a left-exclusive, right-inclusive
	/// range from 4 to 10.
	///
	/// # Panics
	///
	/// Panics if range `start > end`.
	/// Panics if range `start == end` and both bounds are `Excluded`.
	///
	/// # Example
	///
	/// ```
	/// use local_btree::BTreeMap;
	/// use std::ops::Bound::Included;
	///
	/// let mut map = BTreeMap::new();
	/// map.insert(3, "a");
	/// map.insert(5, "b");
	/// map.insert(8, "c");
	/// for (&key, &value) in map.range((Included(&4), Included(&8))) {
	///     println!("{}: {}", key, value);
	/// }
	/// assert_eq!(Some((&5, &"b")), map.range(4..).next());
	/// ```
	pub fn range<T: ?Sized, R>(&self, range: R) -> Range<K, V, C>
	where
		T: Ord,
		K: Borrow<T>,
		R: RangeBounds<T>,
	{
		Range::new(self, range)
	}

	/// Gets an iterator over the keys of the map, in sorted order.
	///
	/// # Example
	///
	/// ```
	/// use local_btree::BTreeMap;
	///
	/// let mut a = BTreeMap::new();
	/// a.insert(2, "b");
	/// a.insert(1, "a");
	///
	/// let keys: Vec<_> = a.keys().cloned().collect();
	/// assert_eq!(keys, [1, 2]);
	/// ```
	pub fn keys(&self) -> Keys<K, V, C> {
		Keys { inner: self.iter() }
	}

	/// Gets an iterator over the values of the map, in order by key.
	///
	/// # Example
	///
	/// ```
	/// use local_btree::BTreeMap;
	///
	/// let mut a = BTreeMap::new();
	/// a.insert(1, "hello");
	/// a.insert(2, "goodbye");
	///
	/// let values: Vec<&str> = a.values().cloned().collect();
	/// assert_eq!(values, ["hello", "goodbye"]);
	/// ```
	pub fn values(&self) -> Values<K, V, C> {
		Values { inner: self.iter() }
	}

	/// Returns `true` if the map contains a value for the specified key.
	///
	/// The key may be any borrowed form of the map's key type, but the ordering
	/// on the borrowed form *must* match the ordering on the key type.
	///
	/// # Example
	/// ```
	/// use local_btree::BTreeMap;
	///
	/// let mut map: BTreeMap<i32, &str> = BTreeMap::new();
	/// map.insert(1, "a");
	/// assert_eq!(map.contains_key(&1), true);
	/// assert_eq!(map.contains_key(&2), false);
	/// ```
	#[inline]
	pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool where K: Borrow<Q>, Q: Ord {
		self.get(key).is_some()
	}

	/// Write the tree in the DOT graph descrption language.
	///
	/// Requires the `dot` feature.
	#[cfg(feature = "dot")]
	#[inline]
	pub fn dot_write<W: std::io::Write>(&self, f: &mut W) -> std::io::Result<()> where K: std::fmt::Display, V: std::fmt::Display {
		write!(f, "digraph tree {{\n\tnode [shape=record];\n")?;
		match self.root {
			Some(id) => self.dot_write_node(f, id)?,
			None => ()
		}
		write!(f, "}}")
	}

	/// Write the given node in the DOT graph descrption language.
	///
	/// Requires the `dot` feature.
	#[cfg(feature = "dot")]
	#[inline]
	fn dot_write_node<W: std::io::Write>(&self, f: &mut W, id: usize) -> std::io::Result<()> where K: std::fmt::Display, V: std::fmt::Display {
		let name = format!("n{}", id);
		let node = self.node(id);

		write!(f, "\t{} [label=\"", name)?;
		if let Some(parent) = node.parent() {
			write!(f, "({})|", parent)?;
		}

		node.dot_write_label(f)?;
		write!(f, "({})\"];\n", id)?;

		for child_id in node.children() {
			self.dot_write_node(f, child_id)?;
			let child_name = format!("n{}", child_id);
			write!(f, "\t{} -> {}\n", name, child_name)?;
		}

		Ok(())
	}
}

impl<K, V, C: ContainerMut<Node<K, V>>> BTreeMap<K, V, C> {
	/// Clears the map, removing all elements.
	///
	/// # Example
	///
	/// ```
	/// use local_btree::BTreeMap;
	///
	/// let mut a = BTreeMap::new();
	/// a.insert(1, "a");
	/// a.clear();
	/// assert!(a.is_empty());
	/// ```
	pub fn clear(&mut self) {
		self.root = None;
		self.len = 0;
		self.nodes.clear()
	}

	#[inline]
	pub fn get_mut(&mut self, key: &K) -> Option<&mut V> where K: Ord {
		match self.root {
			Some(id) => self.get_mut_in(key, id),
			None => None
		}
	}

	/// Gets the given key's corresponding entry in the map for in-place manipulation.
	///
	/// # Example
	///
	/// ```
	/// use local_btree::BTreeMap;
	///
	/// let mut letters = BTreeMap::new();
	///
	/// for ch in "a short treatise on fungi".chars() {
	///     let counter = letters.entry(ch).or_insert(0);
	///     *counter += 1;
	/// }
	///
	/// assert_eq!(letters[&'s'], 2);
	/// assert_eq!(letters[&'t'], 3);
	/// assert_eq!(letters[&'u'], 1);
	/// assert_eq!(letters.get(&'y'), None);
	/// ```
	#[inline]
	pub fn entry(&mut self, key: K) -> Entry<K, V, C> where K: Ord {
		match self.address_of(&key) {
			Ok(addr) => {
				Entry::Occupied(OccupiedEntry {
					map: self,
					addr
				})
			},
			Err(addr) => {
				Entry::Vacant(VacantEntry {
					map: self,
					key,
					addr
				})
			}
		}
	}

	/// Returns the first entry in the map for in-place manipulation.
	/// The key of this entry is the minimum key in the map.
	///
	/// # Example
	///
	/// ```
	/// use local_btree::BTreeMap;
	///
	/// let mut map = BTreeMap::new();
	/// map.insert(1, "a");
	/// map.insert(2, "b");
	/// if let Some(mut entry) = map.first_entry() {
	///     if *entry.key() > 0 {
	///         entry.insert("first");
	///     }
	/// }
	/// assert_eq!(*map.get(&1).unwrap(), "first");
	/// assert_eq!(*map.get(&2).unwrap(), "b");
	/// ```
	pub fn first_entry(&mut self) -> Option<OccupiedEntry<K, V, C>> {
		match self.first_item_address() {
			Some(addr) => {
				Some(OccupiedEntry {
					map: self,
					addr
				})
			},
			None => None
		}
	}

	/// Returns the last entry in the map for in-place manipulation.
	/// The key of this entry is the maximum key in the map.
	///
	/// # Example
	///
	/// ```
	/// use local_btree::BTreeMap;
	///
	/// let mut map = BTreeMap::new();
	/// map.insert(1, "a");
	/// map.insert(2, "b");
	/// if let Some(mut entry) = map.last_entry() {
	///     if *entry.key() > 0 {
	///         entry.insert("last");
	///     }
	/// }
	/// assert_eq!(*map.get(&1).unwrap(), "a");
	/// assert_eq!(*map.get(&2).unwrap(), "last");
	/// ```
	pub fn last_entry(&mut self) -> Option<OccupiedEntry<K, V, C>> {
		match self.last_item_address() {
			Some(addr) => {
				Some(OccupiedEntry {
					map: self,
					addr
				})
			},
			None => None
		}
	}

	/// Insert a key-value pair in the tree.
	#[inline]
	pub fn insert(&mut self, key: K, value: V) -> Option<V> where K: Ord {
		match self.address_of(&key) {
			Ok(addr) => {
				Some(self.replace_at(addr, value))
			},
			Err(addr) => {
				self.insert_exactly_at(addr, Item::new(key, value), None);
				None
			}
		}
	}

	/// Removes and returns the first element in the map.
	/// The key of this element is the minimum key that was in the map.
	///
	/// # Example
	///
	/// Draining elements in ascending order, while keeping a usable map each iteration.
	///
	/// ```
	/// use local_btree::BTreeMap;
	///
	/// let mut map = BTreeMap::new();
	/// map.insert(1, "a");
	/// map.insert(2, "b");
	/// while let Some((key, _val)) = map.pop_first() {
	///     assert!(map.iter().all(|(k, _v)| *k > key));
	/// }
	/// assert!(map.is_empty());
	/// ```
	pub fn pop_first(&mut self) -> Option<(K, V)> {
		self.first_entry().map(|entry| entry.remove_entry())
	}

	/// Removes and returns the last element in the map.
	/// The key of this element is the maximum key that was in the map.
	///
	/// # Example
	///
	/// Draining elements in descending order, while keeping a usable map each iteration.
	///
	/// ```
	/// #![feature(map_first_last)]
	/// use local_btree::BTreeMap;
	///
	/// let mut map = BTreeMap::new();
	/// map.insert(1, "a");
	/// map.insert(2, "b");
	/// while let Some((key, _val)) = map.pop_last() {
	///     assert!(map.iter().all(|(k, _v)| *k < key));
	/// }
	/// assert!(map.is_empty());
	/// ```
	pub fn pop_last(&mut self) -> Option<(K, V)> {
		self.last_entry().map(|entry| entry.remove_entry())
	}

	/// Removes a key from the map, returning the value at the key if the key
	/// was previously in the map.
	///
	/// The key may be any borrowed form of the map's key type, but the ordering
	/// on the borrowed form *must* match the ordering on the key type.
	///
	/// # Example
	///
	/// ```
	/// use local_btree::BTreeMap;
	///
	/// let mut map = BTreeMap::new();
	/// map.insert(1, "a");
	/// assert_eq!(map.remove(&1), Some("a"));
	/// assert_eq!(map.remove(&1), None);
	/// ```
	#[inline]
	pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<V> where K: Borrow<Q>, Q: Ord {
		match self.address_of(key) {
			Ok(addr) => {
				let (item, _) = self.remove_at(addr).unwrap();
				Some(item.into_value())
			},
			Err(_) => None
		}
	}

	/// Removes a key from the map, returning the stored key and value if the key
	/// was previously in the map.
	///
	/// The key may be any borrowed form of the map's key type, but the ordering
	/// on the borrowed form *must* match the ordering on the key type.
	///
	/// # Example
	///
	/// Basic usage:
	///
	/// ```
	/// use local_btree::BTreeMap;
	///
	/// let mut map = BTreeMap::new();
	/// map.insert(1, "a");
	/// assert_eq!(map.remove_entry(&1), Some((1, "a")));
	/// assert_eq!(map.remove_entry(&1), None);
	/// ```
	#[inline]
	pub fn remove_entry<Q: ?Sized>(&mut self, key: &Q) -> Option<(K, V)> where K: Borrow<Q>, Q: Ord {
		match self.address_of(key) {
			Ok(addr) => {
				let (item, _) = self.remove_at(addr).unwrap();
				Some(item.into_pair())
			},
			Err(_) => None
		}
	}

	/// Removes and returns the binding in the map, if any, of which key matches the given one.
	#[inline]
	pub fn take<Q: ?Sized>(&mut self, key: &Q) -> Option<(K, V)> where K: Borrow<Q>, Q: Ord {
		match self.address_of(key) {
			Ok(addr) => {
				let (item, _) = self.remove_at(addr).unwrap();
				Some(item.into_pair())
			},
			Err(_) => None
		}
	}

	/// General-purpose update function.
	///
	/// This can be used to insert, compare, replace or remove the value associated to the given
	/// `key` in the tree.
	/// The action to perform is specified by the `action` function.
	/// This function is called once with:
	///  - `Some(value)` when `value` is aready associated to `key` or
	///  - `None` when the `key` is not associated to any value.
	///
	/// The `action` function must return a pair (`new_value`, `result`) where
	/// `new_value` is the new value to be associated to `key`
	/// (if it is `None` any previous binding is removed) and
	/// `result` is the value returned by the entire `update` function call.
	#[inline]
	pub fn update<T, F>(&mut self, key: K, action: F) -> T where K: Ord, F: FnOnce(Option<V>) -> (Option<V>, T) {
		match self.root {
			Some(id) => self.update_in(id, key, action),
			None => {
				let (to_insert, result) = action(None);

				if let Some(value) = to_insert {
					let new_root = Node::leaf(None, Item::new(key, value));
					self.root = Some(self.allocate_node(new_root));
					self.len += 1;
				}

				result
			}
		}
	}

	#[inline]
	pub fn iter_mut(&mut self) -> IterMut<K, V, C> {
		IterMut::new(self)
	}

	#[inline]
	pub fn bindings_mut(&mut self) -> BindingsMut<K, V, C> {
		BindingsMut::new(self)
	}

	/// Constructs a mutable double-ended iterator over a sub-range of elements in the map.
	/// The simplest way is to use the range syntax `min..max`, thus `range(min..max)` will
	/// yield elements from min (inclusive) to max (exclusive).
	/// The range may also be entered as `(Bound<T>, Bound<T>)`, so for example
	/// `range((Excluded(4), Included(10)))` will yield a left-exclusive, right-inclusive
	/// range from 4 to 10.
	///
	/// # Panics
	///
	/// Panics if range `start > end`.
	/// Panics if range `start == end` and both bounds are `Excluded`.
	///
	/// # Example
	///
	/// ```
	/// use local_btree::BTreeMap;
	///
	/// let mut map: BTreeMap<&str, i32> = ["Alice", "Bob", "Carol", "Cheryl"]
	///     .iter()
	///     .map(|&s| (s, 0))
	///     .collect();
	/// for (_, balance) in map.range_mut("B".."Cheryl") {
	///     *balance += 100;
	/// }
	/// for (name, balance) in &map {
	///     println!("{} => {}", name, balance);
	/// }
	/// ```
	pub fn range_mut<T: ?Sized, R>(&mut self, range: R) -> RangeMut<K, V, C>
	where
		T: Ord,
		K: Borrow<T>,
		R: RangeBounds<T>,
	{
		RangeMut::new(self, range)
	}

	/// Gets a mutable iterator over the values of the map, in order by key.
	///
	/// # Example
	///
	/// ```
	/// use local_btree::BTreeMap;
	///
	/// let mut a = BTreeMap::new();
	/// a.insert(1, String::from("hello"));
	/// a.insert(2, String::from("goodbye"));
	///
	/// for value in a.values_mut() {
	///     value.push_str("!");
	/// }
	///
	/// let values: Vec<String> = a.values().cloned().collect();
	/// assert_eq!(values, [String::from("hello!"),
	///                     String::from("goodbye!")]);
	/// ```
	pub fn values_mut(&mut self) -> ValuesMut<K, V, C> {
		ValuesMut { inner: self.iter_mut() }
	}

	/// Creates an iterator which uses a closure to determine if an element should be removed.
	///
	/// If the closure returns true, the element is removed from the map and yielded.
	/// If the closure returns false, or panics, the element remains in the map and will not be
	/// yielded.
	///
	/// Note that `drain_filter` lets you mutate every value in the filter closure, regardless of
	/// whether you choose to keep or remove it.
	///
	/// If the iterator is only partially consumed or not consumed at all, each of the remaining
	/// elements will still be subjected to the closure and removed and dropped if it returns true.
	///
	/// It is unspecified how many more elements will be subjected to the closure
	/// if a panic occurs in the closure, or a panic occurs while dropping an element,
	/// or if the `DrainFilter` value is leaked.
	///
	/// # Example
	///
	/// Splitting a map into even and odd keys, reusing the original map:
	///
	/// ```
	/// use local_btree::BTreeMap;
	///
	/// let mut map: BTreeMap<i32, i32> = (0..8).map(|x| (x, x)).collect();
	/// let evens: BTreeMap<_, _> = map.drain_filter(|k, _v| k % 2 == 0).collect();
	/// let odds = map;
	/// assert_eq!(evens.keys().copied().collect::<Vec<_>>(), vec![0, 2, 4, 6]);
	/// assert_eq!(odds.keys().copied().collect::<Vec<_>>(), vec![1, 3, 5, 7]);
	/// ```
	pub fn drain_filter<F>(&mut self, pred: F) -> DrainFilter<K, V, C, F> where F: FnMut(&K, &mut V) -> bool {
		DrainFilter::new(self, pred)
	}

	#[inline]
	pub fn into_iter(self) -> IntoIter<K, V, C> {
		IntoIter::new(self)
	}

	/// Creates a consuming iterator visiting all the keys, in sorted order.
	/// The map cannot be used after calling this.
	/// The iterator element type is `K`.
	///
	/// # Example
	///
	/// ```
	/// use local_btree::BTreeMap;
	///
	/// let mut a = BTreeMap::new();
	/// a.insert(2, "b");
	/// a.insert(1, "a");
	///
	/// let keys: Vec<i32> = a.into_keys().collect();
	/// assert_eq!(keys, [1, 2]);
	/// ```
	#[inline]
	pub fn into_keys(self) -> IntoKeys<K, V, C> {
		IntoKeys { inner: self.into_iter() }
	}

	/// Creates a consuming iterator visiting all the values, in order by key.
	/// The map cannot be used after calling this.
	/// The iterator element type is `V`.
	///
	/// # Example
	///
	/// ```
	/// use local_btree::BTreeMap;
	///
	/// let mut a = BTreeMap::new();
	/// a.insert(1, "hello");
	/// a.insert(2, "goodbye");
	///
	/// let values: Vec<&str> = a.into_values().collect();
	/// assert_eq!(values, ["hello", "goodbye"]);
	/// ```
	#[inline]
	pub fn into_values(self) -> IntoValues<K, V, C> {
		IntoValues { inner: self.into_iter() }
	}

	/// Try to rotate left the node `id` to benefits the child number `deficient_child_index`.
	///
	/// Returns true if the rotation succeeded, of false if the target child has no right sibling,
	/// or if this sibling would underflow.
	#[inline]
	fn try_rotate_left(&mut self, id: usize, deficient_child_index: usize, addr: &mut ItemAddr) -> bool {
		let pivot_offset = deficient_child_index.into();
		let right_sibling_index = deficient_child_index + 1;
		let (right_sibling_id, deficient_child_id) = {
			let node = self.node(id);

			if right_sibling_index >= node.child_count() {
				return false // no right sibling
			}

			(node.child_id(right_sibling_index), node.child_id(deficient_child_index))
		};

		match self.node_mut(right_sibling_id).pop_left() {
			Ok((mut value, opt_child_id)) => {
				std::mem::swap(&mut value, self.node_mut(id).item_mut(pivot_offset).unwrap());
				let left_offset = self.node_mut(deficient_child_id).push_right(value, opt_child_id);

				// update opt_child's parent
				if let Some(child_id) = opt_child_id {
					self.node_mut(child_id).set_parent(Some(deficient_child_id))
				}

				// update address.
				if addr.id == right_sibling_id { // addressed item is in the right node.
					if addr.offset == 0 {
						// addressed item is moving to pivot.
						addr.id = id;
						addr.offset = pivot_offset;
					} else {
						// addressed item stays on right.
						addr.offset.decr();
					}
				} else if addr.id == id { // addressed item is in the parent node.
					if addr.offset == pivot_offset {
						// addressed item is the pivot, moving to the left (deficient) node.
						addr.id = deficient_child_id;
						addr.offset = left_offset;
					}
				}

				true // rotation succeeded
			},
			Err(WouldUnderflow) => false // the right sibling would underflow.
		}
	}

	/// Try to rotate right the node `id` to benefits the child number `deficient_child_index`.
	///
	/// Returns true if the rotation succeeded, of false if the target child has no left sibling,
	/// or if this sibling would underflow.
	#[inline]
	fn try_rotate_right(&mut self, id: usize, deficient_child_index: usize, addr: &mut ItemAddr) -> bool {
		if deficient_child_index > 0 {
			let left_sibling_index = deficient_child_index - 1;
			let pivot_offset = left_sibling_index.into();
			let (left_sibling_id, deficient_child_id) = {
				let node = self.node(id);
				(node.child_id(left_sibling_index), node.child_id(deficient_child_index))
			};
			match self.node_mut(left_sibling_id).pop_right() {
				Ok((left_offset, mut value, opt_child_id)) => {
					std::mem::swap(&mut value, self.node_mut(id).item_mut(pivot_offset).unwrap());
					self.node_mut(deficient_child_id).push_left(value, opt_child_id);

					// update opt_child's parent
					if let Some(child_id) = opt_child_id {
						self.node_mut(child_id).set_parent(Some(deficient_child_id))
					}

					// update address.
					if addr.id == deficient_child_id { // addressed item is in the right (deficient) node.
						addr.offset.incr();
					} else if addr.id == left_sibling_id { // addressed item is in the left node.
						if addr.offset == left_offset {
							// addressed item is moving to pivot.
							addr.id = id;
							addr.offset = pivot_offset;
						}
					} else if addr.id == id { // addressed item is in the parent node.
						if addr.offset == pivot_offset {
							// addressed item is the pivot, moving to the left (deficient) node.
							addr.id = deficient_child_id;
							addr.offset = 0.into();
						}
					}

					true // rotation succeeded
				},
				Err(WouldUnderflow) => false // the left sibling would underflow.
			}
		} else {
			false // no left sibling.
		}
	}

	/// Merge the child `deficient_child_index` in node `id` with one of its direct sibling.
	#[inline]
	fn merge(&mut self, id: usize, deficient_child_index: usize, mut addr: ItemAddr) -> (Balance, ItemAddr) {
		let (offset, left_id, right_id, separator, balance) = if deficient_child_index > 0 {
			// merge with left sibling
			self.node_mut(id).merge(deficient_child_index-1, deficient_child_index)
		} else {
			// merge with right sibling
			self.node_mut(id).merge(deficient_child_index, deficient_child_index+1)
		};

		// update children's parent.
		let right_node = self.release_node(right_id);
		for right_child_id in right_node.children() {
			self.node_mut(right_child_id).set_parent(Some(left_id));
		}

		// actually merge.
		let left_offset = self.node_mut(left_id).append(separator, right_node);

		// update addr.
		if addr.id == id {
			if addr.offset == offset {
				addr.id = left_id;
				addr.offset = left_offset;
			} else if addr.offset > offset {
				addr.offset.decr();
			}
		} else if addr.id == right_id {
			addr.id = left_id;
			addr.offset = (addr.offset.unwrap() + left_offset.unwrap() + 1).into();
		}

		(balance, addr)
	}
}

impl<K: Ord, Q: ?Sized, V, C: Container<Node<K, V>>> Index<&Q> for BTreeMap<K, V, C>
where
	K: Borrow<Q>,
	Q: Ord,
{
	type Output = V;

	/// Returns a reference to the value corresponding to the supplied key.
	///
	/// # Panics
	///
	/// Panics if the key is not present in the `BTreeMap`.
	#[inline]
	fn index(&self, key: &Q) -> &V {
		self.get(key).expect("no entry found for key")
	}
}

impl<K, L: PartialEq<K>, V, W: PartialEq<V>, C: Container<Node<K, V>>, D: Container<Node<L, W>>> PartialEq<BTreeMap<L, W, D>> for BTreeMap<K, V, C> {
	fn eq(&self, other: &BTreeMap<L, W, D>) -> bool {
		if self.len() == other.len() {
			let mut it1 = self.iter();
			let mut it2 = other.iter();

			loop {
				match (it1.next(), it2.next()) {
					(None, None) => break,
					(Some((k, v)), Some((l, w))) => {
						if l != k || w != v {
							return false
						}
					},
					_ => return false
				}
			}

			true
		} else {
			false
		}
	}
}

impl<K, V, C: Default> Default for BTreeMap<K, V, C> {
	fn default() -> Self {
		BTreeMap::new()
	}
}

impl<K: Ord, V, C: ContainerMut<Node<K, V>> + Default> std::iter::FromIterator<(K, V)> for BTreeMap<K, V, C> {
	fn from_iter<T>(iter: T) -> BTreeMap<K, V, C> where T: IntoIterator<Item = (K, V)> {
		let mut map = BTreeMap::new();

		for (key, value) in iter {
			map.insert(key, value);
		}

		map
	}
}

impl<K: Ord, V, C: ContainerMut<Node<K, V>>> Extend<(K, V)> for BTreeMap<K, V, C> {
	fn extend<T>(&mut self, iter: T) where T: IntoIterator<Item = (K, V)> {
		for (key, value) in iter {
			self.insert(key, value);
		}
	}
}

impl<'a, K: Ord + Copy, V: Copy, C: ContainerMut<Node<K, V>>> Extend<(&'a K, &'a V)> for BTreeMap<K, V, C> {
	#[inline]
	fn extend<T>(&mut self, iter: T) where T: IntoIterator<Item = (&'a K, &'a V)> {
		self.extend(iter.into_iter().map(|(&key, &value)| (key, value)));
	}
}

impl<K: Eq, V: Eq, C: Container<Node<K, V>>> Eq for BTreeMap<K, V, C> {}

impl<K, L: PartialOrd<K>, V, W: PartialOrd<V>, C: Container<Node<K, V>>, D: Container<Node<L, W>>> PartialOrd<BTreeMap<L, W, D>> for BTreeMap<K, V, C> {
	fn partial_cmp(&self, other: &BTreeMap<L, W, D>) -> Option<Ordering> {
		let mut it1 = self.iter();
		let mut it2 = other.iter();

		loop {
			match (it1.next(), it2.next()) {
				(None, None) => return Some(Ordering::Equal),
				(_, None) => return Some(Ordering::Greater),
				(None, _) => return Some(Ordering::Less),
				(Some((k, v)), Some((l, w))) => match l.partial_cmp(k) {
					Some(Ordering::Greater) => return Some(Ordering::Less),
					Some(Ordering::Less) => return Some(Ordering::Greater),
					Some(Ordering::Equal) => match w.partial_cmp(v) {
						Some(Ordering::Greater) => return Some(Ordering::Less),
						Some(Ordering::Less) => return Some(Ordering::Greater),
						Some(Ordering::Equal) => (),
						None => return None
					},
					None => return None
				}
			}
		}
	}
}

impl<K: Ord, V: Ord, C: Container<Node<K, V>>> Ord for BTreeMap<K, V, C> {
	fn cmp(&self, other: &BTreeMap<K, V, C>) -> Ordering {
		let mut it1 = self.iter();
		let mut it2 = other.iter();

		loop {
			match (it1.next(), it2.next()) {
				(None, None) => return Ordering::Equal,
				(_, None) => return Ordering::Greater,
				(None, _) => return Ordering::Less,
				(Some((k, v)), Some((l, w))) => match l.cmp(k) {
					Ordering::Greater => return Ordering::Less,
					Ordering::Less => return Ordering::Greater,
					Ordering::Equal => match w.cmp(v) {
						Ordering::Greater => return Ordering::Less,
						Ordering::Less => return Ordering::Greater,
						Ordering::Equal => ()
					}
				}
			}
		}
	}
}

impl<K: Hash, V: Hash, C: Container<Node<K, V>>> Hash for BTreeMap<K, V, C> {
	fn hash<H: Hasher>(&self, h: &mut H) {
		for (k, v) in self {
			k.hash(h);
			v.hash(h);
		}
	}
}

pub struct Iter<'a, K, V, C> {
	/// The tree reference.
	btree: &'a BTreeMap<K, V, C>,

	/// Address of the next item.
	addr: Option<ItemAddr>,

	end: Option<ItemAddr>,

	len: usize
}

impl<'a, K, V, C: Container<Node<K, V>>> Iter<'a, K, V, C> {
	pub fn new(btree: &'a BTreeMap<K, V, C>) -> Self {
		let addr = btree.first_item_address();
		let len = btree.len();
		Iter {
			btree,
			addr,
			end: None,
			len
		}
	}
}

impl<'a, K, V, C: Container<Node<K, V>>> Iterator for Iter<'a, K, V, C> {
	type Item = (&'a K, &'a V);

	fn size_hint(&self) -> (usize, Option<usize>) {
		(self.len, Some(self.len))
	}

	fn next(&mut self) -> Option<(&'a K, &'a V)> {
		match self.addr {
			Some(addr) => {
				if self.len > 0 {
					self.len -= 1;

					let item = self.btree.item(addr).unwrap();
					self.addr = self.btree.next_item_address(addr);
					Some((item.key(), item.value()))
				} else {
					None
				}
			},
			None => None
		}
	}
}

impl<'a, K, V, C: Container<Node<K, V>>> std::iter::FusedIterator for Iter<'a, K, V, C> { }
impl<'a, K, V, C: Container<Node<K, V>>> std::iter::ExactSizeIterator for Iter<'a, K, V, C> { }

impl<'a, K, V, C: Container<Node<K, V>>> std::iter::DoubleEndedIterator for Iter<'a, K, V, C> {
	fn next_back(&mut self) -> Option<(&'a K, &'a V)> {
		if self.len > 0 {
			let addr = match self.end {
				Some(addr) =>  self.btree.previous_item_address(addr).unwrap(),
				None => self.btree.last_item_address().unwrap()
			};

			self.len -= 1;

			let item = self.btree.item(addr).unwrap();
			self.end = Some(addr);
			Some((item.key(), item.value()))
		} else {
			None
		}
	}
}

impl<'a, K, V, C: Container<Node<K, V>>> IntoIterator for &'a BTreeMap<K, V, C> {
	type IntoIter = Iter<'a, K, V, C>;
	type Item = (&'a K, &'a V);

	fn into_iter(self) -> Iter<'a, K, V, C> {
		self.iter()
	}
}

pub struct IterMut<'a, K, V, C> {
	/// The tree reference.
	btree: &'a mut BTreeMap<K, V, C>,

	/// Address of the next item.
	addr: Option<ItemAddr>,

	end: Option<ItemAddr>,

	len: usize
}

impl<'a, K, V, C: ContainerMut<Node<K, V>>> IterMut<'a, K, V, C> {
	pub fn new(btree: &'a mut BTreeMap<K, V, C>) -> Self {
		let addr = btree.first_item_address();
		let len = btree.len();
		IterMut {
			btree,
			addr,
			end: None,
			len
		}
	}

	fn next_item(&mut self) -> Option<&'a mut Item<K, V>> {
		match self.addr {
			Some(addr) => {
				if self.len > 0 {
					self.len -= 1;

					self.addr = self.btree.next_item_address(addr);
					let item = self.btree.item_mut(addr).unwrap();
					Some(unsafe { std::mem::transmute(item) }) // this is safe because only one mutable reference to the same item can be emitted.
				} else {
					None
				}
			},
			None => None
		}
	}

	fn next_back_item(&mut self) -> Option<&'a mut Item<K, V>> {
		if self.len > 0 {
			let addr = match self.end {
				Some(addr) =>  self.btree.previous_item_address(addr).unwrap(),
				None => self.btree.last_item_address().unwrap()
			};

			self.len -= 1;

			let item = self.btree.item_mut(addr).unwrap();
			self.end = Some(addr);
			Some(unsafe { std::mem::transmute(item) }) // this is safe because only one mutable reference to the same item can be emitted.s
		} else {
			None
		}
	}
}

impl<'a, K, V, C: ContainerMut<Node<K, V>>> Iterator for IterMut<'a, K, V, C> {
	type Item = (&'a K, &'a mut V);

	fn size_hint(&self) -> (usize, Option<usize>) {
		(self.len, Some(self.len))
	}

	fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
		self.next_item().map(|item| {
			let (key, value) = item.as_pair_mut();
			(key as &'a K, value)
		})
	}
}

impl<'a, K, V, C: ContainerMut<Node<K, V>>> std::iter::FusedIterator for IterMut<'a, K, V, C> { }
impl<'a, K, V, C: ContainerMut<Node<K, V>>> std::iter::ExactSizeIterator for IterMut<'a, K, V, C> { }

impl<'a, K, V, C: ContainerMut<Node<K, V>>> std::iter::DoubleEndedIterator for IterMut<'a, K, V, C> {
	fn next_back(&mut self) -> Option<(&'a K, &'a mut V)> {
		self.next_back_item().map(|item| {
			let (key, value) = item.as_pair_mut();
			(key as &'a K, value)
		})
	}
}

/// Iterator that can mutate the tree in place.
pub struct BindingsMut<'a, K, V, C> {
	/// The tree reference.
	btree: &'a mut BTreeMap<K, V, C>,

	/// Address of the next item, or last valid address.
	addr: ItemAddr,

	len: usize
}

impl<'a, K, V, C: ContainerMut<Node<K, V>>> BindingsMut<'a, K, V, C> {
	/// Create a new iterator over all the items of the map.
	pub fn new(btree: &'a mut BTreeMap<K, V, C>) -> BindingsMut<'a, K, V, C> {
		let addr = btree.first_back_address();
		let len = btree.len();
		BindingsMut {
			btree,
			addr,
			len
		}
	}

	/// Get the next visited item without moving the iterator position.
	pub fn peek(&'a self) -> Option<&'a Item<K, V>> {
		self.btree.item(self.addr)
	}

	/// Get the next visited item without moving the iterator position.
	pub fn peek_mut(&'a mut self) -> Option<&'a mut Item<K, V>> {
		self.btree.item_mut(self.addr)
	}

	/// Get the next item and move the iterator to the next position.
	pub fn next_item(&mut self) -> Option<&'a mut Item<K, V>> {
		let after_addr = self.btree.next_item_or_back_address(self.addr);
		match self.btree.item_mut(self.addr) {
			Some(item) => unsafe {
				self.len -= 1;
				self.addr = after_addr.unwrap();
				Some(std::mem::transmute(item as *mut _)) // this is safe because only one mutable reference to the same item can be emitted.
			},
			None => None
		}
	}

	/// Insert a new item in the map before the next item.
	///
	/// ## Correctness
	/// It is safe to insert any key-value pair here, however this might break the well-formedness
	/// of the underlying tree, which relies on several invariants.
	/// To preserve these invariants,
	/// the key must be *strictly greater* than the previous visited item's key,
	/// and *strictly less* than the next visited item
	/// (which you can retrive through `IterMut::peek` without moving the iterator).
	/// If this rule is not respected, the data structure will become unusable
	/// (invalidate the specification of every method of the API).
	pub fn insert(&mut self, key: K, value: V) {
		let addr = self.btree.insert_at(self.addr, Item::new(key, value));
		self.btree.next_item_or_back_address(addr);
		self.len += 1;
	}

	/// Remove the next item and return it.
	pub fn remove(&mut self) -> Option<Item<K, V>> {
		match self.btree.remove_at(self.addr) {
			Some((item, addr)) => {
				self.len -= 1;
				self.addr = addr;
				Some(item)
			},
			None => None
		}
	}
}

impl<'a, K, V, C: ContainerMut<Node<K, V>>> Iterator for BindingsMut<'a, K, V, C> {
	type Item = (&'a K, &'a mut V);

	fn size_hint(&self) -> (usize, Option<usize>) {
		(self.len, Some(self.len))
	}

	fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
		match self.next_item() {
			Some(item) => {
				let (key, value) = item.as_pair_mut();
				Some((key, value)) // coerce k from `&mut K` to `&K`
			},
			None => None
		}
	}
}

/// An owning iterator over the entries of a `BTreeMap`.
///
/// This `struct` is created by the [`into_iter`] method on [`BTreeMap`]
/// (provided by the `IntoIterator` trait). See its documentation for more.
///
/// [`into_iter`]: IntoIterator::into_iter
pub struct IntoIter<K, V, C> {
	/// The tree reference.
	btree: BTreeMap<K, V, C>,

	/// Address of the next item, or the last valid address.
	addr: Option<ItemAddr>,

	/// Address following the last item.
	end: Option<ItemAddr>,

	/// Number of remaining items.
	len: usize
}

impl<K, V, C: ContainerMut<Node<K, V>>> IntoIter<K, V, C> {
	pub fn new(btree: BTreeMap<K, V, C>) -> Self {
		let addr = btree.first_item_address();
		let len = btree.len();
		IntoIter {
			btree,
			addr,
			end: None,
			len
		}
	}
}

impl<K, V, C: ContainerMut<Node<K, V>>> std::iter::FusedIterator for IntoIter<K, V, C> { }
impl<K, V, C: ContainerMut<Node<K, V>>> std::iter::ExactSizeIterator for IntoIter<K, V, C> { }

impl<K, V, C: ContainerMut<Node<K, V>>> Iterator for IntoIter<K, V, C> {
	type Item = (K, V);

	fn size_hint(&self) -> (usize, Option<usize>) {
		(self.len, Some(self.len))
	}

	fn next(&mut self) -> Option<(K, V)> {
		match self.addr {
			Some(addr) => {
				if self.len > 0 {
					self.len -= 1;
		
					let item = unsafe {
						// this is safe because the item at `self.addr` exists and is never touched again.
						std::ptr::read(self.btree.item(addr).unwrap())
					};
		
					if self.len > 0 {
						self.addr = self.btree.next_back_address(addr); // an item address is always followed by a valid address.
		
						while let Some(addr) = self.addr {
							if addr.offset < self.btree.node(addr.id).item_count() {
								break // we have found an item address.
							} else {
								self.addr = self.btree.next_back_address(addr);
		
								// we have gove through every item of the node, we can release it.
								let node = self.btree.release_node(addr.id);
								std::mem::forget(node); // do not call `drop` on the node since items have been moved.
							}
						}
					} else {
						// cleanup.
						if self.end.is_some() {
							while self.addr != self.end {
								let addr = self.addr.unwrap();
								self.addr = self.btree.next_back_address(addr);
	
								if addr.offset >= self.btree.node(addr.id).item_count() {
									let node = self.btree.release_node(addr.id);
									std::mem::forget(node); // do not call `drop` on the node since items have been moved.
								}
							}
						}

						if let Some(addr) = self.addr {
							let mut id = Some(addr.id);
							while let Some(node_id) = id {
								let node = self.btree.release_node(node_id);
								id = node.parent();
								std::mem::forget(node); // do not call `drop` on the node since items have been moved.
							}
						}
					}
		
					Some(item.into_pair())
				} else {
					None
				}
			},
			None => None
		}
	}
}

impl<K, V, C: ContainerMut<Node<K, V>>> std::iter::DoubleEndedIterator for IntoIter<K, V, C> {
	fn next_back(&mut self) -> Option<(K, V)> {
		if self.len > 0 {
			let addr = match self.end {
				Some(mut addr) => {
					addr = self.btree.previous_front_address(addr).unwrap();
					while addr.offset.is_before() {
						let id = addr.id;
						addr = self.btree.previous_front_address(addr).unwrap();
	
						// we have gove through every item of the node, we can release it.
						let node = self.btree.release_node(id);
						std::mem::forget(node); // do not call `drop` on the node since items have been moved.
					}

					addr
				},
				None => self.btree.last_item_address().unwrap()
			};

			self.len -= 1;

			let item = unsafe {
				// this is safe because the item at `self.end` exists and is never touched again.
				std::ptr::read(self.btree.item(addr).unwrap())
			};

			self.end = Some(addr);

			if self.len == 0 {
				// cleanup.
				while self.addr != self.end {
					let addr = self.addr.unwrap();
					self.addr = self.btree.next_back_address(addr);

					if addr.offset >= self.btree.node(addr.id).item_count() {
						let node = self.btree.release_node(addr.id);
						std::mem::forget(node); // do not call `drop` on the node since items have been moved.
					}
				}

				if let Some(addr) = self.addr {
					let mut id = Some(addr.id);
					while let Some(node_id) = id {
						let node = self.btree.release_node(node_id);
						id = node.parent();
						std::mem::forget(node); // do not call `drop` on the node since items have been moved.
					}
				}
			}

			Some(item.into_pair())
		} else {
			None
		}
	}
}

impl<K, V, C: ContainerMut<Node<K, V>>> IntoIterator for BTreeMap<K, V, C> {
	type IntoIter = IntoIter<K, V, C>;
	type Item = (K, V);

	fn into_iter(self) -> IntoIter<K, V, C> {
		self.into_iter()
	}
}

pub struct DrainFilter<'a, K, V, C: ContainerMut<Node<K, V>>, F> where F: FnMut(&K, &mut V) -> bool {
	pred: F,

	/// The tree reference.
	btree: &'a mut BTreeMap<K, V, C>,

	/// Address of the next item, or last valid address.
	addr: ItemAddr,

	len: usize
}

impl<'a, K: 'a, V: 'a, C: ContainerMut<Node<K, V>>, F> DrainFilter<'a, K, V, C, F> where F: FnMut(&K, &mut V) -> bool {
	pub fn new(btree: &'a mut BTreeMap<K, V, C>, pred: F) -> Self {
		let addr = btree.first_back_address();
		let len = btree.len();
		DrainFilter {
			pred,
			btree,
			addr,
			len
		}
	}

	fn next_item(&mut self) -> Option<Item<K, V>> {
		loop {
			match self.btree.item_mut(self.addr) {
				Some(item) => {
					let (key, value) = item.as_pair_mut();
					self.len -= 1;
					if (self.pred)(key, value) {
						let (item, next_addr) = self.btree.remove_at(self.addr).unwrap();
						self.addr = next_addr;
						return Some(item)
					} else {
						self.addr = self.btree.next_item_or_back_address(self.addr).unwrap();
					}
				},
				None => return None
			}
		}
	}
}

impl<'a, K, V, C: ContainerMut<Node<K, V>>, F> std::iter::FusedIterator for DrainFilter<'a, K, V, C, F> where F: FnMut(&K, &mut V) -> bool { }

impl<'a, K, V, C: ContainerMut<Node<K, V>>, F> Iterator for DrainFilter<'a, K, V, C, F> where F: FnMut(&K, &mut V) -> bool {
	type Item = (K, V);

	fn size_hint(&self) -> (usize, Option<usize>) {
		(0, Some(self.len))
	}

	fn next(&mut self) -> Option<(K, V)> {
		match self.next_item() {
			Some(item) => Some(item.into_pair()),
			None => None
		}
	}
}

impl<'a, K, V, C: ContainerMut<Node<K, V>>, F> Drop for DrainFilter<'a, K, V, C, F> where F: FnMut(&K, &mut V) -> bool {
	fn drop(&mut self) {
		loop {
			if self.next().is_none() {
				break
			}
		}
	}
}

pub struct Keys<'a, K, V, C> {
	inner: Iter<'a, K, V, C>
}

impl<'a, K, V, C: Container<Node<K, V>>> std::iter::FusedIterator for Keys<'a, K, V, C> { }
impl<'a, K, V, C: Container<Node<K, V>>> std::iter::ExactSizeIterator for Keys<'a, K, V, C> { }

impl<'a, K, V, C: Container<Node<K, V>>> Iterator for Keys<'a, K, V, C> {
	type Item = &'a K;

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.inner.size_hint()
	}

	fn next(&mut self) -> Option<&'a K> {
		self.inner.next().map(|(k, _)| k)
	}
}

impl<'a, K, V, C: Container<Node<K, V>>> std::iter::DoubleEndedIterator for Keys<'a, K, V, C> {
	fn next_back(&mut self) -> Option<&'a K> {
		self.inner.next_back().map(|(k, _)| k)
	}
}

impl<K, V, C: ContainerMut<Node<K, V>>> std::iter::FusedIterator for IntoKeys<K, V, C> { }
impl<K, V, C: ContainerMut<Node<K, V>>> std::iter::ExactSizeIterator for IntoKeys<K, V, C> { }

pub struct IntoKeys<K, V, C> {
	inner: IntoIter<K, V, C>
}

impl<K, V, C: ContainerMut<Node<K, V>>> Iterator for IntoKeys<K, V, C> {
	type Item = K;

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.inner.size_hint()
	}

	fn next(&mut self) -> Option<K> {
		self.inner.next().map(|(k, _)| k)
	}
}

impl<K, V, C: ContainerMut<Node<K, V>>> std::iter::DoubleEndedIterator for IntoKeys<K, V, C> {
	fn next_back(&mut self) -> Option<K> {
		self.inner.next_back().map(|(k, _)| k)
	}
}

impl<'a, K, V, C: Container<Node<K, V>>> std::iter::FusedIterator for Values<'a, K, V, C> { }
impl<'a, K, V, C: Container<Node<K, V>>> std::iter::ExactSizeIterator for Values<'a, K, V, C> { }

pub struct Values<'a, K, V, C> {
	inner: Iter<'a, K, V, C>
}

impl<'a, K, V, C: Container<Node<K, V>>> Iterator for Values<'a, K, V, C> {
	type Item = &'a V;

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.inner.size_hint()
	}

	fn next(&mut self) -> Option<&'a V> {
		self.inner.next().map(|(_, v)| v)
	}
}

impl<'a, K, V, C: Container<Node<K, V>>> std::iter::DoubleEndedIterator for Values<'a, K, V, C> {
	fn next_back(&mut self) -> Option<&'a V> {
		self.inner.next_back().map(|(_, v)| v)
	}
}

pub struct ValuesMut<'a, K, V, C> {
	inner: IterMut<'a, K, V, C>
}

impl<'a, K, V, C: ContainerMut<Node<K, V>>> std::iter::FusedIterator for ValuesMut<'a, K, V, C> { }
impl<'a, K, V, C: ContainerMut<Node<K, V>>> std::iter::ExactSizeIterator for ValuesMut<'a, K, V, C> { }

impl<'a, K, V, C: ContainerMut<Node<K, V>>> Iterator for ValuesMut<'a, K, V, C> {
	type Item = &'a mut V;

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.inner.size_hint()
	}

	fn next(&mut self) -> Option<&'a mut V> {
		self.inner.next().map(|(_, v)| v)
	}
}

pub struct IntoValues<K, V, C> {
	inner: IntoIter<K, V, C>
}

impl<K, V, C: ContainerMut<Node<K, V>>> std::iter::FusedIterator for IntoValues<K, V, C> { }
impl<K, V, C: ContainerMut<Node<K, V>>> std::iter::ExactSizeIterator for IntoValues<K, V, C> { }

impl<K, V, C: ContainerMut<Node<K, V>>> Iterator for IntoValues<K, V, C> {
	type Item = V;

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.inner.size_hint()
	}

	fn next(&mut self) -> Option<V> {
		self.inner.next().map(|(_, v)| v)
	}
}

impl<K, V, C: ContainerMut<Node<K, V>>> std::iter::DoubleEndedIterator for IntoValues<K, V, C> {
	fn next_back(&mut self) -> Option<V> {
		self.inner.next_back().map(|(_, v)| v)
	}
}

fn is_valid_range<T, R>(range: &R) -> bool where T: Ord + ?Sized, R: RangeBounds<T> {
	match (range.start_bound(), range.end_bound()) {
		(Bound::Included(start), Bound::Included(end)) => start <= end,
		(Bound::Included(start), Bound::Excluded(end)) => start <= end,
		(Bound::Included(_), Bound::Unbounded) => true,
		(Bound::Excluded(start), Bound::Included(end)) => start <= end,
		(Bound::Excluded(start), Bound::Excluded(end)) => start < end,
		(Bound::Excluded(_), Bound::Unbounded) => true,
		(Bound::Unbounded, _) => true
	}
}

pub struct Range<'a, K, V, C> {
	/// The tree reference.
	btree: &'a BTreeMap<K, V, C>,

	/// Address of the next item or last back address.
	addr: ItemAddr,

	end: ItemAddr
}

impl<'a, K, V, C: Container<Node<K, V>>> Range<'a, K, V, C> {
	pub fn new<T, R>(btree: &'a BTreeMap<K, V, C>, range: R) -> Self where T: Ord + ?Sized, R: RangeBounds<T>, K: Borrow<T> {
		if !is_valid_range(&range) {
			panic!("Invalid range")
		}

		let addr = match range.start_bound() {
			Bound::Included(start) => {
				match btree.address_of(start) {
					Ok(addr) => addr,
					Err(addr) => addr
				}
			},
			Bound::Excluded(start) => {
				match btree.address_of(start) {
					Ok(addr) => btree.next_item_or_back_address(addr).unwrap(),
					Err(addr) => addr
				}
			},
			Bound::Unbounded => btree.first_back_address()
		};

		let end = match range.end_bound() {
			Bound::Included(end) => {
				match btree.address_of(end) {
					Ok(addr) => btree.next_item_or_back_address(addr).unwrap(),
					Err(addr) => addr
				}
			},
			Bound::Excluded(end) => {
				match btree.address_of(end) {
					Ok(addr) => addr,
					Err(addr) => addr
				}
			},
			Bound::Unbounded => btree.first_back_address()
		};
		
		Range {
			btree,
			addr,
			end
		}
	}
}

impl<'a, K, V, C: Container<Node<K, V>>> Iterator for Range<'a, K, V, C> {
	type Item = (&'a K, &'a V);

	fn next(&mut self) -> Option<(&'a K, &'a V)> {
		if self.addr != self.end {
			let item = self.btree.item(self.addr).unwrap();
			self.addr = self.btree.next_item_or_back_address(self.addr).unwrap();
			Some((item.key(), item.value()))
		} else {
			None
		}
	}
}

impl<'a, K, V, C: Container<Node<K, V>>> std::iter::FusedIterator for Range<'a, K, V, C> { }

impl<'a, K, V, C: Container<Node<K, V>>> std::iter::DoubleEndedIterator for Range<'a, K, V, C> {
	fn next_back(&mut self) -> Option<(&'a K, &'a V)> {
		if self.addr != self.end {
			let addr = self.btree.previous_item_address(self.addr).unwrap();
			let item = self.btree.item(addr).unwrap();
			self.end = addr;
			Some((item.key(), item.value()))
		} else {
			None
		}
	}
}

pub struct RangeMut<'a, K, V, C> {
	/// The tree reference.
	btree: &'a mut BTreeMap<K, V, C>,

	/// Address of the next item or last back address.
	addr: ItemAddr,

	end: ItemAddr
}

impl<'a, K, V, C: ContainerMut<Node<K, V>>> RangeMut<'a, K, V, C> {
	pub fn new<T, R>(btree: &'a mut BTreeMap<K, V, C>, range: R) -> Self where T: Ord + ?Sized, R: RangeBounds<T>, K: Borrow<T> {
		if !is_valid_range(&range) {
			panic!("Invalid range")
		}

		let addr = match range.start_bound() {
			Bound::Included(start) => {
				match btree.address_of(start) {
					Ok(addr) => addr,
					Err(addr) => addr
				}
			},
			Bound::Excluded(start) => {
				match btree.address_of(start) {
					Ok(addr) => btree.next_item_or_back_address(addr).unwrap(),
					Err(addr) => addr
				}
			},
			Bound::Unbounded => btree.first_back_address()
		};

		let end = match range.end_bound() {
			Bound::Included(end) => {
				match btree.address_of(end) {
					Ok(addr) => btree.next_item_or_back_address(addr).unwrap(),
					Err(addr) => addr
				}
			},
			Bound::Excluded(end) => {
				match btree.address_of(end) {
					Ok(addr) => addr,
					Err(addr) => addr
				}
			},
			Bound::Unbounded => btree.first_back_address()
		};
		
		RangeMut {
			btree,
			addr,
			end
		}
	}

	fn next_item(&mut self) -> Option<&'a mut Item<K, V>> {
		if self.addr != self.end {
			let addr = self.addr;
			self.addr = self.btree.next_item_or_back_address(addr).unwrap();
			let item = self.btree.item_mut(addr).unwrap();
			Some(unsafe { std::mem::transmute(item) }) // this is safe because only one mutable reference to the same item can be emitted.
		} else {
			None
		}
	}

	fn next_back_item(&mut self) -> Option<&'a mut Item<K, V>> {
		if self.addr != self.end {
			let addr = self.btree.previous_item_address(self.addr).unwrap();
			let item = self.btree.item_mut(addr).unwrap();
			self.end = addr;
			Some(unsafe { std::mem::transmute(item) }) // this is safe because only one mutable reference to the same item can be emitted.s
		} else {
			None
		}
	}
}

impl<'a, K, V, C: ContainerMut<Node<K, V>>> Iterator for RangeMut<'a, K, V, C> {
	type Item = (&'a K, &'a mut V);

	fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
		self.next_item().map(|item| {
			let (key, value) = item.as_pair_mut();
			(key as &'a K, value)
		})
	}
}

impl<'a, K, V, C: ContainerMut<Node<K, V>>> std::iter::FusedIterator for RangeMut<'a, K, V, C> { }

impl<'a, K, V, C: ContainerMut<Node<K, V>>> std::iter::DoubleEndedIterator for RangeMut<'a, K, V, C> {
	fn next_back(&mut self) -> Option<(&'a K, &'a mut V)> {
		self.next_back_item().map(|item| {
			let (key, value) = item.as_pair_mut();
			(key as &'a K, value)
		})
	}
}
