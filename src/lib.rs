#![feature(min_const_generics)]
#![feature(move_ref_pattern)]
#![feature(is_sorted)]
use slab::Slab;
use staticvec::StaticVec;

pub mod utils;
mod node;
mod item;

pub use node::*;
pub use item::*;

const M: usize = 6;

/// Extension methods.
///
/// This trait can be imported to access the internal methods of the B-Tree.
/// These methods are not intended to be directly called by users, but can be used to
/// extends the data structure with new functionalities.
pub trait BTreeExt<K, V> {
	/// Set the new known number of items in the tree.
	fn set_len(&mut self, len: usize);

	/// Get the root node id.
	///
	/// Returns `None` if the tree is empty.
	fn root_id(&self) -> Option<usize>;

	fn set_root_id(&mut self, id: Option<usize>);

	/// Get the node associated to the given `id`.
	///
	/// Panics if `id` is out of bounds.
	fn node(&self, id: usize) -> &Node<K, V>;

	/// Get the node associated to the given `id` mutabily.
	///
	/// Panics if `id` is out of bounds.
	fn node_mut(&mut self, id: usize) -> &mut Node<K, V>;

	fn get_in(&self, key: &K, id: usize) -> Option<&V> where K: Ord;

	fn get_mut_in(&mut self, key: &K, id: usize) -> Option<&mut V> where K: Ord;

	/// Insert a key-value pair in an internal or leaf node.
	///
	/// It is assumed that there is still one free space in the node.
	fn insert_into(&mut self, node_id: usize, key: K, value: V) -> (Option<V>, ItemAddr) where K: Ord;

	/// Rebalance a node, if necessary.
	fn rebalance(&mut self, node_id: usize, addr: ItemAddr) -> ItemAddr;

	/// Remove the item matching the given key from the given node `node_id`.
	fn remove_from(&mut self, node_id: usize, key: &K) -> Option<(V, ItemAddr)> where K: Ord;

	// /// Update a value in the given node `node_id`.
	// fn update_in<T, F>(&mut self, key: K, action: F, id: usize) -> (T, Balance) where K: Ord, F: FnOnce(Option<V>) -> (Option<V>, T);

	/// Take the right-most leaf value in the given node.
	fn remove_rightmost_leaf_of(&mut self, node_id: usize) -> (Item<K, V>, usize);

	/// Try to rotate left the node `node_id` to benefits the child number `deficient_child_index`.
	///
	/// Returns true if the rotation succeeded, of false if the target child has no right sibling,
	/// or if this sibling would underflow.
	fn try_rotate_left(&mut self, node_id: usize, deficient_child_index: usize) -> bool;

	/// Try to rotate right the node `node_id` to benefits the child number `deficient_child_index`.
	///
	/// Returns true if the rotation succeeded, of false if the target child has no left sibling,
	/// or if this sibling would underflow.
	fn try_rotate_right(&mut self, node_id: usize, deficient_child_index: usize) -> bool;

	/// Merge the `deficient_child_index` child of `node_id` with its left or right sibling.
	///
	/// It is assumed that both siblings of the child `deficient_child_index` can be used for
	/// merging: the resulting merged node should not overflow.
	///
	/// Returns the balance of `node_id` after the merge (it may underflow).
	fn merge(&mut self, node_id: usize, deficient_child_index: usize) -> Balance;

	/// Allocate a free identifier for the given node.
	fn allocate_node(&mut self, node: Node<K, V>) -> usize;

	/// Release the given node identifier and return the node it used to identify.
	fn release_node(&mut self, id: usize) -> Node<K, V>;

	/// Validate the tree.
	///
	/// Panics if the tree is not a valid B-Tree.
	#[cfg(debug_assertions)]
	fn validate(&self) where K: Ord;

	/// Validate the given node and returns the depth of the node.
	///
	/// Panics if the tree is not a valid B-Tree.
	#[cfg(debug_assertions)]
	fn validate_node(&self, id: usize, parent: Option<usize>, min: Option<&K>, max: Option<&K>) -> usize where K: Ord;
}

/// B-tree of Knuth order `M` storing keys of type `K` associated to values of type `V`.
///
/// The Knuth order must be at least 4.
pub struct BTreeMap<K, V> {
	/// Allocated and free nodes.
	nodes: Slab<Node<K, V>>,

	/// Root node id.
	root: Option<usize>,

