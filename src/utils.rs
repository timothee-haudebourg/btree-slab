use std::borrow::Borrow;
use crate::generic::node::Keyed;

/// Search in `vec` for the item with the nearest key smaller or equal to the given one.
///
/// `vec` is assumed to be sorted.
#[inline]
pub(crate) fn binary_search_min<T: Keyed, Q: ?Sized>(vec: &[T], key: &Q) -> Option<usize> where T::Key: Borrow<Q>, Q: Ord {
	if vec.is_empty() || vec[0].key().borrow() > key {
		None
	} else {
		let mut i = 0;
		let mut j = vec.len() - 1;

		if vec[j].key().borrow() <= key {
			return Some(j)
		}

		// invariants:
		// vec[i].key <= key
		// vec[j].key > key
		// j > i

		while j-i > 1 {
			let k = (i + j) / 2;

			if vec[k].key().borrow() > key {
				j = k;
				// vec[k].key > key --> vec[j] > key
			} else {
				i = k;
				// vec[k].key <= key --> vec[i] <= key
			}
		}

		Some(i)
	}
}
