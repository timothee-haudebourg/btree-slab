use crate::generic::{map, node::Node, BTreeMap};
use cc_traits::{SimpleCollectionMut, SimpleCollectionRef, Slab, SlabMut};
use std::{
	borrow::Borrow,
	cmp::Ordering,
	hash::{Hash, Hasher},
	iter::{DoubleEndedIterator, ExactSizeIterator, FromIterator, FusedIterator, Peekable},
	ops::RangeBounds,
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
	map: BTreeMap<T, (), C>,
}

impl<T, C> BTreeSet<T, C> {
	/// Makes a new, empty `BTreeSet`.
	///
	/// # Example
	///
	/// ```
	/// # #![allow(unused_mut)]
	/// use btree_slab::BTreeSet;
	///
	/// let mut set: BTreeSet<i32> = BTreeSet::new();
	/// ```
	#[inline]
	pub fn new() -> Self
	where
		C: Default,
	{
		Self::default()
	}

	/// Returns the number of elements in the set.
	///
	/// # Example
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let mut v = BTreeSet::new();
	/// assert_eq!(v.len(), 0);
	/// v.insert(1);
	/// assert_eq!(v.len(), 1);
	/// ```
	#[inline]
	pub fn len(&self) -> usize {
		self.map.len()
	}

	/// Returns `true` if the set contains no elements.
	///
	/// # Example
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let mut v = BTreeSet::new();
	/// assert!(v.is_empty());
	/// v.insert(1);
	/// assert!(!v.is_empty());
	/// ```
	#[inline]
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}
}

impl<T, C: Default> Default for BTreeSet<T, C> {
	fn default() -> Self {
		BTreeSet {
			map: BTreeMap::default(),
		}
	}
}

impl<T, C: Slab<Node<T, ()>>> BTreeSet<T, C>
where
	C: SimpleCollectionRef,
{
	/// Gets an iterator that visits the values in the `BTreeSet` in ascending order.
	///
	/// # Examples
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let set: BTreeSet<usize> = [1, 2, 3].iter().cloned().collect();
	/// let mut set_iter = set.iter();
	/// assert_eq!(set_iter.next(), Some(&1));
	/// assert_eq!(set_iter.next(), Some(&2));
	/// assert_eq!(set_iter.next(), Some(&3));
	/// assert_eq!(set_iter.next(), None);
	/// ```
	///
	/// Values returned by the iterator are returned in ascending order:
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let set: BTreeSet<usize> = [3, 1, 2].iter().cloned().collect();
	/// let mut set_iter = set.iter();
	/// assert_eq!(set_iter.next(), Some(&1));
	/// assert_eq!(set_iter.next(), Some(&2));
	/// assert_eq!(set_iter.next(), Some(&3));
	/// assert_eq!(set_iter.next(), None);
	/// ```
	#[inline]
	pub fn iter(&self) -> Iter<T, C> {
		Iter {
			inner: self.map.keys(),
		}
	}
}