	len: usize
}

impl<K, V> BTreeMap<K, V> {
	/// Create a new empty B-tree.
	pub fn new() -> BTreeMap<K, V> {
		assert!(M >= 4);
		BTreeMap {
			nodes: Slab::new(),
			root: None,
			len: 0
		}
	}

	#[inline]
	pub fn is_empty(&self) -> bool {
		self.root.is_none()
	}

	#[inline]
	pub fn len(&self) -> usize {
		self.len
	}

	#[inline]
	pub fn get(&self, key: &K) -> Option<&V> where K: Ord {
		match self.root {
			Some(id) => self.get_in(key, id),
			None => None
		}
	}

	#[inline]
	pub fn get_mut(&mut self, key: &K) -> Option<&mut V> where K: Ord {
		match self.root {
			Some(id) => self.get_mut_in(key, id),
			None => None
		}
	}

	#[inline]
	pub fn contains(&self, key: &K) -> bool where K: Ord {
		self.get(key).is_some()
	}

	/// Insert a key-value pair in the tree.
	#[inline]
	pub fn insert(&mut self, key: K, value: V) -> Option<V> where K: Ord {
		match self.root {
			Some(id) => {
				let (old_value, _) = self.insert_into(id, key, value);
				old_value
			},
			None => {
				let new_root = Node::leaf(None, Item { key, value });
				self.root = Some(self.allocate_node(new_root));
				self.len += 1;
				None
			}
		}
	}

	// /// Insert an item at the given address.
	// /// Return the address of the inserted item in the tree
	// /// (it may differ from the input address if the tree is rebalanced).
	// ///
	// /// ## Correctness
	// /// It is assumed that it is btree-correct to insert the given item at the given address.
	// fn insert_at(&mut self, addr: ItemAddr, item: Item<K, V>, opt_right_id: Option<usize>) -> ItemAddr {
	// 	match self.node_mut(addr.id).insert(addr.offset, item, opt_right_id) {
	// 		Balance::Balanced => {
	// 			addr
	// 		},
	// 		Balance::Overflow => {
	// 			match self.node_mut(addr.id).split() {
	// 				Ok((left_node_len, median, right_node)) => {
	// 					let right_id = self.allocate_node(right_node);
	//
	// 					let new_addr = if addr.offset == left_node_len {
	// 						// item is median.
	// 						None // we don't know the median address yet.
	// 					} else if addr.offset > left_node_len {
	// 						// item is moved on right_node.
	// 						Some(ItemAddr {
	// 							id: right_id,
	// 							offset: addr.offset - left_node_len - 1
	// 						})
	// 					} else {
	// 						// item hasn't moved.
	// 						Some(addr)
	// 					};
	//
	// 					let median_addr = match self.node(addr.id).parent() {
	// 						Some(parent_id) => {
	// 							let offset = self.node(parent_id).child_index(addr.id).unwrap();
	// 							let addr = ItemAddr {
	// 								id: parent_id,
	// 								offset
	// 							};
	// 							self.insert_at(addr, item, Some(right_id))
	// 						},
	// 						None => {
	// 							let left_id = addr.id;
	// 							let new_root = Node::binary(None, left_id, median, right_id);
	// 							let id = self.allocate_node(new_root);
	//
	// 							self.root = Some(id);
	// 							self.nodes[left_id].set_parent(self.root);
	// 							self.nodes[right_id].set_parent(self.root);
	//
	// 							ItemAddr {
	// 								id,
	// 								offset: 0
	// 							}
	// 						}
	// 					};
	//
	// 					match new_addr {
	// 						Some(addr) => addr,
	// 						None => median_addr
	// 					}
	// 				}
	// 			}
	// 		},
	// 		Balance::Underflow(_) => unreachable!()
	// 	}
	// }

	fn before(&self, id: usize, offset: usize) -> Option<ItemAddr> {
		panic!("TODO")
	}

	fn after(&self, id: usize, offset: usize) -> Option<ItemAddr> {
		panic!("TODO")
	}

	// Delete an item by key.
	#[inline]
	pub fn remove(&mut self, key: &K) -> Option<V> where K: Ord {
		match self.root {
			Some(id) => match self.remove_from(id, key) {
				Some((value, _)) => Some(value),
				None => None
			},
			None => None
		}
	}

