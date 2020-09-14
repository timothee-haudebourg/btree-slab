#![feature(min_const_generics)]
#![feature(move_ref_pattern)]

mod utils;
mod node;
mod item;

use node::*;
use item::*;

/// B-tree of Knuth order `M` storing keys of type `K` associated to values of type `V`.
///
/// The Knuth order must be at least 4.
pub struct BTree<K, V, const M: usize> {
	/// Allocated and free nodes.
	nodes: Vec<Node<K, V, M>>,

	/// Root node id.
	root: Option<usize>,

	/// First free node.
	first_free_node: Option<usize>
}

impl<K, V, const M: usize> BTree<K, V, M> {
	/// Create a new empty B-tree.
	pub fn new(&self) -> BTree<K, V, M> {
		assert!(M >= 4);
		BTree {
			nodes: Vec::new(),
			root: None,
			first_free_node: None
		}
	}

	/// Insert a key-value pair in an internal or leaf node.
	///
	/// It is assumed that there is still one free space in the node.
	#[inline]
	fn internal_insert(&mut self, mut key: K, mut value: V, mut id: usize) -> Option<V> where K: Ord {
		loop {
			// Try to insert the value in the current node.
			// For internal nodes, this works only if the key is already there.
			match self.nodes[id].insert(key, value) {
				Ok(old_value) => return old_value,
				Err((k, v, child_pos, child_id)) => {
					// Direct insertion failed.
					// We need to insert the element in a subtree.
					let child = &mut self.nodes[child_id];
					match child.split() {
						Ok((median, right_node)) => {
							let right_id = self.allocate_node(right_node);
							match &mut self.nodes[id] {
								Node::Internal(node) => {
									node.insert_node(child_pos, median, right_id)
								},
								_ => unreachable!()
							}
						},
						_ => ()
					}

					key = k;
					value = v;
					id = child_id;
				}
			}
		}
	}

	/// Rebalance a child node, if necessary.
	///
	/// Return the balance of the node after rebalancing the child.
	#[inline]
	fn rebalance_child(&mut self, id: usize, child_index: usize, child_balance: Balance) -> Balance {
		match child_balance {
			Balance::Balanced => Balance::Balanced,
			Balance::Underflow(_) => {
				// An underflow append in the child node.
				// First we try to rebalance the tree by rotation.
				if self.try_rotate_left(id, child_index) || self.try_rotate_right(id, child_index) {
					// Rotation worked.
					Balance::Balanced
				} else {
					// Rotation didn't work.
					// This means that all existing child sibling have enough few elements to be merged with this child.
					self.merge(id, child_index)
					// The `merge` function returns the current balance of the node `id`,
					// since it may underflow after the merging operation.
				}
			}
		}
	}

	/// Remove the item matching the given key from the given internal node `id`.
	fn remove_from(&mut self, key: &K, id: usize) -> Option<(V, Balance)> where K: Ord {
		match self.nodes[id].offset_of(key) {
			Ok(offset) => {
				match self.nodes[id].take(offset) {
					Ok((item, balance)) => { // removed from a leaf.
						Some((item.value, balance))
					},
					Err(left_child_id) => { // removed from an internal node.
						let left_child_index = offset;
						let (separator, left_child_balance) = self.remove_rightmost_leaf_of(left_child_id);
						let item = self.nodes[id].replace(offset, separator);
						let balance = self.rebalance_child(id, left_child_index, left_child_balance);
						Some((item.value, balance))
					}
				}
			},
			Err(Some((child_index, child_id))) => {
				match self.remove_from(key, child_id) {
					Some((value, child_balance)) => {
						let balance = self.rebalance_child(id, child_index, child_balance);
						Some((value, balance))
					},
					None => None
				}
			},
			Err(None) => None
		}
	}

	/// Take the right-most leaf value in the given node.
	#[inline]
	fn remove_rightmost_leaf_of(&mut self, id: usize) -> (Item<K, V>, Balance) {
		match self.nodes[id].take_rightmost_leaf() {
			Ok(result) => result,
			Err((rightmost_child_index, child_id)) => {
				let (item, child_balance) = self.remove_rightmost_leaf_of(child_id);
				let balance = self.rebalance_child(id, rightmost_child_index, child_balance);
				(item, balance)
			}
		}
	}

