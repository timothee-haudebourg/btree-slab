use slab::Slab;

use std::marker::PhantomData;
use crate::{
	node::{
		Item,
		ItemAddr,
		Node,
		Balance,
		WouldUnderflow
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
/// In addition the crate provides advances functions to iterate through and update the map
/// efficiently.
///
/// # Basic usage
/// Basic usage is similar to the map data structures offered by the standard library.
/// ```
/// use linear_btree::BTreeMap;
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
/// This crate also reproduces the [`Entry API`] defined by the standard library,
/// which allows for more complex methods of getting, setting, updating and removing keys and
/// their values:
/// ```
/// use linear_btree::BTreeMap;
///
/// // type inference lets us omit an explicit type signature (which
/// // would be `BTreeMap<&str, u8>` in this example).
/// let mut player_stats = BTreeMap::new();
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
pub struct BTreeMap<K, V, C = Slab<Node<K, V>>> {
	/// Allocated and free nodes.
	nodes: C,

	/// Root node id.
	root: Option<usize>,

	/// Number of items in the tree.
	len: usize,

	k: PhantomData<K>,
	v: PhantomData<V>
}

impl<K, V, C: Container<Node<K, V>>> BTreeMap<K, V, C> {
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

	#[inline]
	pub fn get(&self, key: &K) -> Option<&V> where K: Ord {
		match self.root {
			Some(id) => self.get_in(key, id),
			None => None
		}
	}

	#[inline]
	pub fn contains(&self, key: &K) -> bool where K: Ord {
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
	#[inline]
	pub fn get_mut(&mut self, key: &K) -> Option<&mut V> where K: Ord {
		match self.root {
			Some(id) => self.get_mut_in(key, id),
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

	// Delete an item by key.
	#[inline]
	pub fn remove(&mut self, key: &K) -> Option<V> where K: Ord {
		match self.address_of(key) {
			Ok(addr) => {
				let (item, _) = self.remove_at(addr).unwrap();
				Some(item.into_value())
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

	/// Try to rotate left the node `id` to benefits the child number `deficient_child_index`.
	///
	/// Returns true if the rotation succeeded, of false if the target child has no right sibling,
	/// or if this sibling would underflow.
	#[inline]
	fn try_rotate_left(&mut self, id: usize, deficient_child_index: usize, addr: &mut ItemAddr) -> bool {
		let pivot_offset = deficient_child_index;
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
						addr.offset -= 1;
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
			let pivot_offset = left_sibling_index;
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
						addr.offset += 1;
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
							addr.offset = 0;
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
				addr.offset -= 1;
			}
		} else if addr.id == right_id {
			addr.id = left_id;
			addr.offset += left_offset + 1;
		}

		(balance, addr)
	}
}

/// Iterator that can mutate the tree in place.
pub struct IterMut<'a, K, V, C = Slab<Node<K, V>>> {
	/// The tree reference.
	btree: &'a mut BTreeMap<K, V, C>,

	/// Address of the next item.
	addr: ItemAddr
}

impl<'a, K, V, C: ContainerMut<Node<K, V>>> IterMut<'a, K, V, C> {
	/// Create a new iterator over all the items of the map.
	pub fn new(btree: &'a mut BTreeMap<K, V, C>) -> IterMut<'a, K, V, C> {
		let addr = btree.first_address();
		IterMut {
			btree,
			addr
		}
	}

	/// Get the next visited item without moving the iterator position.
	pub fn peek(&'a self) -> Option<&'a Item<K, V>> {
		self.btree.item(self.addr)
	}

	/// Get the next item and move the iterator to the next position.
	pub fn next(&'a mut self) -> Option<&'a mut Item<K, V>> {
		let after_addr = self.btree.next_address(self.addr);
		match self.btree.item_mut(self.addr) {
			Some(item) => {
				self.addr = after_addr.unwrap();
				Some(item)
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
		self.addr = self.btree.next_address(addr).unwrap();
	}

	/// Remove the next item and return it.
	pub fn remove(&mut self) -> Option<Item<K, V>> {
		match self.btree.remove_at(self.addr) {
			Some((item, addr)) => {
				self.addr = addr;
				Some(item)
			},
			None => None
		}
	}
}
