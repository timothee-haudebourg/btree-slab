/// Search in `vec` for the item with the nearest key smaller or equal to the given one.
///
/// `vec` is assumed to be sorted.
#[inline]
pub fn binary_search_min<T, K>(vec: &[T], key: &K) -> Option<usize> where T: PartialOrd<K> {
	if vec.is_empty() || &vec[0] > key {
		None
	} else {
		let mut i = 0;
		let mut j = vec.len() - 1;

		if &vec[j] <= key {
			return Some(j)
		}

		// invariants:
		// vec[i] <= key
		// vec[j] > key
		// j > i

		while j-i > 1 {
			let k = (i + j) / 2;

			if &vec[k] > key {
				j = k;
				// vec[k] > key --> vec[j] > key
			} else {
				i = k;
				// vec[k] <= key --> vec[i] <= key
			}
		}

		Some(i)
	}
}