impl<T: Ord, C: Slab<Node<T, ()>>> BTreeSet<T, C>
where
	C: SimpleCollectionRef,
{
	/// Returns `true` if the set contains a value.
	///
	/// The value may be any borrowed form of the set's value type,
	/// but the ordering on the borrowed form *must* match the
	/// ordering on the value type.
	///
	/// # Example
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let set: BTreeSet<_> = [1, 2, 3].iter().cloned().collect();
	/// assert_eq!(set.contains(&1), true);
	/// assert_eq!(set.contains(&4), false);
	/// ```
	#[inline]
	pub fn contains<Q: ?Sized>(&self, value: &Q) -> bool
	where
		T: Borrow<Q>,
		Q: Ord,
	{
		self.map.contains_key(value)
	}

	/// Returns a reference to the value in the set, if any, that is equal to the given value.
	///
	/// The value may be any borrowed form of the set's value type,
	/// but the ordering on the borrowed form *must* match the
	/// ordering on the value type.
	///
	/// # Example
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let set: BTreeSet<_> = [1, 2, 3].iter().cloned().collect();
	/// assert_eq!(set.get(&2), Some(&2));
	/// assert_eq!(set.get(&4), None);
	/// ```
	#[inline]
	pub fn get<Q: ?Sized>(&self, value: &Q) -> Option<&T>
	where
		T: Borrow<Q>,
		Q: Ord,
	{
		match self.map.get_key_value(value) {
			Some((t, ())) => Some(t),
			None => None,
		}
	}

	/// Constructs a double-ended iterator over a sub-range of elements in the set.
	/// The simplest way is to use the range syntax `min..max`, thus `range(min..max)` will
	/// yield elements from min (inclusive) to max (exclusive).
	/// The range may also be entered as `(Bound<T>, Bound<T>)`, so for example
	/// `range((Excluded(4), Included(10)))` will yield a left-exclusive, right-inclusive
	/// range from 4 to 10.
	///
	/// # Example
	///
	/// ```
	/// use btree_slab::BTreeSet;
	/// use std::ops::Bound::Included;
	///
	/// let mut set = BTreeSet::new();
	/// set.insert(3);
	/// set.insert(5);
	/// set.insert(8);
	/// for &elem in set.range((Included(&4), Included(&8))) {
	///     println!("{}", elem);
	/// }
	/// assert_eq!(Some(&5), set.range(4..).next());
	/// ```
	#[inline]
	pub fn range<K: ?Sized, R>(&self, range: R) -> Range<T, C>
	where
		K: Ord,
		T: Borrow<K>,
		R: RangeBounds<K>,
	{
		Range {
			inner: self.map.range(range),
		}
	}

	/// Visits the values representing the union,
	/// i.e., all the values in `self` or `other`, without duplicates,
	/// in ascending order.
	///
	/// # Example
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let mut a = BTreeSet::new();
	/// a.insert(1);
	///
	/// let mut b = BTreeSet::new();
	/// b.insert(2);
	///
	/// let union: Vec<_> = a.union(&b).cloned().collect();
	/// assert_eq!(union, [1, 2]);
	/// ```
	#[inline]
	pub fn union<'a, D: Slab<Node<T, ()>>>(
		&'a self,
		other: &'a BTreeSet<T, D>,
	) -> Union<'a, T, C, D>
	where
		D: SimpleCollectionRef,
	{
		Union {
			it1: self.iter().peekable(),
			it2: other.iter().peekable(),
		}
	}

	/// Visits the values representing the intersection,
	/// i.e., the values that are both in `self` and `other`,
	/// in ascending order.
	///
	/// # Example
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let mut a = BTreeSet::new();
	/// a.insert(1);
	/// a.insert(2);
	///
	/// let mut b = BTreeSet::new();
	/// b.insert(2);
	/// b.insert(3);
	///
	/// let intersection: Vec<_> = a.intersection(&b).cloned().collect();
	/// assert_eq!(intersection, [2]);
	/// ```
	#[inline]
	pub fn intersection<'a, D: Slab<Node<T, ()>>>(
		&'a self,
		other: &'a BTreeSet<T, D>,
	) -> Intersection<'a, T, C, D>
	where
		D: SimpleCollectionRef,
	{
		Intersection {
			it1: self.iter(),
			it2: other.iter().peekable(),
		}
	}

	/// Visits the values representing the difference,
	/// i.e., the values that are in `self` but not in `other`,
	/// in ascending order.
	///
	/// # Example
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let mut a = BTreeSet::new();
	/// a.insert(1);
	/// a.insert(2);
	///
	/// let mut b = BTreeSet::new();
	/// b.insert(2);
	/// b.insert(3);
	///
	/// let diff: Vec<_> = a.difference(&b).cloned().collect();
	/// assert_eq!(diff, [1]);
	/// ```
	#[inline]
	pub fn difference<'a, D: Slab<Node<T, ()>>>(
		&'a self,
		other: &'a BTreeSet<T, D>,
	) -> Difference<'a, T, C, D>
	where
		D: SimpleCollectionRef,
	{
		Difference {
			it1: self.iter(),
			it2: other.iter().peekable(),
		}
	}

	/// Visits the values representing the symmetric difference,
	/// i.e., the values that are in `self` or in `other` but not in both,
	/// in ascending order.
	///
	/// # Example
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let mut a = BTreeSet::new();
	/// a.insert(1);
	/// a.insert(2);
	///
	/// let mut b = BTreeSet::new();
	/// b.insert(2);
	/// b.insert(3);
	///
	/// let sym_diff: Vec<_> = a.symmetric_difference(&b).cloned().collect();
	/// assert_eq!(sym_diff, [1, 3]);
	/// ```
	#[inline]
	pub fn symmetric_difference<'a, D: Slab<Node<T, ()>>>(
		&'a self,
		other: &'a BTreeSet<T, D>,
	) -> SymmetricDifference<'a, T, C, D>
	where
		D: SimpleCollectionRef,
	{
		SymmetricDifference {
			it1: self.iter().peekable(),
			it2: other.iter().peekable(),
		}
	}

	/// Returns `true` if `self` has no elements in common with `other`.
	/// This is equivalent to checking for an empty intersection.
	///
	/// # Example
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let a: BTreeSet<_> = [1, 2, 3].iter().cloned().collect();
	/// let mut b = BTreeSet::new();
	///
	/// assert_eq!(a.is_disjoint(&b), true);
	/// b.insert(4);
	/// assert_eq!(a.is_disjoint(&b), true);
	/// b.insert(1);
	/// assert_eq!(a.is_disjoint(&b), false);
	/// ```
	#[inline]
	pub fn is_disjoint<D: Slab<Node<T, ()>>>(&self, other: &BTreeSet<T, D>) -> bool
	where
		D: SimpleCollectionRef,
	{
		self.intersection(other).next().is_none()
	}

	/// Returns `true` if the set is a subset of another,
	/// i.e., `other` contains at least all the values in `self`.
	///
	/// # Example
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let sup: BTreeSet<_> = [1, 2, 3].iter().cloned().collect();
	/// let mut set = BTreeSet::new();
	///
	/// assert_eq!(set.is_subset(&sup), true);
	/// set.insert(2);
	/// assert_eq!(set.is_subset(&sup), true);
	/// set.insert(4);
	/// assert_eq!(set.is_subset(&sup), false);
	/// ```
	#[inline]
	pub fn is_subset<D: Slab<Node<T, ()>>>(&self, other: &BTreeSet<T, D>) -> bool
	where
		D: SimpleCollectionRef,
	{
		self.difference(other).next().is_none()
	}

	/// Returns `true` if the set is a superset of another,
	/// i.e., `self` contains at least all the values in `other`.
	///
	/// # Example
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let sub: BTreeSet<_> = [1, 2].iter().cloned().collect();
	/// let mut set = BTreeSet::new();
	///
	/// assert_eq!(set.is_superset(&sub), false);
	///
	/// set.insert(0);
	/// set.insert(1);
	/// assert_eq!(set.is_superset(&sub), false);
	///
	/// set.insert(2);
	/// assert_eq!(set.is_superset(&sub), true);
	/// ```
	#[inline]
	pub fn is_superset<D: Slab<Node<T, ()>>>(&self, other: &BTreeSet<T, D>) -> bool
	where
		D: SimpleCollectionRef,
	{
		other.is_subset(self)
	}

	/// Returns a reference to the first value in the set, if any.
	/// This value is always the minimum of all values in the set.
	///
	/// # Example
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let mut map = BTreeSet::new();
	/// assert_eq!(map.first(), None);
	/// map.insert(1);
	/// assert_eq!(map.first(), Some(&1));
	/// map.insert(2);
	/// assert_eq!(map.first(), Some(&1));
	/// ```
	#[inline]
	pub fn first(&self) -> Option<&T> {
		self.map.first_key_value().map(|(k, _)| k)
	}

	/// Returns a reference to the last value in the set, if any.
	/// This value is always the maximum of all values in the set.
	///
	/// # Example
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let mut map = BTreeSet::new();
	/// assert_eq!(map.first(), None);
	/// map.insert(1);
	/// assert_eq!(map.last(), Some(&1));
	/// map.insert(2);
	/// assert_eq!(map.last(), Some(&2));
	/// ```
	#[inline]
	pub fn last(&self) -> Option<&T> {
		self.map.last_key_value().map(|(k, _)| k)
	}
}

