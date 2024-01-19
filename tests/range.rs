use btree_slab::BTreeMap;

use core::borrow::Borrow;
use core::cmp::Ordering;
use core::ops::Bound;
use core::ops::RangeBounds;
use std::ops::Bound::{Excluded, Included, Unbounded};
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

fn range_keys(map: &BTreeMap<i32, i32>, range: impl RangeBounds<i32>) -> Vec<i32> {
	Vec::from_iter(map.range(range).map(|(&k, &v)| {
		assert_eq!(k, v);
		k
	}))
}

#[test]
fn test_range_small() {
	let size = 4;

	let all = Vec::from_iter(1..=size);
	let (first, last) = (vec![all[0]], vec![all[size as usize - 1]]);
	let map = BTreeMap::from_iter(all.iter().copied().map(|i| (i, i)));

	assert_eq!(range_keys(&map, (Excluded(0), Excluded(size + 1))), all);
	assert_eq!(range_keys(&map, (Excluded(0), Included(size + 1))), all);
	assert_eq!(range_keys(&map, (Excluded(0), Included(size))), all);
	assert_eq!(range_keys(&map, (Excluded(0), Unbounded)), all);
	assert_eq!(range_keys(&map, (Included(0), Excluded(size + 1))), all);
	assert_eq!(range_keys(&map, (Included(0), Included(size + 1))), all);
	assert_eq!(range_keys(&map, (Included(0), Included(size))), all);
	assert_eq!(range_keys(&map, (Included(0), Unbounded)), all);
	assert_eq!(range_keys(&map, (Included(1), Excluded(size + 1))), all);
	assert_eq!(range_keys(&map, (Included(1), Included(size + 1))), all);
	assert_eq!(range_keys(&map, (Included(1), Included(size))), all);
	assert_eq!(range_keys(&map, (Included(1), Unbounded)), all);
	assert_eq!(range_keys(&map, (Unbounded, Excluded(size + 1))), all);
	assert_eq!(range_keys(&map, (Unbounded, Included(size + 1))), all);
	assert_eq!(range_keys(&map, (Unbounded, Included(size))), all);
	assert_eq!(range_keys(&map, ..), all);

	assert_eq!(range_keys(&map, (Excluded(0), Excluded(1))), vec![]);
	assert_eq!(range_keys(&map, (Excluded(0), Included(0))), vec![]);
	assert_eq!(range_keys(&map, (Included(0), Included(0))), vec![]);
	assert_eq!(range_keys(&map, (Included(0), Excluded(1))), vec![]);
	assert_eq!(range_keys(&map, (Unbounded, Excluded(1))), vec![]);
	assert_eq!(range_keys(&map, (Unbounded, Included(0))), vec![]);
	assert_eq!(range_keys(&map, (Excluded(0), Excluded(2))), first);
	assert_eq!(range_keys(&map, (Excluded(0), Included(1))), first);
	assert_eq!(range_keys(&map, (Included(0), Excluded(2))), first);
	assert_eq!(range_keys(&map, (Included(0), Included(1))), first);
	assert_eq!(range_keys(&map, (Included(1), Excluded(2))), first);
	assert_eq!(range_keys(&map, (Included(1), Included(1))), first);
	assert_eq!(range_keys(&map, (Unbounded, Excluded(2))), first);
	assert_eq!(range_keys(&map, (Unbounded, Included(1))), first);
	assert_eq!(
		range_keys(&map, (Excluded(size - 1), Excluded(size + 1))),
		last
	);
	assert_eq!(
		range_keys(&map, (Excluded(size - 1), Included(size + 1))),
		last
	);
	assert_eq!(range_keys(&map, (Excluded(size - 1), Included(size))), last);
	assert_eq!(range_keys(&map, (Excluded(size - 1), Unbounded)), last);
	assert_eq!(range_keys(&map, (Included(size), Excluded(size + 1))), last);
	assert_eq!(range_keys(&map, (Included(size), Included(size + 1))), last);
	assert_eq!(range_keys(&map, (Included(size), Included(size))), last);
	assert_eq!(range_keys(&map, (Included(size), Unbounded)), last);
	assert_eq!(
		range_keys(&map, (Excluded(size), Excluded(size + 1))),
		vec![]
	);
	assert_eq!(range_keys(&map, (Excluded(size), Included(size))), vec![]);
	assert_eq!(range_keys(&map, (Excluded(size), Unbounded)), vec![]);
	assert_eq!(
		range_keys(&map, (Included(size + 1), Excluded(size + 1))),
		vec![]
	);
	assert_eq!(
		range_keys(&map, (Included(size + 1), Included(size + 1))),
		vec![]
	);
	assert_eq!(range_keys(&map, (Included(size + 1), Unbounded)), vec![]);

	assert_eq!(range_keys(&map, ..3), vec![1, 2]);
	assert_eq!(range_keys(&map, 3..), vec![3, 4]);
	assert_eq!(range_keys(&map, 2..=3), vec![2, 3]);
}

