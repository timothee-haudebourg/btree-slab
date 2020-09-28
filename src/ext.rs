use std::mem::MaybeUninit;
use staticvec::StaticVec;
use crate::{
	M,
	BTreeMap,
	Node,
	Balance,
	Item,
	ItemAddr
};

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

	fn item(&self, addr: ItemAddr) -> Option<&Item<K, V>>;

	fn item_mut(&mut self, addr: ItemAddr) -> Option<&mut Item<K, V>>;

	fn first_address(&self) -> ItemAddr;

	fn last_address(&self) -> ItemAddr;

	fn normalize(&self, addr: ItemAddr) -> Option<ItemAddr>;

	fn leaf_address(&self, addr: ItemAddr) -> ItemAddr;

	fn previous_address(&self, addr: ItemAddr) -> Option<ItemAddr>;

	fn next_address(&self, addr: ItemAddr) -> Option<ItemAddr>;

	fn address_of(&self, key: &K) -> Result<ItemAddr, ItemAddr> where K: Ord;

	fn address_in(&self, id: usize, key: &K) -> Result<ItemAddr, ItemAddr> where K: Ord;

	fn insert_at(&mut self, addr: ItemAddr, item: Item<K, V>, opt_right_id: Option<usize>) -> ItemAddr;

	fn replace_at(&mut self, addr: ItemAddr, value: V) -> V;

	fn remove_at(&mut self, addr: ItemAddr) -> Option<(Item<K, V>, ItemAddr)>;

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

	fn item(&self, addr: ItemAddr) -> Option<&Item<K, V>> {
		self.node(addr.id).item(addr.offset)
	}

	fn item_mut(&mut self, addr: ItemAddr) -> Option<&mut Item<K, V>> {
		self.node_mut(addr.id).item_mut(addr.offset)
	}

	fn first_address(&self) -> ItemAddr {
		match self.root {
			Some(mut id) => loop {
				match self.node(id).child_id_opt(0) {
					Some(child_id) => {
						id = child_id
					},
					None => return ItemAddr::new(id, 0)
				}
			},
			None => ItemAddr::nowhere()
		}
	}

	fn last_address(&self) -> ItemAddr {
		match self.root {
			Some(mut id) => loop {
				let node = self.node(id);
				let index = node.item_count();
				match node.child_id_opt(index) {
					Some(child_id) => id = child_id,
					None => return ItemAddr::new(id, index - 1)
				}
			},
			None => ItemAddr::nowhere()
		}
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
	fn previous_address(&self, mut addr: ItemAddr) -> Option<ItemAddr> {
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
	fn next_address(&self, mut addr: ItemAddr) -> Option<ItemAddr> {
		if addr.is_nowhere() {
			return None
		}

		let item_count = self.node(addr.id).item_count();
		if addr.offset < item_count {
			addr.offset += 1;
		} else if addr.offset > item_count {
			return None
		}

		let original_addr_shifted = addr;

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
							None => {
								return Some(original_addr_shifted)
							}
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
		self.node_mut(addr.id).item_mut(addr.offset).unwrap().set_value(value)
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
	fn remove_at(&mut self, addr: ItemAddr) -> Option<(Item<K, V>, ItemAddr)> {
		self.len -= 1;
		match self.node_mut(addr.id).leaf_remove(addr.offset) {
			Some(Ok(item)) => { // removed from a leaf.
				let addr = self.rebalance(addr.id, addr);
				Some((item, addr))
			},
			Some(Err(left_child_id)) => { // removed from an internal node.
				let new_addr = self.next_address(addr).unwrap();
				let (separator, leaf_id) = self.remove_rightmost_leaf_of(left_child_id);
				let item = self.node_mut(addr.id).replace(addr.offset, separator);
				let addr = self.rebalance(leaf_id, new_addr);
				Some((item, addr))
			},
			None => None
		}
	}

	fn update_in<T, F>(&mut self, mut id: usize, key: K, action: F) -> T where K: Ord, F: FnOnce(Option<V>) -> (Option<V>, T) {
		loop {
			match self.nodes[id].offset_of(&key) {
				Ok(offset) => unsafe {
					let mut value = MaybeUninit::uninit();
					let item = self.nodes[id].item_mut(offset).unwrap();
					std::mem::swap(&mut value, item.maybe_uninit_value_mut());
					let (opt_new_value, result) = action(Some(value.assume_init()));
					match opt_new_value {
						Some(new_value) => {
							let mut new_value = MaybeUninit::new(new_value);
							std::mem::swap(&mut new_value, item.maybe_uninit_value_mut());
						},
						None => {
							let (item, _) = self.remove_at(ItemAddr { id, offset }).unwrap();
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
