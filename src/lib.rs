#![feature(min_const_generics)]
#![feature(move_ref_pattern)]
#![feature(is_sorted)]
#![feature(maybe_uninit_ref)]
use std::mem::MaybeUninit;
use slab::Slab;
use staticvec::StaticVec;

pub mod utils;
mod node;
mod item;

pub use node::*;
pub use item::*;

const M: usize = 4;

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

	fn item(&self, addr: ItemAddr) -> &Item<K, V>;

	fn item_mut(&mut self, addr: ItemAddr) -> &mut Item<K, V>;

	fn normalize(&self, addr: ItemAddr) -> Option<ItemAddr>;

	fn leaf_address(&self, addr: ItemAddr) -> ItemAddr;

	fn before(&self, addr: ItemAddr) -> Option<ItemAddr>;

	fn after(&self, addr: ItemAddr) -> Option<ItemAddr>;

	fn address_of(&self, key: &K) -> Result<ItemAddr, ItemAddr> where K: Ord;

	fn address_in(&self, id: usize, key: &K) -> Result<ItemAddr, ItemAddr> where K: Ord;

	fn insert_at(&mut self, addr: ItemAddr, item: Item<K, V>, opt_right_id: Option<usize>) -> ItemAddr;

	fn replace_at(&mut self, addr: ItemAddr, value: V) -> V;

	fn remove_at(&mut self, addr: ItemAddr) -> (Item<K, V>, ItemAddr);

	/// Rebalance a node, if necessary.
	fn rebalance(&mut self, node_id: usize, addr: ItemAddr) -> ItemAddr;

	// /// Update a value in the given node `node_id`.
	fn update_in<T, F>(&mut self, id: usize, key: K, action: F) -> T where K: Ord, F: FnOnce(Option<V>) -> (Option<V>, T);

	/// Take the right-most leaf value in the given node.
	fn remove_rightmost_leaf_of(&mut self, node_id: usize) -> (Item<K, V>, usize);

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
		match self.address_of(&key) {
			Ok(addr) => {
				Some(self.replace_at(addr, value))
			},
			Err(addr) => {
				self.insert_at(addr, Item::new(key, value), None);
				None
			}
		}
	}

	// Delete an item by key.
	#[inline]
	pub fn remove(&mut self, key: &K) -> Option<V> where K: Ord {
		match self.address_of(key) {
			Ok(addr) => {
				let (item, _) = self.remove_at(addr);
				Some(item.into_value())
			},
			Err(_) => None
		}
	}

	/// General-purpose update function.
	///
	/// This can be used to insert, compare, replace or remove the value associated to the given
	/// `key` in the tree.
	/// The action to perform is specified by the `action` function.
	/// This function is called once with:
	///  - `Some(value)` when `value` is aready associated to `key` in the tree or
	///  - `None` when the `key` is not associated to any value in the tree.
	/// The `action` function must return a pair (`new_value`, `result`) where
	/// `new_value` is the new value to be associated to `key`
	/// (if it is `None` any previous binding is removed) and
	/// `result` is the value returned by the entire `update` function call.
	#[inline]
	pub fn update<T, F>(&mut self, key: K, action: F) -> T where K: Ord, F: FnOnce(Option<V>) -> (Option<V>, T) {
		match self.root {
			Some(id) => self.update_in(id, key, action),
			None => {
				let (to_insert, result) = action(None);

				if let Some(value) = to_insert {
					let new_root = Node::leaf(None, Item::new(key, value));
					self.root = Some(self.allocate_node(new_root));
					self.len += 1;
				}

				result
			}
		}
	}

	/// Try to rotate left the node `id` to benefits the child number `deficient_child_index`.
	///
	/// Returns true if the rotation succeeded, of false if the target child has no right sibling,
	/// or if this sibling would underflow.
	#[inline]
	fn try_rotate_left(&mut self, id: usize, deficient_child_index: usize, addr: &mut ItemAddr) -> bool {
		let pivot_offset = deficient_child_index;
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
				std::mem::swap(&mut value, &mut self.nodes[id].item_mut(pivot_offset));
				let left_offset = self.nodes[deficient_child_id].push_right(value, opt_child_id);

				// update opt_child's parent
				if let Some(child_id) = opt_child_id {
					self.nodes[child_id].set_parent(Some(deficient_child_id))
				}

				// update address.
				if addr.id == right_sibling_id { // addressed item is in the right node.
					if addr.offset == 0 {
						// addressed item is moving to pivot.
						addr.id = id;
						addr.offset = pivot_offset;
					} else {
						// addressed item stays on right.
						addr.offset -= 1;
					}
				} else if addr.id == id { // addressed item is in the parent node.
					if addr.offset == pivot_offset {
						// addressed item is the pivot, moving to the left (deficient) node.
						addr.id = deficient_child_id;
						addr.offset = left_offset;
					}
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
	fn try_rotate_right(&mut self, id: usize, deficient_child_index: usize, addr: &mut ItemAddr) -> bool {
		if deficient_child_index > 0 {
			let left_sibling_index = deficient_child_index - 1;
			let pivot_offset = left_sibling_index;
			let (left_sibling_id, deficient_child_id) = {
				let node = &self.nodes[id];
				(node.child_id(left_sibling_index), node.child_id(deficient_child_index))
			};
			match self.nodes[left_sibling_id].pop_right() {
				Ok((left_offset, mut value, opt_child_id)) => {
					std::mem::swap(&mut value, &mut self.nodes[id].item_mut(pivot_offset));
					self.nodes[deficient_child_id].push_left(value, opt_child_id);

					// update opt_child's parent
					if let Some(child_id) = opt_child_id {
						self.nodes[child_id].set_parent(Some(deficient_child_id))
					}

					// update address.
					if addr.id == deficient_child_id { // addressed item is in the right (deficient) node.
						addr.offset += 1;
					} else if addr.id == left_sibling_id { // addressed item is in the left node.
						if addr.offset == left_offset {
							// addressed item is moving to pivot.
							addr.id = id;
							addr.offset = pivot_offset;
						}
					} else if addr.id == id { // addressed item is in the parent node.
						if addr.offset == pivot_offset {
							// addressed item is the pivot, moving to the left (deficient) node.
							addr.id = deficient_child_id;
							addr.offset = 0;
						}
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
	fn merge(&mut self, id: usize, deficient_child_index: usize, mut addr: ItemAddr) -> (Balance, ItemAddr) {
		let (offset, left_id, right_id, separator, balance) = if deficient_child_index > 0 {
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
		let left_offset = self.nodes[left_id].append(separator, right_node);

		// update addr.
		if addr.id == id {
			if addr.offset == offset {
				addr.id = left_id;
				addr.offset = left_offset;
			} else if addr.offset > offset {
				addr.offset -= 1;
			}
		} else if addr.id == right_id {
			addr.id = left_id;
			addr.offset += left_offset + 1;
		}

		(balance, addr)
	}

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

	fn item(&self, addr: ItemAddr) -> &Item<K, V> {
		self.node(addr.id).item(addr.offset)
	}

	fn item_mut(&mut self, addr: ItemAddr) -> &mut Item<K, V> {
		self.node_mut(addr.id).item_mut(addr.offset)
	}

	/// Normalize an item address so that an out-of-node-bounds address points to the next item.
	fn normalize(&self, mut addr: ItemAddr) -> Option<ItemAddr> {
		loop {
			let node = self.node(addr.id);
			if addr.offset >= node.item_count() {
				match node.parent() {
					Some(parent_id) => {
						addr.offset = self.node(parent_id).child_index(addr.id).unwrap();
						addr.id = parent_id;
					},
					None => return None
				}
			} else {
				return Some(addr)
			}
		}
	}

	#[inline]
	fn leaf_address(&self, mut addr: ItemAddr) -> ItemAddr {
		if !addr.is_nowhere() {
			loop {
				let node = self.node(addr.id);
				match node.child_id_opt(addr.offset) {
					Some(child_id) => {
						addr.id = child_id;
						addr.offset = self.node(child_id).item_count()
					},
					None => break
				}
			}
		}

		addr
	}

	/// Get the address of the item located before this address.
	#[inline]
	fn before(&self, mut addr: ItemAddr) -> Option<ItemAddr> {
		if addr.is_nowhere() {
			return None
		}

		loop {
			let node = self.node(addr.id);

			match node.child_id_opt(addr.offset) {
				Some(child_id) => {
					addr.offset = self.node(child_id).item_count();
					addr.id = child_id;
				},
				None => {
					loop {
						if addr.offset > 0 {
							addr.offset -= 1;
							return Some(addr)
						}

						match self.node(addr.id).parent() {
							Some(parent_id) => {
								addr.offset = self.node(parent_id).child_index(addr.id).unwrap();
								addr.id = parent_id;
							},
							None => return None
						}
					}
				}
			}
		}
	}

	/// Get the address of the item located after this address.
	#[inline]
	fn after(&self, mut addr: ItemAddr) -> Option<ItemAddr> {
		if addr.is_nowhere() {
			return None
		}

		addr.offset += 1;

		loop {
			let node = self.node(addr.id);

			match node.child_id_opt(addr.offset) {
				Some(child_id) => {
					addr.offset = 0;
					addr.id = child_id;
				},
				None => {
					loop {
						let node = self.node(addr.id);

						if addr.offset < node.item_count() {
							return Some(addr)
						}

						match node.parent() {
							Some(parent_id) => {
								addr.offset = self.node(parent_id).child_index(addr.id).unwrap();
								addr.id = parent_id;
							},
							None => return None
						}
					}
				}
			}
		}
	}

	/// Insert an item at the given address.
	/// Return the address of the inserted item in the tree
	/// (it may differ from the input address if the tree is rebalanced).
	///
	/// ## Correctness
	/// It is assumed that it is btree-correct to insert the given item at the given address.
	fn insert_at(&mut self, addr: ItemAddr, item: Item<K, V>, opt_right_id: Option<usize>) -> ItemAddr {
		if addr.is_nowhere() {
			if self.is_empty() {
				let new_root = Node::leaf(None, item);
				let id = self.allocate_node(new_root);
				self.root = Some(id);
				self.len += 1;
				ItemAddr { id, offset: 0 }
			} else {
				panic!("invalid item address")
			}
		} else {
			if self.is_empty() {
				panic!("invalid item address")
			} else {
				self.node_mut(addr.id).insert(addr.offset, item, opt_right_id);
				let new_addr = self.rebalance(addr.id, addr);
				self.len += 1;
				new_addr
			}
		}
	}

	fn replace_at(&mut self, addr: ItemAddr, value: V) -> V {
		self.node_mut(addr.id).item_mut(addr.offset).set_value(value)
	}

	fn address_of(&self, key: &K) -> Result<ItemAddr, ItemAddr> where K: Ord {
		match self.root {
			Some(id) => self.address_in(id, key),
			None => Err(ItemAddr::nowhere())
		}
	}

	fn address_in(&self, mut id: usize, key: &K) -> Result<ItemAddr, ItemAddr> where K: Ord {
		loop {
			match self.nodes[id].offset_of(key) {
				Ok(offset) => {
					return Ok(ItemAddr { id, offset })
				},
				Err((offset, None)) => {
					return Err(ItemAddr { id, offset })
				},
				Err((_, Some(child_id))) => {
					id = child_id;
				}
			}
		}
	}

	/// Remove the item at the given address.
	/// Return the address of the next item.
	/// All other addresses are to be considered invalid.
	///
	/// It is assumed that this item exists.
	#[inline]
	fn remove_at(&mut self, addr: ItemAddr) -> (Item<K, V>, ItemAddr) {
		self.len -= 1;
		match self.node_mut(addr.id).leaf_remove(addr.offset) {
			Ok(item) => { // removed from a leaf.
				let addr = self.rebalance(addr.id, addr);
				(item, addr)
			},
			Err(left_child_id) => { // removed from an internal node.
				let new_addr = self.after(addr).unwrap();
				let (separator, leaf_id) = self.remove_rightmost_leaf_of(left_child_id);
				let item = self.node_mut(addr.id).replace(addr.offset, separator);
				let addr = self.rebalance(leaf_id, new_addr);
				(item, addr)
			}
		}
	}

	fn update_in<T, F>(&mut self, mut id: usize, key: K, action: F) -> T where K: Ord, F: FnOnce(Option<V>) -> (Option<V>, T) {
		loop {
			match self.nodes[id].offset_of(&key) {
				Ok(offset) => unsafe {
					let mut value = MaybeUninit::uninit();
					let item = &mut self.nodes[id].item_mut(offset);
					std::mem::swap(&mut value, item.maybe_uninit_value_mut());
					let (opt_new_value, result) = action(Some(value.assume_init()));
					match opt_new_value {
						Some(new_value) => {
							let mut new_value = MaybeUninit::new(new_value);
							std::mem::swap(&mut new_value, item.maybe_uninit_value_mut());
						},
						None => {
							let (item, _) = self.remove_at(ItemAddr { id, offset });
							// item's value is NOT initialized here.
							// It must not be dropped.
							item.forget_value()
						}
					}

					return result
				},
				Err((offset, None)) => {
					let (opt_new_value, result) = action(None);
					if let Some(new_value) = opt_new_value {
						self.insert_at(ItemAddr { id, offset }, Item::new(key, new_value), None);
					}

					return result
				},
				Err((_, Some(child_id))) => {
					id = child_id;
				}
			}
		}
	}

	/// Rebalance the given node.
	#[inline]
	fn rebalance(&mut self, mut id: usize, mut addr: ItemAddr) -> ItemAddr {
		let mut balance = self.node(id).balance();

		loop {
			match balance {
				Balance::Balanced => {
					break
				},
				Balance::Overflow => {
					assert!(!self.node_mut(id).is_underflowing());
					let (median_offset, median, right_node) = self.node_mut(id).split();
					let right_id = self.allocate_node(right_node);

					match self.node(id).parent() {
						Some(parent_id) => {
							let parent = self.node_mut(parent_id);
							let offset = parent.child_index(id).unwrap();
							parent.insert(offset, median, Some(right_id));

							// new address.
							if addr.id == id {
								if addr.offset == median_offset {
									addr = ItemAddr { id: parent_id, offset }
								} else if addr.offset > median_offset {
									addr = ItemAddr {
										id: right_id,
										offset: addr.offset - median_offset - 1
									}
								}
							} else if addr.id == parent_id {
								if addr.offset >= offset {
									addr.offset += 1
								}
							}

							id = parent_id;
							balance = parent.balance()
						},
						None => {
							let left_id = id;
							let new_root = Node::binary(None, left_id, median, right_id);
							let root_id = self.allocate_node(new_root);

							self.root = Some(root_id);
							self.nodes[left_id].set_parent(self.root);
							self.nodes[right_id].set_parent(self.root);

							// new address.
							if addr.id == id {
								if addr.offset == median_offset {
									addr = ItemAddr { id: root_id, offset: 0 }
								} else if addr.offset > median_offset {
									addr = ItemAddr {
										id: right_id,
										offset: addr.offset - median_offset - 1
									}
								}
							}

							break
						}
					};
				},
				Balance::Underflow(is_empty) => {
					match self.node(id).parent() {
						Some(parent_id) => {
							let index = self.node(parent_id).child_index(id).unwrap();
							// An underflow append in the child node.
							// First we try to rebalance the tree by rotation.
							if self.try_rotate_left(parent_id, index, &mut addr) || self.try_rotate_right(parent_id, index, &mut addr) {
								break
							} else {
								// Rotation didn't work.
								// This means that all existing child sibling have enough few elements to be merged with this child.
								let (new_balance, new_addr) = self.merge(parent_id, index, addr);
								balance = new_balance;
								addr = new_addr;
								// The `merge` function returns the current balance of the parent node,
								// since it may underflow after the merging operation.
								id = parent_id
							}
						},
						None => {
							// if root is empty.
							if is_empty {
								self.root = self.node(id).child_id_opt(0);

								// update root's parent and addr.
								match self.root {
									Some(root_id) => {
										let root = self.node_mut(root_id);
										root.set_parent(None);

										if addr.id == id {
											addr.id = root_id;
											addr.offset = root.item_count()
										}
									},
									None => {
										addr = ItemAddr::nowhere()
									}
								}

								self.release_node(id);
							}

							break
						}
					}
				}
			}
		}

		addr
	}

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
