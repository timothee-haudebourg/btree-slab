use slab::Slab;

pub trait Container<T> {
	fn get(&self, id: usize) -> Option<&T>;
}

pub trait ContainerMut<T>: Container<T> {
	fn clear(&mut self);

	fn get_mut(&mut self, id: usize) -> Option<&mut T>;

	fn insert(&mut self, t: T) -> usize;

	fn remove(&mut self, id: usize) -> T;
}

impl<'a, T, C: Container<T>> Container<T> for &'a C {
	fn get(&self, id: usize) -> Option<&T> {
		C::get(*self, id)
	}
}

impl<'a, T, C: Container<T>> Container<T> for &'a mut C {
	fn get(&self, id: usize) -> Option<&T> {
		C::get(*self, id)
	}
}

impl<'a, T, C: ContainerMut<T>> ContainerMut<T> for &'a mut C {
	fn clear(&mut self) {
		C::clear(*self)
	}

	fn get_mut(&mut self, id: usize) -> Option<&mut T> {
		C::get_mut(*self, id)
	}

	fn insert(&mut self, t: T) -> usize {
		C::insert(*self, t)
	}

	fn remove(&mut self, id: usize) -> T {
		C::remove(*self, id)
	}
}

impl<T> Container<T> for Slab<T> {
	fn get(&self, id: usize) -> Option<&T> {
		self.get(id)
	}
}

impl<T> ContainerMut<T> for Slab<T> {
	fn clear(&mut self) {
		self.clear()
	}

	fn get_mut(&mut self, id: usize) -> Option<&mut T> {
		self.get_mut(id)
	}

	fn insert(&mut self, t: T) -> usize {
		self.insert(t)
	}

	fn remove(&mut self, id: usize) -> T {
		self.remove(id)
	}
}