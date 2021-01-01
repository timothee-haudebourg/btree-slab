use std::{
	borrow::Borrow,
	cmp::Ordering,
	hash::{
		Hash,
		Hasher
	}
};
use crate::{
	generic::{
		BTreeMap,
		node::{
			Node
		}
	},
	Container,
	ContainerMut
};

/// A set based on a B-Tree.
/// 
/// See [`BTreeMap`]'s documentation for a detailed discussion of this collection's performance benefits and drawbacks.
/// 
/// It is a logic error for an item to be modified in such a way that the item's ordering relative
/// to any other item, as determined by the [`Ord`] trait, changes while it is in the set. This is
/// normally only possible through [`Cell`], [`RefCell`], global state, I/O, or unsafe code.
///
/// [`Ord`]: core::cmp::Ord
/// [`Cell`]: core::cell::Cell
/// [`RefCell`]: core::cell::RefCell
pub struct BTreeSet<T, C> {
	map: BTreeMap<T, (), C>
}

impl<T, C> BTreeSet<T, C> {
	/// Makes a new, empty `BTreeSet`.
	///
	/// # Example
	///
	/// ```
	/// # #![allow(unused_mut)]
	/// use local_btree::BTreeSet;
	///
	/// let mut set: BTreeSet<i32> = BTreeSet::new();
	/// ```
	pub fn new() -> Self where C: Default {
		BTreeSet {
			map: BTreeMap::new()
		}
	}

	/// Returns the number of elements in the set.
	///
	/// # Example
	///
	/// ```
	/// use local_btree::BTreeSet;
	///
	/// let mut v = BTreeSet::new();
	/// assert_eq!(v.len(), 0);
	/// v.insert(1);
	/// assert_eq!(v.len(), 1);
	/// ```
	pub fn len(&self) -> usize {
		self.map.len()
	}

	/// Returns `true` if the set contains no elements.
	///
	/// # Example
	///
	/// ```
	/// use local_btree::BTreeSet;
	///
	/// let mut v = BTreeSet::new();
	/// assert!(v.is_empty());
	/// v.insert(1);
	/// assert!(!v.is_empty());
	/// ```
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}
}

impl<T, C: Container<Node<T, ()>>> BTreeSet<T, C> {
	// ...
}

impl<T, C: ContainerMut<Node<T, ()>>> BTreeSet<T, C> {
	/// Clears the set, removing all values.
	///
	/// # Examples
	///
	/// ```
	/// use local_btree::BTreeSet;
	///
	/// let mut v = BTreeSet::new();
	/// v.insert(1);
	/// v.clear();
	/// assert!(v.is_empty());
	/// ```
	pub fn clear(&mut self) {
		self.map.clear()
	}

	/// Adds a value to the set.
	///
	/// If the set did not have this value present, `true` is returned.
	///
	/// If the set did have this value present, `false` is returned, and the
	/// entry is not updated. See the [module-level documentation] for more.
	///
	/// [module-level documentation]: index.html#insert-and-complex-keys
	///
	/// # Example
	///
	/// ```
	/// use local_btree::BTreeSet;
	///
	/// let mut set = BTreeSet::new();
	///
	/// assert_eq!(set.insert(2), true);
	/// assert_eq!(set.insert(2), false);
	/// assert_eq!(set.len(), 1);
	/// ```
	pub fn insert(&mut self, element: T) -> bool where T: Ord {
		self.map.insert(element, ()).is_none()
	}

	/// Removes a value from the set. Returns whether the value was
	/// present in the set.
	///
	/// The value may be any borrowed form of the set's value type,
	/// but the ordering on the borrowed form *must* match the
	/// ordering on the value type.
	///
	/// # Example
	///
	/// ```
	/// use local_btree::BTreeSet;
	///
	/// let mut set = BTreeSet::new();
	///
	/// set.insert(2);
	/// assert_eq!(set.remove(&2), true);
	/// assert_eq!(set.remove(&2), false);
	/// ```
	pub fn remove<Q: ?Sized>(&mut self, value: &Q) -> bool where T: Borrow<Q>, Q: Ord {
		self.map.remove(value).is_some()
	}
	
	/// Removes and returns the value in the set, if any, that is equal to the given one.
	///
	/// The value may be any borrowed form of the set's value type,
	/// but the ordering on the borrowed form *must* match the
	/// ordering on the value type.
	///
	/// # Examples
	///
	/// ```
	/// use local_btree::BTreeSet;
	///
	/// let mut set: BTreeSet<_> = [1, 2, 3].iter().cloned().collect();
	/// assert_eq!(set.take(&2), Some(2));
	/// assert_eq!(set.take(&2), None);
	/// ```
	pub fn take<Q: ?Sized>(&mut self, value: &Q) -> Option<T> where T: Borrow<Q>, Q: Ord {
		match self.map.take(value) {
			Some((t, _)) => Some(t),
			None => None
		}
	}
}

impl<T: Clone, C: Clone> Clone for BTreeSet<T, C> {
	fn clone(&self) -> Self {
		BTreeSet { map: self.map.clone() }
	}

	fn clone_from(&mut self, other: &Self) {
		self.map.clone_from(&other.map);
	}
}

impl<T: Ord, C: ContainerMut<Node<T, ()>> + Default> std::iter::FromIterator<T> for BTreeSet<T, C> {
	fn from_iter<I>(iter: I) -> Self where I: IntoIterator<Item=T> {
		let mut set = BTreeSet::new();
		set.extend(iter);
		set
	}
}

impl<T: Ord, C: ContainerMut<Node<T, ()>>> Extend<T> for BTreeSet<T, C> {
	fn extend<I>(&mut self, iter: I) where I: IntoIterator<Item=T> {
		for t in iter {
			self.insert(t);
		}
	}
}

impl<'a, T: Ord + Copy, C: ContainerMut<Node<T, ()>>> Extend<&'a T> for BTreeSet<T, C> {
	fn extend<I>(&mut self, iter: I) where I: IntoIterator<Item=&'a T> {
		self.extend(iter.into_iter().map(|&t| t))
	}
}

impl<T, L: PartialEq<T>, C: Container<Node<T, ()>>, D: Container<Node<L, ()>>> PartialEq<BTreeSet<L, D>> for BTreeSet<T, C> {
	fn eq(&self, other: &BTreeSet<L, D>) -> bool {
		self.map.eq(&other.map)
	}
}

impl<T: Eq, C: Container<Node<T, ()>>> Eq for BTreeSet<T, C> {}

impl<T, L: PartialOrd<T>, C: Container<Node<T, ()>>, D: Container<Node<L, ()>>> PartialOrd<BTreeSet<L, D>> for BTreeSet<T, C> {
	fn partial_cmp(&self, other: &BTreeSet<L, D>) -> Option<Ordering> {
		self.map.partial_cmp(&other.map)
	}
}

impl<T: Ord, C: Container<Node<T, ()>>> Ord for BTreeSet<T, C> {
	fn cmp(&self, other: &BTreeSet<T, C>) -> Ordering {
		self.map.cmp(&other.map)
	}
}

impl<T: Hash, C: Container<Node<T, ()>>> Hash for BTreeSet<T, C> {
	fn hash<H: Hasher>(&self, h: &mut H) {
		self.map.hash(h)
	}
}