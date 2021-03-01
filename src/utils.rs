use std::borrow::Borrow;
use crate::generic::node::Keyed;

/// Search in `sorted_slice` for the item with the nearest key smaller or equal to the given one.
///
/// `sorted_slice` is assumed to be sorted.
#[inline]
pub fn binary_search_min<T: Keyed, Q: ?Sized>(sorted_slice: &[T], key: &Q) -> Option<usize> where T::Key: Borrow<Q>, Q: Ord {
	if sorted_slice.is_empty() || sorted_slice[0].key().borrow() > key {
		None
	} else {
		let mut i = 0;
		let mut j = sorted_slice.len() - 1;

		if sorted_slice[j].key().borrow() <= key {
			return Some(j)
		}

		// invariants:
		// sorted_slice[i].key <= key
		// sorted_slice[j].key > key
		// j > i

		while j-i > 1 {
			let k = (i + j) / 2;

			if sorted_slice[k].key().borrow() > key {
				j = k;
				// sorted_slice[k].key > key --> sorted_slice[j] > key
			} else {
				i = k;
				// sorted_slice[k].key <= key --> sorted_slice[i] <= key
			}
		}

		Some(i)
	}
}
