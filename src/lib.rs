#![feature(min_const_generics)]
#![feature(is_sorted)]
#![feature(maybe_uninit_ref)]

use slab::Slab;

pub mod utils;
mod container;
pub mod generic;

pub use container::{
	Container,
	ContainerMut
};

/// B-Tree based on `Slab`.
pub type BTreeMap<K, V> = generic::BTreeMap<K, V, Slab<generic::Node<K, V>>>;