impl<T: Ord, C: SlabMut<Node<T, ()>>> BTreeSet<T, C>
where
	C: SimpleCollectionRef,
	C: SimpleCollectionMut,
{
	/// Clears the set, removing all values.
	///
	/// # Examples
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let mut v = BTreeSet::new();
	/// v.insert(1);
	/// v.clear();
	/// assert!(v.is_empty());
	/// ```
	#[inline]
	pub fn clear(&mut self)
	where
		C: cc_traits::Clear,
	{
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
	/// use btree_slab::BTreeSet;
	///
	/// let mut set = BTreeSet::new();
	///
	/// assert_eq!(set.insert(2), true);
	/// assert_eq!(set.insert(2), false);
	/// assert_eq!(set.len(), 1);
	/// ```
	#[inline]
	pub fn insert(&mut self, element: T) -> bool
	where
		T: Ord,
	{
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
	/// use btree_slab::BTreeSet;
	///
	/// let mut set = BTreeSet::new();
	///
	/// set.insert(2);
	/// assert_eq!(set.remove(&2), true);
	/// assert_eq!(set.remove(&2), false);
	/// ```
	#[inline]
	pub fn remove<Q: ?Sized>(&mut self, value: &Q) -> bool
	where
		T: Borrow<Q>,
		Q: Ord,
	{
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
	/// use btree_slab::BTreeSet;
	///
	/// let mut set: BTreeSet<_> = [1, 2, 3].iter().cloned().collect();
	/// assert_eq!(set.take(&2), Some(2));
	/// assert_eq!(set.take(&2), None);
	/// ```
	#[inline]
	pub fn take<Q: ?Sized>(&mut self, value: &Q) -> Option<T>
	where
		T: Borrow<Q>,
		Q: Ord,
	{
		self.map.take(value).map(|(t, _)| t)
	}

	/// Adds a value to the set, replacing the existing value, if any, that is equal to the given
	/// one. Returns the replaced value.
	///
	/// # Example
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let mut set = BTreeSet::new();
	/// set.insert(Vec::<i32>::new());
	///
	/// assert_eq!(set.get(&[][..]).unwrap().capacity(), 0);
	/// set.replace(Vec::with_capacity(10));
	/// assert_eq!(set.get(&[][..]).unwrap().capacity(), 10);
	/// ```
	#[inline]
	pub fn replace(&mut self, value: T) -> Option<T> {
		self.map.replace(value, ()).map(|(t, ())| t)
	}

	/// Removes the first value from the set and returns it, if any.
	/// The first value is always the minimum value in the set.
	///
	/// # Example
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let mut set = BTreeSet::new();
	///
	/// set.insert(1);
	/// while let Some(n) = set.pop_first() {
	///     assert_eq!(n, 1);
	/// }
	/// assert!(set.is_empty());
	/// ```
	#[inline]
	pub fn pop_first(&mut self) -> Option<T> {
		self.map.pop_first().map(|kv| kv.0)
	}

	/// Removes the last value from the set and returns it, if any.
	/// The last value is always the maximum value in the set.
	///
	/// # Example
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let mut set = BTreeSet::new();
	///
	/// set.insert(1);
	/// while let Some(n) = set.pop_last() {
	///     assert_eq!(n, 1);
	/// }
	/// assert!(set.is_empty());
	/// ```
	#[inline]
	pub fn pop_last(&mut self) -> Option<T> {
		self.map.pop_last().map(|kv| kv.0)
	}

	/// Retains only the elements specified by the predicate.
	///
	/// In other words, remove all elements `e` such that `f(&e)` returns `false`.
	///
	/// # Example
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let xs = [1, 2, 3, 4, 5, 6];
	/// let mut set: BTreeSet<i32> = xs.iter().cloned().collect();
	/// // Keep only the even numbers.
	/// set.retain(|&k| k % 2 == 0);
	/// assert!(set.iter().eq([2, 4, 6].iter()));
	/// ```
	#[inline]
	pub fn retain<F>(&mut self, mut f: F)
	where
		F: FnMut(&T) -> bool,
	{
		self.drain_filter(|v| !f(v));
	}

	/// Moves all elements from `other` into `Self`, leaving `other` empty.
	///
	/// # Example
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let mut a = BTreeSet::new();
	/// a.insert(1);
	/// a.insert(2);
	/// a.insert(3);
	///
	/// let mut b = BTreeSet::new();
	/// b.insert(3);
	/// b.insert(4);
	/// b.insert(5);
	///
	/// a.append(&mut b);
	///
	/// assert_eq!(a.len(), 5);
	/// assert_eq!(b.len(), 0);
	///
	/// assert!(a.contains(&1));
	/// assert!(a.contains(&2));
	/// assert!(a.contains(&3));
	/// assert!(a.contains(&4));
	/// assert!(a.contains(&5));
	/// ```
	#[inline]
	pub fn append(&mut self, other: &mut Self)
	where
		C: Default,
	{
		self.map.append(&mut other.map);
	}

	/// Creates an iterator which uses a closure to determine if a value should be removed.
	///
	/// If the closure returns true, then the value is removed and yielded.
	/// If the closure returns false, the value will remain in the list and will not be yielded
	/// by the iterator.
	///
	/// If the iterator is only partially consumed or not consumed at all, each of the remaining
	/// values will still be subjected to the closure and removed and dropped if it returns true.
	///
	/// It is unspecified how many more values will be subjected to the closure
	/// if a panic occurs in the closure, or if a panic occurs while dropping a value, or if the
	/// `DrainFilter` itself is leaked.
	///
	/// # Example
	///
	/// Splitting a set into even and odd values, reusing the original set:
	///
	/// ```
	/// use btree_slab::BTreeSet;
	///
	/// let mut set: BTreeSet<i32> = (0..8).collect();
	/// let evens: BTreeSet<_> = set.drain_filter(|v| v % 2 == 0).collect();
	/// let odds = set;
	/// assert_eq!(evens.into_iter().collect::<Vec<_>>(), vec![0, 2, 4, 6]);
	/// assert_eq!(odds.into_iter().collect::<Vec<_>>(), vec![1, 3, 5, 7]);
	/// ```
	#[inline]
	pub fn drain_filter<'a, F>(&'a mut self, pred: F) -> DrainFilter<'a, T, C, F>
	where
		F: 'a + FnMut(&T) -> bool,
	{
		DrainFilter::new(self, pred)
	}
}

impl<T: Clone, C: Clone> Clone for BTreeSet<T, C> {
	#[inline]
	fn clone(&self) -> Self {
		BTreeSet {
			map: self.map.clone(),
		}
	}

	#[inline]
	fn clone_from(&mut self, other: &Self) {
		self.map.clone_from(&other.map);
	}
}

impl<T: Ord, C: SlabMut<Node<T, ()>> + Default> FromIterator<T> for BTreeSet<T, C>
where
	C: SimpleCollectionRef,
	C: SimpleCollectionMut,
{
	#[inline]
	fn from_iter<I>(iter: I) -> Self
	where
		I: IntoIterator<Item = T>,
	{
		let mut set = BTreeSet::new();
		set.extend(iter);
		set
	}
}

impl<T, C: SlabMut<Node<T, ()>>> IntoIterator for BTreeSet<T, C>
where
	C: SimpleCollectionRef,
	C: SimpleCollectionMut,
{
	type Item = T;
	type IntoIter = IntoIter<T, C>;

	#[inline]
	fn into_iter(self) -> IntoIter<T, C> {
		IntoIter {
			inner: self.map.into_keys(),
		}
	}
}

impl<'a, T, C: SlabMut<Node<T, ()>>> IntoIterator for &'a BTreeSet<T, C>
where
	C: SimpleCollectionRef,
{
	type Item = &'a T;
	type IntoIter = Iter<'a, T, C>;

	#[inline]
	fn into_iter(self) -> Iter<'a, T, C> {
		self.iter()
	}
}

impl<T: Ord, C: SlabMut<Node<T, ()>>> Extend<T> for BTreeSet<T, C>
where
	C: SimpleCollectionRef,
	C: SimpleCollectionMut,
{
	#[inline]
	fn extend<I>(&mut self, iter: I)
	where
		I: IntoIterator<Item = T>,
	{
		for t in iter {
			self.insert(t);
		}
	}
}

impl<'a, T: 'a + Ord + Copy, C: SlabMut<Node<T, ()>>> Extend<&'a T> for BTreeSet<T, C>
where
	C: SimpleCollectionRef,
	C: SimpleCollectionMut,
{
	#[inline]
	fn extend<I>(&mut self, iter: I)
	where
		I: IntoIterator<Item = &'a T>,
	{
		self.extend(iter.into_iter().copied())
	}
}

impl<T, L: PartialEq<T>, C: Slab<Node<T, ()>>, D: Slab<Node<L, ()>>> PartialEq<BTreeSet<L, D>>
	for BTreeSet<T, C>
where
	C: SimpleCollectionRef,
	D: SimpleCollectionRef,
{
	#[inline]
	fn eq(&self, other: &BTreeSet<L, D>) -> bool {
		self.map.eq(&other.map)
	}
}

impl<T: Eq, C: Slab<Node<T, ()>>> Eq for BTreeSet<T, C> where C: SimpleCollectionRef {}

impl<T, L: PartialOrd<T>, C: Slab<Node<T, ()>>, D: Slab<Node<L, ()>>> PartialOrd<BTreeSet<L, D>>
	for BTreeSet<T, C>
where
	C: SimpleCollectionRef,
	D: SimpleCollectionRef,
{
	#[inline]
	fn partial_cmp(&self, other: &BTreeSet<L, D>) -> Option<Ordering> {
		self.map.partial_cmp(&other.map)
	}
}

impl<T: Ord, C: Slab<Node<T, ()>>> Ord for BTreeSet<T, C>
where
	C: SimpleCollectionRef,
{
	#[inline]
	fn cmp(&self, other: &BTreeSet<T, C>) -> Ordering {
		self.map.cmp(&other.map)
	}
}

impl<T: Hash, C: Slab<Node<T, ()>>> Hash for BTreeSet<T, C>
where
	C: SimpleCollectionRef,
{
	#[inline]
	fn hash<H: Hasher>(&self, h: &mut H) {
		self.map.hash(h)
	}
}

pub struct Iter<'a, T, C> {
	inner: map::Keys<'a, T, (), C>,
}

