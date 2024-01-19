use btree_slab::BTreeMap;
use std::{cell::Cell, rc::Rc};

#[test]
pub fn iter() {
	let mut map = BTreeMap::new();
	for i in 0..10 {
		map.insert(i, i);
	}

	let mut i = 0;
	for (key, _) in &map {
		assert_eq!(*key, i);
		i += 1;
	}

	assert_eq!(i, 10)
}

#[test]
pub fn into_iter() {
	struct Element {
		/// Drop counter.
		counter: Rc<Cell<usize>>,
		value: i32,
	}

	impl Element {
		pub fn new(counter: &Rc<Cell<usize>>, value: i32) -> Self {
			Element {
				counter: counter.clone(),
				value,
			}
		}

		pub fn inner(&self) -> i32 {
			self.value
		}
	}

	impl Drop for Element {
		fn drop(&mut self) {
			let c = self.counter.get();
			self.counter.set(c + 1);
		}
	}

	let counter = Rc::new(Cell::new(0));
	let mut map = BTreeMap::new();
	for i in 0..100 {
		map.insert(i, Element::new(&counter, i));
	}

	for (key, value) in map {
		assert_eq!(key, value.inner());
	}

	assert_eq!(counter.get(), 100);
}

#[test]
pub fn into_iter_rev() {
	struct Element {
		/// Drop counter.
		counter: Rc<Cell<usize>>,
		value: i32,
	}

	impl Element {
		pub fn new(counter: &Rc<Cell<usize>>, value: i32) -> Self {
			Element {
				counter: counter.clone(),
				value,
			}
		}

		pub fn inner(&self) -> i32 {
			self.value
		}
	}

	impl Drop for Element {
		fn drop(&mut self) {
			let c = self.counter.get();
			self.counter.set(c + 1);
		}
	}

	let counter = Rc::new(Cell::new(0));
	let mut map = BTreeMap::new();
	for i in 0..100 {
		map.insert(i, Element::new(&counter, i));
	}

	for (key, value) in map.into_iter().rev() {
		assert_eq!(key, value.inner());
	}

	assert_eq!(counter.get(), 100);
}

#[test]
pub fn into_iter_both_ends1() {
	struct Element {
		/// Drop counter.
		counter: Rc<Cell<usize>>,
		value: i32,
	}

	impl Element {
		pub fn new(counter: &Rc<Cell<usize>>, value: i32) -> Self {
			Element {
				counter: counter.clone(),
				value,
			}
		}

		pub fn inner(&self) -> i32 {
			self.value
		}
	}

	impl Drop for Element {
		fn drop(&mut self) {
			let c = self.counter.get();
			self.counter.set(c + 1);
		}
	}

	let counter = Rc::new(Cell::new(0));
	let mut map = BTreeMap::new();
	for i in 0..100 {
		map.insert(i, Element::new(&counter, i));
	}

	let mut it = map.into_iter();
	while let Some((key, value)) = it.next() {
		assert_eq!(key, value.inner());

		let (key, value) = it.next_back().unwrap();
		assert_eq!(key, value.inner());
	}

	assert_eq!(counter.get(), 100);
}

#[test]
pub fn into_iter_both_ends2() {
	struct Element {
		/// Drop counter.
		counter: Rc<Cell<usize>>,
		value: i32,
	}

	impl Element {
		pub fn new(counter: &Rc<Cell<usize>>, value: i32) -> Self {
			Element {
				counter: counter.clone(),
				value,
			}
		}

		pub fn inner(&self) -> i32 {
			self.value
		}
	}

	impl Drop for Element {
		fn drop(&mut self) {
			let c = self.counter.get();
			self.counter.set(c + 1);
		}
	}

	let counter = Rc::new(Cell::new(0));
	let mut map = BTreeMap::new();
	for i in 0..100 {
		map.insert(i, Element::new(&counter, i));
	}

	let mut it = map.into_iter();
	while let Some((key, value)) = it.next_back() {
		assert_eq!(key, value.inner());

		let (key, value) = it.next().unwrap();
		assert_eq!(key, value.inner());
	}

	assert_eq!(counter.get(), 100);
}

#[test]
pub fn range_next_back() {
	let mut map = BTreeMap::new();
	for i in 0..100 {
		map.insert(i, i);
	}

	let mut it = map.range(..);
	while let Some((key, value)) = it.next_back() {
		assert_eq!(key, value);
	}
}