#[test]
fn test_range_large() {
	let size = 200;

	let all = Vec::from_iter(1..=size);
	let (first, last) = (vec![all[0]], vec![all[size as usize - 1]]);
	let map = BTreeMap::from_iter(all.iter().copied().map(|i| (i, i)));

	assert_eq!(range_keys(&map, (Excluded(0), Excluded(size + 1))), all);
	assert_eq!(range_keys(&map, (Excluded(0), Included(size + 1))), all);
	assert_eq!(range_keys(&map, (Excluded(0), Included(size))), all);
	assert_eq!(range_keys(&map, (Excluded(0), Unbounded)), all);
	assert_eq!(range_keys(&map, (Included(0), Excluded(size + 1))), all);
	assert_eq!(range_keys(&map, (Included(0), Included(size + 1))), all);
	assert_eq!(range_keys(&map, (Included(0), Included(size))), all);
	assert_eq!(range_keys(&map, (Included(0), Unbounded)), all);
	assert_eq!(range_keys(&map, (Included(1), Excluded(size + 1))), all);
	assert_eq!(range_keys(&map, (Included(1), Included(size + 1))), all);
	assert_eq!(range_keys(&map, (Included(1), Included(size))), all);
	assert_eq!(range_keys(&map, (Included(1), Unbounded)), all);
	assert_eq!(range_keys(&map, (Unbounded, Excluded(size + 1))), all);
	assert_eq!(range_keys(&map, (Unbounded, Included(size + 1))), all);
	assert_eq!(range_keys(&map, (Unbounded, Included(size))), all);
	assert_eq!(range_keys(&map, ..), all);

	assert_eq!(range_keys(&map, (Excluded(0), Excluded(1))), vec![]);
	assert_eq!(range_keys(&map, (Excluded(0), Included(0))), vec![]);
	assert_eq!(range_keys(&map, (Included(0), Included(0))), vec![]);
	assert_eq!(range_keys(&map, (Included(0), Excluded(1))), vec![]);
	assert_eq!(range_keys(&map, (Unbounded, Excluded(1))), vec![]);
	assert_eq!(range_keys(&map, (Unbounded, Included(0))), vec![]);
	assert_eq!(range_keys(&map, (Excluded(0), Excluded(2))), first);
	assert_eq!(range_keys(&map, (Excluded(0), Included(1))), first);
	assert_eq!(range_keys(&map, (Included(0), Excluded(2))), first);
	assert_eq!(range_keys(&map, (Included(0), Included(1))), first);
	assert_eq!(range_keys(&map, (Included(1), Excluded(2))), first);
	assert_eq!(range_keys(&map, (Included(1), Included(1))), first);
	assert_eq!(range_keys(&map, (Unbounded, Excluded(2))), first);
	assert_eq!(range_keys(&map, (Unbounded, Included(1))), first);
	assert_eq!(
		range_keys(&map, (Excluded(size - 1), Excluded(size + 1))),
		last
	);
	assert_eq!(
		range_keys(&map, (Excluded(size - 1), Included(size + 1))),
		last
	);
	assert_eq!(range_keys(&map, (Excluded(size - 1), Included(size))), last);
	assert_eq!(range_keys(&map, (Excluded(size - 1), Unbounded)), last);
	assert_eq!(range_keys(&map, (Included(size), Excluded(size + 1))), last);
	assert_eq!(range_keys(&map, (Included(size), Included(size + 1))), last);
	assert_eq!(range_keys(&map, (Included(size), Included(size))), last);
	assert_eq!(range_keys(&map, (Included(size), Unbounded)), last);
	assert_eq!(
		range_keys(&map, (Excluded(size), Excluded(size + 1))),
		vec![]
	);
	assert_eq!(range_keys(&map, (Excluded(size), Included(size))), vec![]);
	assert_eq!(range_keys(&map, (Excluded(size), Unbounded)), vec![]);
	assert_eq!(
		range_keys(&map, (Included(size + 1), Excluded(size + 1))),
		vec![]
	);
	assert_eq!(
		range_keys(&map, (Included(size + 1), Included(size + 1))),
		vec![]
	);
	assert_eq!(range_keys(&map, (Included(size + 1), Unbounded)), vec![]);

	fn check<'a, L, R>(lhs: L, rhs: R)
	where
		L: IntoIterator<Item = (&'a i32, &'a i32)>,
		R: IntoIterator<Item = (&'a i32, &'a i32)>,
	{
		assert_eq!(Vec::from_iter(lhs), Vec::from_iter(rhs));
	}

	check(map.range(..=100), map.range(..101));
	check(
		map.range(5..=8),
		vec![(&5, &5), (&6, &6), (&7, &7), (&8, &8)],
	);
	check(map.range(-1..=2), vec![(&1, &1), (&2, &2)]);
}