impl<'a, T, C: Slab<Node<T, ()>>> Iterator for Iter<'a, T, C>
where
	C: SimpleCollectionRef,
{
	type Item = &'a T;

	#[inline]
	fn size_hint(&self) -> (usize, Option<usize>) {
		self.inner.size_hint()
	}

	#[inline]
	fn next(&mut self) -> Option<&'a T> {
		self.inner.next()
	}
}

impl<'a, T, C: Slab<Node<T, ()>>> DoubleEndedIterator for Iter<'a, T, C>
where
	C: SimpleCollectionRef,
{
	#[inline]
	fn next_back(&mut self) -> Option<&'a T> {
		self.inner.next_back()
	}
}

impl<'a, T, C: Slab<Node<T, ()>>> FusedIterator for Iter<'a, T, C> where C: SimpleCollectionRef {}
impl<'a, T, C: Slab<Node<T, ()>>> ExactSizeIterator for Iter<'a, T, C> where C: SimpleCollectionRef {}

pub struct IntoIter<T, C> {
	inner: map::IntoKeys<T, (), C>,
}

impl<T, C: SlabMut<Node<T, ()>>> Iterator for IntoIter<T, C>
where
	C: SimpleCollectionRef,
	C: SimpleCollectionMut,
{
	type Item = T;

	#[inline]
	fn size_hint(&self) -> (usize, Option<usize>) {
		self.inner.size_hint()
	}

	#[inline]
	fn next(&mut self) -> Option<T> {
		self.inner.next()
	}
}

