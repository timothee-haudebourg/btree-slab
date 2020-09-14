/// Search in `vec` for the item with the nearest key smaller than the given one.
///
/// `vec` is assumed to be sorted.
#[inline]
pub fn binary_search_min<T, K>(vec: &[T], key: &K) -> Option<usize> where T: PartialOrd<K> {
	if vec.is_empty() || &vec[0] > key {
		None
	} else {
		let mut i = 0;
		let mut j = vec.len();

		while i != j {
			let k = (i + j) / 2;

			if &vec[k] > key {
				j = k;
			} else {
				i = k;
			}
		}

		Some(i)
	}
}