#[test]
fn test_range_inclusive_max_value() {
	let max = usize::MAX;
	let mut map = BTreeMap::new();
	map.insert(max, 0);
	assert_eq!(Vec::from_iter(map.range(max..=max)), &[(&max, &0)]);
}

#[test]
fn test_range_equal_empty_cases() {
	let map = BTreeMap::from_iter((0..5).map(|i| (i, i)));
	assert_eq!(map.range((Included(2), Excluded(2))).next(), None);
	assert_eq!(map.range((Excluded(2), Included(2))).next(), None);
}

#[test]
#[should_panic]
fn test_range_equal_excluded() {
	let map = BTreeMap::from_iter((0..5).map(|i| (i, i)));
	let _ = map.range((Excluded(2), Excluded(2)));
}

#[test]
#[should_panic]
fn test_range_backwards_1() {
	let map = BTreeMap::from_iter((0..5).map(|i| (i, i)));
	let _ = map.range((Included(3), Included(2)));
}

#[test]
#[should_panic]
fn test_range_backwards_2() {
	let map = BTreeMap::from_iter((0..5).map(|i| (i, i)));
	let _ = map.range((Included(3), Excluded(2)));
}

#[test]
#[should_panic]
fn test_range_backwards_3() {
	let map = BTreeMap::from_iter((0..5).map(|i| (i, i)));
	let _ = map.range((Excluded(3), Included(2)));
}

#[test]
#[should_panic]
fn test_range_backwards_4() {
	let map = BTreeMap::from_iter((0..5).map(|i| (i, i)));
	let _ = map.range((Excluded(3), Excluded(2)));
}

#[test]
fn test_range_finding_ill_order_in_range_ord() {
	// Has proper order the first time asked, then flips around.
	struct EvilTwin(i32);

	impl PartialOrd for EvilTwin {
		fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
			Some(self.cmp(other))
		}
	}

	static COMPARES: AtomicUsize = AtomicUsize::new(0);
	impl Ord for EvilTwin {
		fn cmp(&self, other: &Self) -> Ordering {
			let ord = self.0.cmp(&other.0);
			if COMPARES.fetch_add(1, SeqCst) > 0 {
				ord.reverse()
			} else {
				ord
			}
		}
	}

	impl PartialEq for EvilTwin {
		fn eq(&self, other: &Self) -> bool {
			self.0.eq(&other.0)
		}
	}

	impl Eq for EvilTwin {}

	#[derive(PartialEq, Eq, PartialOrd, Ord)]
	struct CompositeKey(i32, EvilTwin);

	impl Borrow<EvilTwin> for CompositeKey {
		fn borrow(&self) -> &EvilTwin {
			&self.1
		}
	}

	let map = BTreeMap::from_iter((0..12).map(|i| (CompositeKey(i, EvilTwin(i)), ())));
	let _ = map.range(EvilTwin(5)..=EvilTwin(7));
}

