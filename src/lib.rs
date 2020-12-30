#![feature(min_const_generics)]
#![feature(is_sorted)]
#![feature(maybe_uninit_ref)]

use slab::Slab;

pub(crate) mod utils;
mod container;
pub mod generic;

pub use container::{
	Container,
	ContainerMut
};

/// B-Tree map based on `Slab`.
pub type BTreeMap<K, V> = generic::BTreeMap<K, V, Slab<generic::Node<K, V>>>;

/// B-Tree set based on `Slab`.
pub type BTreeSet<T> = generic::BTreeSet<T, Slab<generic::Node<T, ()>>>;