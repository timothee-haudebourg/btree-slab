use std::{
	rc::Rc,
	cell::Cell
};
use local_btree::BTreeMap;

#[test]
pub fn iter() {
	let mut map = BTreeMap::new();
	for i in 0..10 {
		map.insert(i, i);
	}

	let mut i = 0;
	for (key, value) in &map {
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
		value: i32
	}

	impl Element {
		pub fn new(counter: &Rc<Cell<usize>>, value: i32) -> Self {
			Element {
				counter: counter.clone(),
				value
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