#[test]
fn test_range_1000() {
	let size = 1000;
	let map = BTreeMap::from_iter((0..size).map(|i| (i, i)));

	fn test(map: &BTreeMap<u32, u32>, size: u32, min: Bound<&u32>, max: Bound<&u32>) {
		let mut kvs = map.range((min, max)).map(|(&k, &v)| (k, v));
		let mut pairs = (0..size).map(|i| (i, i));

		for (kv, pair) in kvs.by_ref().zip(pairs.by_ref()) {
			assert_eq!(kv, pair);
		}
		assert_eq!(kvs.next(), None);
		assert_eq!(pairs.next(), None);
	}
	test(&map, size, Included(&0), Excluded(&size));
	test(&map, size, Unbounded, Excluded(&size));
	test(&map, size, Included(&0), Included(&(size - 1)));
	test(&map, size, Unbounded, Included(&(size - 1)));
	test(&map, size, Included(&0), Unbounded);
	test(&map, size, Unbounded, Unbounded);
}

#[test]
fn test_range_borrowed_key() {
	let mut map = BTreeMap::new();
	map.insert("aardvark".to_string(), 1);
	map.insert("baboon".to_string(), 2);
	map.insert("coyote".to_string(), 3);
	map.insert("dingo".to_string(), 4);
	// NOTE: would like to use simply "b".."d" here...
	let mut iter = map.range::<str, _>((Included("b"), Excluded("d")));
	assert_eq!(iter.next(), Some((&"baboon".to_string(), &2)));
	assert_eq!(iter.next(), Some((&"coyote".to_string(), &3)));
	assert_eq!(iter.next(), None);
}

#[test]
fn test_range() {
	let size = 200;
	// Miri is too slow
	let step = if cfg!(miri) { 66 } else { 1 };
	let map = BTreeMap::from_iter((0..size).map(|i| (i, i)));

	for i in (0..size).step_by(step) {
		for j in (i..size).step_by(step) {
			let mut kvs = map
				.range((Included(&i), Included(&j)))
				.map(|(&k, &v)| (k, v));
			let mut pairs = (i..=j).map(|i| (i, i));

			for (kv, pair) in kvs.by_ref().zip(pairs.by_ref()) {
				assert_eq!(kv, pair);
			}
			assert_eq!(kvs.next(), None);
			assert_eq!(pairs.next(), None);
		}
	}
}

#[test]
fn test_range_mut() {
	let size = 200;
	// Miri is too slow
	let step = if cfg!(miri) { 66 } else { 1 };
	let mut map = BTreeMap::from_iter((0..size).map(|i| (i, i)));

	for i in (0..size).step_by(step) {
		for j in (i..size).step_by(step) {
			let mut kvs = map
				.range_mut((Included(&i), Included(&j)))
				.map(|(&k, &mut v)| (k, v));
			let mut pairs = (i..=j).map(|i| (i, i));

			for (kv, pair) in kvs.by_ref().zip(pairs.by_ref()) {
				assert_eq!(kv, pair);
			}
			assert_eq!(kvs.next(), None);
			assert_eq!(pairs.next(), None);
		}
	}
	//	map.check();
}

#[should_panic]
#[test]
fn test_range_panic_1() {
	let mut map = BTreeMap::new();
	map.insert(3, "a");
	map.insert(5, "b");
	map.insert(8, "c");

	let _invalid_range = map.range((Included(&8), Included(&3)));
}

#[should_panic]
#[test]
fn test_range_panic_2() {
	let mut map = BTreeMap::new();
	map.insert(3, "a");
	map.insert(5, "b");
	map.insert(8, "c");

	let _invalid_range = map.range((Excluded(&5), Excluded(&5)));
}

#[should_panic]
#[test]
fn test_range_panic_3() {
	let mut map: BTreeMap<i32, ()> = BTreeMap::new();
	map.insert(3, ());
	map.insert(5, ());
	map.insert(8, ());

	let _invalid_range = map.range((Excluded(&5), Excluded(&5)));
}