	/// Try to rotate left the node `id` to benefits the child number `deficient_child_index`.
	///
	/// Returns true if the rotation succeeded, of false if the target child has no right sibling,
	/// or if this sibling would underflow.
	#[inline]
	fn try_rotate_left(&mut self, id: usize, deficient_child_index: usize) -> bool {
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
				std::mem::swap(&mut value, &mut self.nodes[id].item_at_mut(right_sibling_index));
				self.nodes[deficient_child_id].push_right(value, opt_child_id);
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
			let (left_sibling_id, deficient_child_id) = {
				let node = &self.nodes[id];
				(node.child_id(left_sibling_index), node.child_id(deficient_child_index))
			};
			match self.nodes[left_sibling_id].pop_right() {
				Ok((mut value, opt_child_id)) => {
					std::mem::swap(&mut value, &mut self.nodes[id].item_at_mut(left_sibling_index));
					self.nodes[deficient_child_id].push_left(value, opt_child_id);
					true // rotation succeeded
				},
				Err(WouldUnderflow) => false // the left sibling would underflow.
			}
		} else {
			false // no left sibling.
		}
	}

	#[inline]
	fn merge(&mut self, id: usize, deficient_child_index: usize) -> Balance {
		let (left_id, right_id, separator, balancing) = if deficient_child_index > 0 {
			// merge with left sibling
			self.nodes[id].merge(deficient_child_index-1, deficient_child_index)
		} else {
			// merge with right sibling
			self.nodes[id].merge(deficient_child_index, deficient_child_index+1)
		};

		let right_node = self.release_node(right_id);
		self.nodes[left_id].append(separator, right_node);

		balancing
	}

	/// Insert a key-value pair in the tree.
	#[inline]
	pub fn insert(&mut self, key: K, value: V) -> Option<V> where K: Ord {
		match self.root {
			Some(id) => {
				let root = &mut self.nodes[id];
				match root.split() {
					Ok((median, right_node)) => {
						let new_root = Node::binary(id, median, self.allocate_node(right_node));
						self.root = Some(self.allocate_node(new_root))
					},
					_ => ()
				}

				self.internal_insert(key, value, id)
			},
			None => {
				let new_root = Node::leaf(Item { key, value });
				self.root = Some(self.allocate_node(new_root));
				None
			}
		}
	}

	// Delete an item by key.
	#[inline]
	pub fn remove(&mut self, key: &K) -> Option<V> where K: Ord {
		match self.root {
			Some(id) => match self.remove_from(key, id) {
				Some((value, balance)) => {
					match balance {
						Balance::Underflow(true) => { // The root node ended-up empty.
							self.release_node(id);
							self.root = None
						},
						_ => ()
					};

					Some(value)
				},
				None => None
			},
			None => None
		}
	}

	/// Allocate a free node.
	#[inline]
	fn allocate_node(&mut self, node: Node<K, V, M>) -> usize {
		// get the next free id.
		let allocated_id = match self.first_free_node {
			Some(id) => {
				match self.nodes[id] {
					Node::Free(_, next_id) => {
						self.first_free_node = next_id; // update next free node id.
						self.nodes[id] = node;
						id
					},
					_ => unreachable!()
				}
			},
			None => {
				let id = self.nodes.len();
				self.nodes.push(node);
				id
			}
		};

		// update the next free node link.
		match self.first_free_node {
			Some(id) => {
				match &mut self.nodes[id] {
					Node::Free(prev_id, _) => {
						*prev_id = None
					},
					_ => unreachable!()
				}
			},
			None => ()
		}

		allocated_id
	}

	/// Release a node.
	#[inline]
	fn release_node(&mut self, id: usize) -> Node<K, V, M> {
		let mut node = Node::Free(None, self.first_free_node);
		std::mem::swap(&mut node, &mut self.nodes[id]);

		match self.first_free_node {
			Some(id) => {
				match &mut self.nodes[id] {
					Node::Free(prev_id, _) => {
						*prev_id = Some(id)
					},
					_ => unreachable!()
				}
			},
			None => ()
		}

		self.first_free_node = Some(id);

		node
	}

	/// Shrink the capacity of the internal vector as much as possible.
	///
	/// Note that because of the internal fragmentation of the internal node vector,
	/// the capacity may still be larger than the number of nodes in the tree.
	#[inline]
	pub fn shrink_to_fit(&mut self) {
		loop {
			match self.nodes.last() {
				Some(last) => match last.as_free_node() {
					Ok((prev_id, next_id)) => {
						match prev_id {
							Some(prev_id) => match &mut self.nodes[prev_id] {
								Node::Free(_, id) => {
									*id = next_id
								},
								_ => unreachable!()
							},
							None => self.first_free_node = next_id
						}

						if let Some(next_id) = next_id {
							match &mut self.nodes[next_id] {
								Node::Free(id, _) => {
									*id = prev_id
								},
								_ => unreachable!()
							}
						}
					},
					Err(_) => break
				},
				None => break
			}
		}

		self.nodes.shrink_to_fit()
	}
}