impl<T, C: SlabMut<Node<T, ()>>> DoubleEndedIterator for IntoIter<T, C>
where
	C: SimpleCollectionRef,
	C: SimpleCollectionMut,
{
	#[inline]
	fn next_back(&mut self) -> Option<T> {
		self.inner.next_back()
	}
}

impl<T, C: SlabMut<Node<T, ()>>> FusedIterator for IntoIter<T, C>
where
	C: SimpleCollectionRef,
	C: SimpleCollectionMut,
{
}
impl<T, C: SlabMut<Node<T, ()>>> ExactSizeIterator for IntoIter<T, C>
where
	C: SimpleCollectionRef,
	C: SimpleCollectionMut,
{
}

pub struct Union<'a, T, C: Slab<Node<T, ()>>, D: Slab<Node<T, ()>>>
where
	C: SimpleCollectionRef,
	D: SimpleCollectionRef,
{
	it1: Peekable<Iter<'a, T, C>>,
	it2: Peekable<Iter<'a, T, D>>,
}

impl<'a, T: Ord, C: Slab<Node<T, ()>>, D: Slab<Node<T, ()>>> Iterator for Union<'a, T, C, D>
where
	C: SimpleCollectionRef,
	D: SimpleCollectionRef,
{
	type Item = &'a T;

	#[inline]
	fn size_hint(&self) -> (usize, Option<usize>) {
		let len1 = self.it1.len();
		let len2 = self.it2.len();

		(std::cmp::min(len1, len2), Some(std::cmp::max(len1, len2)))
	}

	#[inline]
	fn next(&mut self) -> Option<&'a T> {
		match (self.it1.peek(), self.it2.peek()) {
			(Some(v1), Some(v2)) => match v1.cmp(v2) {
				Ordering::Equal => {
					self.it1.next();
					self.it2.next()
				}
				Ordering::Less => self.it1.next(),
				Ordering::Greater => self.it2.next(),
			},
			(Some(_), None) => self.it1.next(),
			(None, Some(_)) => self.it2.next(),
			(None, None) => None,
		}
	}
}

