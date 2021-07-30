#![feature(is_sorted)]
#![feature(trait_alias)]

use slab::Slab;

pub mod utils;
pub mod generic;

/// B-Tree map based on `Slab`.
pub type BTreeMap<K, V> = generic::BTreeMap<K, V, Slab<generic::Node<K, V>>>;

/// B-Tree set based on `Slab`.
pub type BTreeSet<T> = generic::BTreeSet<T, Slab<generic::Node<T, ()>>>;