	/// Remove the item at the given address.
	/// Return the address of the next item.
	/// All other addresses are to be considered invalid.
	///
	/// It is assumed that this item exists.
	#[inline]
	fn remove_at(&mut self, addr: ItemAddr) -> (Item<K, V>, ItemAddr) {
		match self.node_mut(addr.id).leaf_remove(addr.offset) {
			Ok(item) => { // removed from a leaf.
				let addr = self.rebalance(addr.id, addr);
				(item, addr)
			},
			Err(left_child_id) => { // removed from an internal node.
				let (separator, leaf_id) = self.remove_rightmost_leaf_of(left_child_id);
				let item = self.node_mut(addr.id).replace(addr.offset, separator);
				let addr = self.rebalance(leaf_id, addr);
				(item, addr)
			}
		}
	}

	// /// General-purpose update function.
	// ///
	// /// This can be used to insert, compare, replace or remove the value associated to the given
	// /// `key` in the tree.
	// /// The action to perform is specified by the `action` function.
	// /// This function is called once with:
	// ///  - `Some(value)` when `value` is aready associated to `key` in the tree or
	// ///  - `None` when the `key` is not associated to any value in the tree.
	// /// The `action` function must return a pair (`new_value`, `result`) where
	// /// `new_value` is the new value to be associated to `key`
	// /// (if it is `None` any previous binding is removed) and
	// /// `result` is the value returned by the entire `update` function call.
	// #[inline]
	// pub fn update<T, F>(&mut self, key: K, action: F) -> T where K: Ord, F: FnOnce(Option<V>) -> (Option<V>, T) {
	// 	match self.root {
	// 		Some(id) => {
	// 			let (result, balance) = self.update_in(key, action, id);
	//
	// 			match balance {
	// 				Balance::Underflow(true) => { // The root is empty.
	// 					self.root = self.node(id).child_id_opt(0);
	//
	// 					// update root's parent
	// 					if let Some(root_id) = self.root {
	// 						self.node_mut(root_id).set_parent(None)
	// 					}
	//
	// 					self.release_node(id);
	// 				},
	// 				_ => ()
	// 			};
	//
	// 			result
	// 		},
	// 		None => {
	// 			let (to_insert, result) = action(None);
	//
	// 			if let Some(value) = to_insert {
	// 				let new_root = Node::leaf(None, Item { key, value });
	// 				self.root = Some(self.allocate_node(new_root));
	// 				self.len += 1;
	// 			}
	//
	// 			result
	// 		}
	// 	}
	// }

	/// Write the tree in the DOT graph descrption language.
	///
	/// Requires the `dot` feature.
	#[cfg(feature = "dot")]
	#[inline]
	pub fn dot_write<W: std::io::Write>(&self, f: &mut W) -> std::io::Result<()> where K: std::fmt::Display, V: std::fmt::Display {
		write!(f, "digraph tree {{\n\tnode [shape=record];\n")?;
		match self.root {
			Some(id) => self.dot_write_node(f, id)?,
			None => ()
		}
		write!(f, "}}")
	}

	/// Write the given node in the DOT graph descrption language.
	///
	/// Requires the `dot` feature.
	#[cfg(feature = "dot")]
	#[inline]
	fn dot_write_node<W: std::io::Write>(&self, f: &mut W, id: usize) -> std::io::Result<()> where K: std::fmt::Display, V: std::fmt::Display {
		let name = format!("n{}", id);
		let node = self.node(id);

		write!(f, "\t{} [label=\"", name)?;
		if let Some(parent) = node.parent() {
			write!(f, "({})|", parent)?;
		}

		node.dot_write_label(f)?;
		write!(f, "({})\"];\n", id)?;

		for child_id in node.children() {
			self.dot_write_node(f, child_id)?;
			let child_name = format!("n{}", child_id);
			write!(f, "\t{} -> {}\n", name, child_name)?;
		}

		Ok(())
	}
}

impl<K, V> BTreeExt<K, V> for BTreeMap<K, V> {
	#[inline]
	fn set_len(&mut self, new_len: usize) {
		self.len = new_len
	}

	#[inline]
	fn set_root_id(&mut self, id: Option<usize>) {
		self.root = id
	}

	#[inline]
	fn root_id(&self) -> Option<usize> {
		self.root
	}

	#[inline]
	fn node(&self, id: usize) -> &Node<K, V> {
		&self.nodes[id]
	}