impl<'a, T: Ord, C: Slab<Node<T, ()>>, D: Slab<Node<T, ()>>> FusedIterator for Union<'a, T, C, D>
where
	C: SimpleCollectionRef,
	D: SimpleCollectionRef,
{
}

pub struct Intersection<'a, T, C, D: Slab<Node<T, ()>>>
where
	D: SimpleCollectionRef,
{
	it1: Iter<'a, T, C>,
	it2: Peekable<Iter<'a, T, D>>,
}

impl<'a, T: Ord, C: Slab<Node<T, ()>>, D: Slab<Node<T, ()>>> Iterator for Intersection<'a, T, C, D>
where
	C: SimpleCollectionRef,
	D: SimpleCollectionRef,
{
	type Item = &'a T;

	#[inline]
	fn size_hint(&self) -> (usize, Option<usize>) {
		let len1 = self.it1.len();
		let len2 = self.it2.len();

		(0, Some(std::cmp::min(len1, len2)))
	}

	#[inline]
	fn next(&mut self) -> Option<&'a T> {
		loop {
			match self.it1.next() {
				Some(value) => {
					let keep = loop {
						match self.it2.peek() {
							Some(other) => match value.cmp(other) {
								Ordering::Equal => break true,
								Ordering::Greater => {
									self.it2.next();
								}
								Ordering::Less => break false,
							},
							None => break false,
						}
					};

					if keep {
						break Some(value);
					}
				}
				None => break None,
			}
		}
	}
}