	#[inline]
	fn node_mut(&mut self, id: usize) -> &mut Node<K, V> {
		&mut self.nodes[id]
	}

	#[inline]
	fn get_in(&self, key: &K, mut id: usize) -> Option<&V> where K: Ord {
		loop {
			match self.nodes[id].get(key) {
				Ok(value_opt) => return value_opt,
				Err(child_id) => {
					id = child_id
				}
			}
		}
	}

	#[inline]
	fn get_mut_in<'a>(&'a mut self, key: &K, mut id: usize) -> Option<&'a mut V> where K: Ord {
		// The borrow checker is unable to predict that `*self`
		// is not borrowed more that once at a time.
		// That's why we need this little unsafe pointer gymnastic.

		let value_ptr = loop {
			match self.nodes[id].get_mut(key) {
				Ok(value_opt) => break value_opt.map(|value_ref| value_ref as *mut V),
				Err(child_id) => {
					id = child_id
				}
			}
		};

		unsafe {
			value_ptr.map(|ptr| &mut *ptr)
		}
	}

	/// Insert a key-value pair in an internal or leaf node.
	///
	/// It is assumed that there is still one free space in the node.
	#[inline]
	fn insert_into(&mut self, mut id: usize, mut key: K, mut value: V) -> (Option<V>, ItemAddr) where K: Ord {
		loop {
			match self.node_mut(id).insert_by_key(key, value) {
				Ok((offset, old_value)) => {
					let addr = self.rebalance(id, ItemAddr { id, offset });
					return (old_value, addr)
				},
				Err((k, v, _, child_id)) => {
					key = k;
					value = v;
					id = child_id
				}
			}
		}
	}

	/// Rebalance the given node.
	#[inline]
	fn rebalance(&mut self, mut id: usize, addr: ItemAddr) -> ItemAddr {
		let mut balance = self.node(id).balance();

		loop {
			match balance {
				Balance::Balanced => {
					break
				},
				Balance::Overflow => {
					let (left_node_len, median, right_node) = self.node_mut(id).split();
					let right_id = self.allocate_node(right_node);

					match self.node(id).parent() {
						Some(parent_id) => {
							let index = self.node(parent_id).child_index(id).unwrap();
							self.node_mut(id).insert(index, median, Some(right_id));
						},
						None => {
							let left_id = id;
							let new_root = Node::binary(None, left_id, median, right_id);
							let id = self.allocate_node(new_root);

							self.root = Some(id);
							self.nodes[left_id].set_parent(self.root);
							self.nodes[right_id].set_parent(self.root);
						}
					}
				},
				Balance::Underflow(is_empty) => {
					match self.node(id).parent() {
						Some(parent_id) => {
							let index = self.node(parent_id).child_index(id).unwrap();
							// An underflow append in the child node.
							// First we try to rebalance the tree by rotation.
							if !self.try_rotate_left(parent_id, index) && !self.try_rotate_right(parent_id, index) {
								// Rotation didn't work.
								// This means that all existing child sibling have enough few elements to be merged with this child.
								balance = self.merge(parent_id, index);
								// The `merge` function returns the current balance of the parent node,
								// since it may underflow after the merging operation.
								id = parent_id
							}
						},
						None => {
							// if root is empty.
							if is_empty {
								self.root = self.node(id).child_id_opt(0);

								// update root's parent
								if let Some(root_id) = self.root {
									self.node_mut(root_id).set_parent(None)
								}

								self.release_node(id);
							}
						}
					}
				}
			}
		}

		addr
	}

	/// Remove the item matching the given key from the given internal node `id`.
	#[inline]
	fn remove_from(&mut self, mut id: usize, key: &K) -> Option<(V, ItemAddr)> where K: Ord {
		loop {
			match self.nodes[id].offset_of(key) {
				Ok(offset) => {
					let (item, addr) = self.remove_at(ItemAddr { id, offset });
					return Some((item.value, addr))
				},
				Err(Some(child_id)) => {
					id = child_id;
				},
				Err(None) => return None
			}
		}
	}

	// fn update_in<T, F>(&mut self, key: K, action: F, id: usize) -> (T, Balance) where K: Ord, F: FnOnce(Option<V>) -> (Option<V>, T) {
	// 	match self.nodes[id].offset_of(&key) {
	// 		Ok(offset) => {
	// 			match self.nodes[id].remove(offset) {
	// 				Ok((mut item, balance)) => { // update in leaf.
	// 					let (to_insert, result) = action(Some(item.value));
	// 					match to_insert {
	// 						Some(value) => {
	// 							item.value = value;
	// 							self.nodes[id].insert(offset, item, None);
	// 							(result, Balance::Balanced)
	// 						},
	// 						None => {
	// 							self.len -= 1;
	// 							(result, balance)
	// 						}
	// 					}
	// 				},
	// 				Err((left_child_id, mut item, right_child_id)) => { // update in internal node.
	// 					let (to_insert, result) = action(Some(item.value));
	// 					match to_insert {
	// 						Some(value) => {
	// 							item.value = value;
	// 							self.nodes[id].insert(offset, item, Some(right_child_id));
	// 							(result, Balance::Balanced)
	// 						},
	// 						None => {
	// 							let left_child_index = offset;
	// 							let (separator, left_child_balance) = self.remove_rightmost_leaf_of(left_child_id);
	// 							self.nodes[id].insert(offset, separator, Some(right_child_id));
	// 							let balance = self.rebalance_child(id, left_child_index, left_child_balance);
	// 							self.len -= 1;
	// 							(result, balance)
	// 						}
	// 					}
	// 				}
	// 			}
	// 		},
	// 		Err(Some((child_index, child_id))) => { // update in child
	// 			// split the child if necessary.
	// 			let child = &mut self.nodes[child_id];
	// 			let child_id = match child.split() {
	// 				Ok((median, right_node)) => {
	// 					let insert_right = key > median.key;
	// 					let right_id = self.allocate_node(right_node);
	// 					match &mut self.nodes[id] {
	// 						Node::Internal(node) => {
	// 							node.insert(child_index, median, right_id)
	// 						},
	// 						_ => unreachable!()
	// 					}
	//
	// 					if insert_right {
	// 						right_id
	// 					} else {
	// 						child_id
	// 					}
	// 				},
	// 				_ => child_id
	// 			};
	//
	// 			let (result, child_balance) = self.update_in(key, action, child_id);
	// 			let balance = self.rebalance_child(id, child_index, child_balance);
	// 			(result, balance)
	// 		},
	// 		Err(None) => { // update nowhere. We are in a leaf.
	// 			let (to_insert, result) = action(None);
	//
	// 			if let Some(value) = to_insert {
	// 				match self.nodes[id].insert_by_key(key, value) {
	// 					Ok((_, None)) => (),
	// 					_ => unreachable!()
	// 				}
	// 			}
	//
	// 			self.len += 1;
	//
	// 			(result, Balance::Balanced)
	// 		}
	// 	}
	// }

	/// Take the right-most leaf value in the given node.
	///
	/// Note that this does not change the registred length of the tree.
	/// The returned item is expected to be reinserted in the tree.
	#[inline]
	fn remove_rightmost_leaf_of(&mut self, mut id: usize) -> (Item<K, V>, usize) {
		loop {
			match self.nodes[id].remove_rightmost_leaf() {
				Ok(result) => return (result, id),
				Err(child_id) => {
					id = child_id;
				}
			}
		}
	}

	/// Try to rotate left the node `id` to benefits the child number `deficient_child_index`.
	///
	/// Returns true if the rotation succeeded, of false if the target child has no right sibling,
	/// or if this sibling would underflow.
	#[inline]
	fn try_rotate_left(&mut self, id: usize, deficient_child_index: usize) -> bool {
		let pivot_index = deficient_child_index;
		let right_sibling_index = deficient_child_index + 1;
		let (right_sibling_id, deficient_child_id) = {
			let node = &self.nodes[id];

			if right_sibling_index >= node.child_count() {
				return false // no right sibling
			}

			(node.child_id(right_sibling_index), node.child_id(deficient_child_index))
		};

		match self.nodes[right_sibling_id].pop_left() {
			Ok((mut value, opt_child_id)) => {
				std::mem::swap(&mut value, &mut self.nodes[id].item_at_mut(pivot_index));
				self.nodes[deficient_child_id].push_right(value, opt_child_id);

				// update opt_child's parent
				if let Some(child_id) = opt_child_id {
					self.nodes[child_id].set_parent(Some(deficient_child_id))
				}

				true // rotation succeeded
			},
			Err(WouldUnderflow) => false // the right sibling would underflow.
		}
	}

	/// Try to rotate right the node `id` to benefits the child number `deficient_child_index`.
	///
	/// Returns true if the rotation succeeded, of false if the target child has no left sibling,
	/// or if this sibling would underflow.
	#[inline]
	fn try_rotate_right(&mut self, id: usize, deficient_child_index: usize) -> bool {
		if deficient_child_index > 0 {
			let left_sibling_index = deficient_child_index - 1;
			let pivot_index = left_sibling_index;
			let (left_sibling_id, deficient_child_id) = {
				let node = &self.nodes[id];
				(node.child_id(left_sibling_index), node.child_id(deficient_child_index))
			};
			match self.nodes[left_sibling_id].pop_right() {
				Ok((mut value, opt_child_id)) => {
					std::mem::swap(&mut value, &mut self.nodes[id].item_at_mut(pivot_index));
					self.nodes[deficient_child_id].push_left(value, opt_child_id);

					// update opt_child's parent
					if let Some(child_id) = opt_child_id {
						self.nodes[child_id].set_parent(Some(deficient_child_id))
					}

					true // rotation succeeded
				},
				Err(WouldUnderflow) => false // the left sibling would underflow.
			}
		} else {
			false // no left sibling.
		}
	}

	/// Merge the child `deficient_child_index` in node `id` with one of its direct sibling.
	#[inline]
	fn merge(&mut self, id: usize, deficient_child_index: usize) -> Balance {
		let (left_id, right_id, separator, balancing) = if deficient_child_index > 0 {
			// merge with left sibling
			self.nodes[id].merge(deficient_child_index-1, deficient_child_index)
		} else {
			// merge with right sibling
			self.nodes[id].merge(deficient_child_index, deficient_child_index+1)
		};

		// update children's parent.
		let right_node = self.release_node(right_id);
		for right_child_id in right_node.children() {
			self.nodes[right_child_id].set_parent(Some(left_id));
		}

		// actually merge.
		self.nodes[left_id].append(separator, right_node);

		balancing
	}

	/// Allocate a free node.
	#[inline]
	fn allocate_node(&mut self, node: Node<K, V>) -> usize {
		let mut children: StaticVec<usize, M> = StaticVec::new();
		let id = self.nodes.insert(node);

		for child_id in self.nodes[id].children() {
			children.push(child_id)
		}

		for child_id in children {
			self.nodes[child_id].set_parent(Some(id))
		}

		id
	}

	/// Release a node.
	#[inline]
	fn release_node(&mut self, id: usize) -> Node<K, V> {
		self.nodes.remove(id)
	}

	#[cfg(debug_assertions)]
	fn validate(&self) where K: Ord {
		match self.root {
			Some(id) => {
				self.validate_node(id, None, None, None);
			},
			None => ()
		}
	}

	/// Validate the given node and returns the depth of the node.
	#[cfg(debug_assertions)]
	fn validate_node(&self, id: usize, parent: Option<usize>, min: Option<&K>, max: Option<&K>) -> usize where K: Ord {
		let node = self.node(id);
		node.validate(parent, min, max);

		let mut depth = None;
		for (i, child_id) in node.children().enumerate() {
			let (min, max) = node.separators(i);

			let child_depth = self.validate_node(child_id, Some(id), min, max);
			match depth {
				None => depth = Some(child_depth),
				Some(depth) => {
					if depth != child_depth {
						panic!("tree not balanced")
					}
				}
			}
		}

		match depth {
			Some(depth) => depth + 1,
			None => 0
		}
	}
}

pub struct ItemMut<'a, K, V> {
	it: &'a mut IterMut<'a, K, V>,
	node: usize,
	offset: usize
}

pub struct IterMut<'a, K, V> {
	map: &'a mut BTreeMap<K, V>,
	current: Option<(usize, usize)>
}

impl<'a, K, V> IterMut<'a, K, V> {
	/// Get the current item visited by the iterator.
	fn current(&self) -> Option<&'a Item<K, V>> {
		panic!("TODO")
	}

	fn insert(&mut self, key: K, value: V) {
		panic!("TODO")
	}

	fn remove(&mut self) -> Option<Item<K, V>> {
		panic!("TODO")
	}
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
	type Item = &'a Item<K, V>;

	fn next(&mut self) -> Option<&'a Item<K, V>> {
		match self.current.take() {
			Some((node, offset)) => {
				panic!("TODO")
			},
			None => None
		}
	}
}