impl<'a, T: Ord, C: Slab<Node<T, ()>>, D: Slab<Node<T, ()>>> FusedIterator
	for Intersection<'a, T, C, D>
where
	C: SimpleCollectionRef,
	D: SimpleCollectionRef,
{
}

pub struct Difference<'a, T, C, D: Slab<Node<T, ()>>>
where
	D: SimpleCollectionRef,
{
	it1: Iter<'a, T, C>,
	it2: Peekable<Iter<'a, T, D>>,
}

impl<'a, T: Ord, C: Slab<Node<T, ()>>, D: Slab<Node<T, ()>>> Iterator for Difference<'a, T, C, D>
where
	C: SimpleCollectionRef,
	D: SimpleCollectionRef,
{
	type Item = &'a T;

	#[inline]
	fn size_hint(&self) -> (usize, Option<usize>) {
		let len1 = self.it1.len();
		let len2 = self.it2.len();

		(len1.saturating_sub(len2), Some(self.it1.len()))
	}

	#[inline]
	fn next(&mut self) -> Option<&'a T> {
		loop {
			match self.it1.next() {
				Some(value) => {
					let keep = loop {
						match self.it2.peek() {
							Some(other) => match value.cmp(other) {
								Ordering::Equal => break false,
								Ordering::Greater => {
									self.it2.next();
								}
								Ordering::Less => break true,
							},
							None => break true,
						}
					};

					if keep {
						break Some(value);
					}
				}
				None => break None,
			}
		}
	}
}

impl<'a, T: Ord, C: Slab<Node<T, ()>>, D: Slab<Node<T, ()>>> FusedIterator
	for Difference<'a, T, C, D>
where
	C: SimpleCollectionRef,
	D: SimpleCollectionRef,
{
}

pub struct SymmetricDifference<'a, T, C: Slab<Node<T, ()>>, D: Slab<Node<T, ()>>>
where
	C: SimpleCollectionRef,
	D: SimpleCollectionRef,
{
	it1: Peekable<Iter<'a, T, C>>,
	it2: Peekable<Iter<'a, T, D>>,
}

impl<'a, T: Ord, C: Slab<Node<T, ()>>, D: Slab<Node<T, ()>>> Iterator
	for SymmetricDifference<'a, T, C, D>
where
	C: SimpleCollectionRef,
	D: SimpleCollectionRef,
{
	type Item = &'a T;

	#[inline]
	fn size_hint(&self) -> (usize, Option<usize>) {
		let len1 = self.it1.len();
		let len2 = self.it2.len();

		(0, len1.checked_add(len2))
	}

	#[inline]
	fn next(&mut self) -> Option<&'a T> {
		loop {
			match (self.it1.peek(), self.it2.peek()) {
				(Some(v1), Some(v2)) => match v1.cmp(v2) {
					Ordering::Equal => {
						self.it1.next();
						self.it2.next();
					}
					Ordering::Less => break self.it1.next(),
					Ordering::Greater => break self.it2.next(),
				},
				(Some(_), None) => break self.it1.next(),
				(None, Some(_)) => break self.it2.next(),
				(None, None) => break None,
			}
		}
	}
}

impl<'a, T: Ord, C: Slab<Node<T, ()>>, D: Slab<Node<T, ()>>> FusedIterator
	for SymmetricDifference<'a, T, C, D>
where
	C: SimpleCollectionRef,
	D: SimpleCollectionRef,
{
}

pub struct DrainFilter<'a, T, C: SlabMut<Node<T, ()>>, F>
where
	F: FnMut(&T) -> bool,
	C: SimpleCollectionRef,
	C: SimpleCollectionMut,
{
	pred: F,

	inner: map::DrainFilterInner<'a, T, (), C>,
}

impl<'a, T: 'a, C: SlabMut<Node<T, ()>>, F> DrainFilter<'a, T, C, F>
where
	F: FnMut(&T) -> bool,
	C: SimpleCollectionRef,
	C: SimpleCollectionMut,
{
	#[inline]
	pub fn new(set: &'a mut BTreeSet<T, C>, pred: F) -> Self {
		DrainFilter {
			pred,
			inner: map::DrainFilterInner::new(&mut set.map),
		}
	}
}

impl<'a, T, C: SlabMut<Node<T, ()>>, F> FusedIterator for DrainFilter<'a, T, C, F>
where
	F: FnMut(&T) -> bool,
	C: SimpleCollectionRef,
	C: SimpleCollectionMut,
{
}

impl<'a, T, C: SlabMut<Node<T, ()>>, F> Iterator for DrainFilter<'a, T, C, F>
where
	F: FnMut(&T) -> bool,
	C: SimpleCollectionRef,
	C: SimpleCollectionMut,
{
	type Item = T;

	#[inline]
	fn size_hint(&self) -> (usize, Option<usize>) {
		self.inner.size_hint()
	}

	#[inline]
	fn next(&mut self) -> Option<T> {
		let pred = &mut self.pred;
		self.inner.next(&mut |t, _| (*pred)(t)).map(|(t, ())| t)
	}
}

impl<'a, T, C: SlabMut<Node<T, ()>>, F> Drop for DrainFilter<'a, T, C, F>
where
	F: FnMut(&T) -> bool,
	C: SimpleCollectionRef,
	C: SimpleCollectionMut,
{
	fn drop(&mut self) {
		loop {
			if self.next().is_none() {
				break;
			}
		}
	}
}

pub struct Range<'a, T, C> {
	inner: map::Range<'a, T, (), C>,
}

impl<'a, T, C: Slab<Node<T, ()>>> Iterator for Range<'a, T, C>
where
	C: SimpleCollectionRef,
{
	type Item = &'a T;

	#[inline]
	fn size_hint(&self) -> (usize, Option<usize>) {
		self.inner.size_hint()
	}

	#[inline]
	fn next(&mut self) -> Option<&'a T> {
		self.inner.next().map(|(k, ())| k)
	}
}

impl<'a, T, C: Slab<Node<T, ()>>> DoubleEndedIterator for Range<'a, T, C>
where
	C: SimpleCollectionRef,
{
	#[inline]
	fn next_back(&mut self) -> Option<&'a T> {
		self.inner.next_back().map(|(k, ())| k)
	}
}

impl<'a, T, C: Slab<Node<T, ()>>> FusedIterator for Range<'a, T, C> where C: SimpleCollectionRef {}
