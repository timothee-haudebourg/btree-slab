use std::{
	mem::MaybeUninit,
	borrow::Borrow
};
use staticvec::StaticVec;
use crate::{
	generic::{
		map::{
			M,
			BTreeMap
		},
		node::{
			Node,
			Balance,
			Item,
			Address,
			Offset
		}
	},
	Container,
	ContainerMut
};

/// Extended API.
///
/// This trait can be imported to access the internal functions of the B-Tree.
/// These functions are not intended to be directly called by the users, but can be used to
/// extends the data structure with new functionalities.
/// 
/// # Addressing
/// 
/// In this implementation of B-Trees, each node of a tree is addressed
/// by the [`Address`] type.
/// Each node is identified by a `usize`, and each item/entry in the node by an [`Offset`].
/// This extended API allows the caller to explore, access and modify the
/// internal structure of the tree using this addressing system.
/// 
/// Note that a valid address does not always refer to an actual item in the tree.
/// See the [`Address`] type documentation for more details.
pub trait BTreeExt<K, V> {
	/// Get the root node id.
	///
	/// Returns `None` if the tree is empty.
	fn root_id(&self) -> Option<usize>;

	/// Get the node associated to the given `id`.
	///
	/// Panics if `id` is out of bounds.
	fn node(&self, id: usize) -> &Node<K, V>;

	/// Get a reference to the value associated to the given `key` in the node `id`, if any.
	fn get_in<Q: ?Sized>(&self, key: &Q, id: usize) -> Option<&V> where K: Borrow<Q>, Q: Ord;

	/// Get a reference to the item located at the given address.
	fn item(&self, addr: Address) -> Option<&Item<K, V>>;

	/// Get the first item address, if any.
	/// 
	/// Returns the first occupied valid address, or `None` if the tree is empty.
	fn first_item_address(&self) -> Option<Address>;

	/// Get the first back address.
	/// 
	/// The returned address may not be occupied if the tree is empty.
	fn first_back_address(&self) -> Address;

	/// Get the last item address, if any.
	/// 
	/// Returns the last occupied valid address, or `None` if the tree is empty.
	fn last_item_address(&self) -> Option<Address>;

	/// Get the last valid address in the tree.
	fn last_valid_address(&self) -> Address;

	/// Normalizes the given item address so that an out-of-node-bounds address points to the next item.
	fn normalize(&self, addr: Address) -> Option<Address>;

	/// Returns the greatest valid leaf address that directly precedes the given address.
	/// 
	/// A "leaf address" is an address located in a leaf node.
	fn leaf_address(&self, addr: Address) -> Address;

	/// Get the previous item address.
	/// 
	/// Returns the previous valid occupied address.
	/// 
	/// The following diagram shows the order between addresses defined by this function.
	/// ```text
	///                                          ┌───────────┐ 
	///                            ╔═════════════╪══╗  ╔══╗  │ 
	///                            ║             │┌─v─┐║┌─v─┐│  
	///                ┌───────────╫─────────────││ 0 │║│ 1 ││──────────────────────┐
	///                │           ║             │└─v─┘║└─v─┘│                      │
	///                │           ║             └──╫──╫──╫──┘                      │
	///    start v     │           ║                ║  ║│ ╚══════════════════════╗  │  ^ end
	///          ║     │           ║             ╔══╝  ╚╪══════════╗             ║  │  ║ 
	///       ┌──╫──────────────┐  ║          ┌──╫──────────────┐  ║          ┌──╫─────╫──┐ 
	///       │  ║     ╔═════╗  │  ║          │  ║     ╔═════╗  │  ║          │  ║     ║  │
	///       │┌─v─┐ ┌─^─┐ ┌─v─┐│  ║          │┌─v─┐ ┌─^─┐ ┌─v─┐│  ║          │┌─v─┐ ┌─^─┐│
	///       ││ 0 │ │ 1 │ │ 2 ││  ║          ││ 0 │ │ 1 │ │ 2 ││  ║          ││ 0 │ │ 1 ││
	///       │└─v─┘ └─^─┘ └─v─┘│  ║          │└─v─┘ └─^─┘ └─v─┘│  ║          │└─v─┘ └─^─┘│
	///       │  ╚═════╝     ╚══╪══╝          │  ╚═════╝     ╚══╪══╝          │  ╚═════╝  │
	///       └─────────────────┘             └─────────────────┘             └───────────┘
	/// ```
	fn previous_item_address(&self, addr: Address) -> Option<Address>;

	/// Get the previous front address.
	/// 
	/// A "front address" is a valid address whose offset is less that the number of items in the node.
	/// If `addr.offset` is equal to `-1`, then it doesn't actually refer to an existing item in the node.
	/// 
	/// The following diagram shows the order between addresses defined by this function.
	/// ```text
	///                                                         ^ end
	///                                               ┌─────────║──┐
	///                            ╔═══════════════╗  │╔══╗     ║  │
	///                            ║             ┌─v─┐│║┌─v─┐ ┌─^─┐│
	///                      ┌─────╫─────────────│-1 ││║│ 0 │ │ 1 ││ ─────────────────────┐
	///                      │     ║             └─v─┘│║└─v─┘ └─^─┘│                      │
	///                      │     ║               ║  └╫──╫─────╫──┘                      │
	///                      │     ║               ║   ║  ║  │  ╚═════════════════════════╪═══════╗
	///    start v           │     ║               ║   ║  ╚══╪═══════════════════╗        │       ║
	///          ║           │     ║             ╔═╝   ╚═════╪═════╗             ║        │       ║
	///          ║  ┌──────────────╫──┐          ║  ┌──────────────╫──┐          ║  ┌───────────┐ ║
	///          ║  │  ╔═════╗     ║  │          ║  │  ╔═════╗     ║  │          ║  │  ╔═════╗  │ ║
	///        ┌─v─┐│┌─^─┐ ┌─v─┐ ┌─^─┐│        ┌─v─┐│┌─^─┐ ┌─v─┐ ┌─^─┐│        ┌─v─┐│┌─^─┐ ┌─v─┐│ ║
	///        │-1 │││ 0 │ │ 1 │ │ 2 ││        │-1 │││ 0 │ │ 1 │ │ 2 ││        │-1 │││ 0 │ │ 1 ││ ║
	///        └─v─┘│└─^─┘ └─v─┘ └─^─┘│        └─v─┘│└─^─┘ └─v─┘ └─^─┘│        └─v─┘│└─^─┘ └─v─┘│ ║
	///          ╚══╪══╝     ╚═════╝  │          ╚══╪══╝     ╚═════╝  │          ╚══╪══╝     ╚══╪═╝
	///             └─────────────────┘             └─────────────────┘             └───────────┘
	/// ```
	fn previous_front_address(&self, addr: Address) -> Option<Address>;

	/// Get the next item address.
	/// 
	/// Returns the next valid occupied address.
	/// 
	/// The following diagram shows the order between addresses defined by this function.
	/// ```text
	///                                          ┌───────────┐ 
	///                            ╔═════════════╪══╗  ╔══╗  │ 
	///                            ║             │┌─v─┐║┌─v─┐│  
	///                ┌───────────╫─────────────││ 0 │║│ 1 ││──────────────────────┐
	///                │           ║             │└─v─┘║└─v─┘│                      │
	///                │           ║             └──╫──╫──╫──┘                      │
	///    start v     │           ║                ║  ║│ ╚══════════════════════╗  │  ^ end
	///          ║     │           ║             ╔══╝  ╚╪══════════╗             ║  │  ║ 
	///       ┌──╫──────────────┐  ║          ┌──╫──────────────┐  ║          ┌──╫─────╫──┐ 
	///       │  ║     ╔═════╗  │  ║          │  ║     ╔═════╗  │  ║          │  ║     ║  │
	///       │┌─v─┐ ┌─^─┐ ┌─v─┐│  ║          │┌─v─┐ ┌─^─┐ ┌─v─┐│  ║          │┌─v─┐ ┌─^─┐│
	///       ││ 0 │ │ 1 │ │ 2 ││  ║          ││ 0 │ │ 1 │ │ 2 ││  ║          ││ 0 │ │ 1 ││
	///       │└─v─┘ └─^─┘ └─v─┘│  ║          │└─v─┘ └─^─┘ └─v─┘│  ║          │└─v─┘ └─^─┘│
	///       │  ╚═════╝     ╚══╪══╝          │  ╚═════╝     ╚══╪══╝          │  ╚═════╝  │
	///       └─────────────────┘             └─────────────────┘             └───────────┘
	/// ```
	fn next_item_address(&self, addr: Address) -> Option<Address>;

	/// Get the next back address.
	/// 
	/// A "back address" is a valid address whose offset is at least `0`.
	/// If `addr.offset` is equal to the number of items in the node then it doesn't actually refer
	/// to an existing item in the node,
	/// but it can be used to insert a new item with `BTreeExt::insert_at`.
	/// 
	/// The following diagram shows the order between addresses defined by this function.
	/// ```text
	///                                          ┌───────────┐  ^ end
	///                            ╔═════════════╪══╗  ╔══╗  │  ║
	///                            ║             │┌─v─┐║┌─v─┐│┌─^─┐
	///                ┌───────────╫─────────────││ 0 │║│ 1 │││ 2 │─────────────────┐
	///                │           ║             │└─v─┘║└─v─┘│└─^─┘                 │
	///                │           ║             └──╫──╫──╫──┘  ╚═══════════════════╪════════════╗
	///    start v     │           ║                ║  ║│ ╚══════════════════════╗  │            ║
	///          ║     │           ║             ╔══╝  ╚╪══════════╗             ║  │            ║
	///       ┌──╫──────────────┐  ║          ┌──╫──────────────┐  ║          ┌──╫────────┐      ║
	///       │  ║     ╔═════╗  │  ║          │  ║     ╔═════╗  │  ║          │  ║     ╔══╪══╗   ║
	///       │┌─v─┐ ┌─^─┐ ┌─v─┐│┌─^─┐        │┌─v─┐ ┌─^─┐ ┌─v─┐│┌─^─┐        │┌─v─┐ ┌─^─┐│┌─v─┐ ║
	///       ││ 0 │ │ 1 │ │ 2 │││ 3 │        ││ 0 │ │ 1 │ │ 2 │││ 3 │        ││ 0 │ │ 1 │││ 2 >═╝
	///       │└─v─┘ └─^─┘ └─v─┘│└─^─┘        │└─v─┘ └─^─┘ └─v─┘│└─^─┘        │└─v─┘ └─^─┘│└───┘
	///       │  ╚═════╝     ╚══╪══╝          │  ╚═════╝     ╚══╪══╝          │  ╚═════╝  │
	///       └─────────────────┘             └─────────────────┘             └───────────┘
	/// ```
	fn next_back_address(&self, addr: Address) -> Option<Address>;

	/// Get the next item address if any, or the next back address otherwise.
	fn next_item_or_back_address(&self, addr: Address) -> Option<Address>;

	/// Get the address of the given key.
	/// 
	/// Returns `Ok(addr)` if the key is used in the tree.
	/// If the key is not used in the tree then `Err(addr)` is returned,
	/// where `addr` can be used to insert the missing key.
	fn address_of<Q: ?Sized>(&self, key: &Q) -> Result<Address, Address> where K: Borrow<Q>, Q: Ord;

	/// Search for the address of the given key from the given node `id`.
	/// 
	/// Users should directly use [`address_of`].
	fn address_in<Q: ?Sized>(&self, id: usize, key: &Q) -> Result<Address, Address> where K: Borrow<Q>, Q: Ord;

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

/// Extended mutable API.
/// 
/// This trait can be imported to access and modify the internal functions of the B-Tree.
/// These functions are not intended to be directly called by the users, but can be used to
/// extends the data structure with new functionalities.
/// 
/// # Correctness
/// 
/// The user of this trait is responsible to preserve the invariants of the data-structure.
/// In particular, no item must be modified or inserted in a way that
/// break the order between keys.
pub trait BTreeExtMut<K, V> {
	/// Set the new known number of items in the tree.
	fn set_len(&mut self, len: usize);

	/// Set the root node identifier.
	fn set_root_id(&mut self, id: Option<usize>);

	/// Get the node associated to the given `id` mutabily.
	///
	/// Panics if `id` is out of bounds.
	fn node_mut(&mut self, id: usize) -> &mut Node<K, V>;

	/// Get a mutable reference to the value associated to the given `key` in the node `id`, if any.
	fn get_mut_in(&mut self, key: &K, id: usize) -> Option<&mut V> where K: Ord;

	/// Get a mutable reference to the item located at the given address.
	fn item_mut(&mut self, addr: Address) -> Option<&mut Item<K, V>>;

	/// Insert an item at the given address.
	/// 
	/// The address is first converted into a leaf address using [`BTreeExt::leaf_address`]
	/// and the item inserted using [`insert_exactly_at`].
	fn insert_at(&mut self, addr: Address, item: Item<K, V>) -> Address;

	/// Insert an item at the given address.
	/// 
	/// If the address refers to an internal node,
	/// `opt_right_id` defines the identifier of the child node inserted on the right of the inserted item.
	/// 
	/// Returns the address of the inserted item in the tree
	/// (it may differ from the input address if the tree is rebalanced).
	/// 
	/// # Correctness
	/// 
	/// It is assumed that it is btree-correct to insert the given item at the given address.
	/// 
	/// # Panic
	/// 
	/// This function panics if the address refers to an internal node and `opt_right_id` is `None`.
	fn insert_exactly_at(&mut self, addr: Address, item: Item<K, V>, opt_right_id: Option<usize>) -> Address;

	/// Replaces the key-value binding at the given address.
	fn replace_at(&mut self, addr: Address, key: K, value: V) -> (K, V);

	/// Replaces the value at the given address.
	fn replace_value_at(&mut self, addr: Address, value: V) -> V;

	/// Removes the item at the given address, if any.
	/// 
	/// If an item is removed then
	/// this function returns a pair where the first hand side is the removed item,
	/// and the right hand side is the updated address where the item can be reinserted at.
	fn remove_at(&mut self, addr: Address) -> Option<(Item<K, V>, Address)>;

	/// Rebalance a node, if necessary.
	fn rebalance(&mut self, node_id: usize, addr: Address) -> Address;

	/// Update a value in the given node `node_id`.
	fn update_in<T, F>(&mut self, id: usize, key: K, action: F) -> T where K: Ord, F: FnOnce(Option<V>) -> (Option<V>, T);

	/// Update a valud at the given address.
	fn update_at<T, F>(&mut self, addr: Address, action: F) -> T where K: Ord, F: FnOnce(V) -> (Option<V>, T);

	/// Take the right-most leaf value in the given node.
	///
	/// Note that this does not change the registred length of the tree.
	/// The returned item is expected to be reinserted in the tree.
	fn remove_rightmost_leaf_of(&mut self, node_id: usize) -> (Item<K, V>, usize);

	/// Allocate a free identifier for the given node.
	fn allocate_node(&mut self, node: Node<K, V>) -> usize;

	/// Release the given node identifier and return the node it used to identify.
	fn release_node(&mut self, id: usize) -> Node<K, V>;
}

impl<K, V, C: Container<Node<K, V>>> BTreeExt<K, V> for BTreeMap<K, V, C> {
	#[inline]
	fn root_id(&self) -> Option<usize> {
		self.root
	}

	#[inline]
	fn node(&self, id: usize) -> &Node<K, V> {
		self.nodes.get(id).unwrap()
	}

	#[inline]
	fn get_in<Q: ?Sized>(&self, key: &Q, mut id: usize) -> Option<&V> where K: Borrow<Q>, Q: Ord {
		loop {
			match self.node(id).get(key) {
				Ok(value_opt) => return value_opt,
				Err(child_id) => {
					id = child_id
				}
			}
		}
	}

	fn item(&self, addr: Address) -> Option<&Item<K, V>> {
		self.node(addr.id).item(addr.offset)
	}

	fn first_item_address(&self) -> Option<Address> {
		match self.root {
			Some(mut id) => loop {
				match self.node(id).child_id_opt(0) {
					Some(child_id) => {
						id = child_id
					},
					None => return Some(Address::new(id, 0.into()))
				}
			},
			None => None
		}
	}

	fn first_back_address(&self) -> Address {
		match self.root {
			Some(mut id) => loop {
				match self.node(id).child_id_opt(0) {
					Some(child_id) => {
						id = child_id
					},
					None => return Address::new(id, 0.into()) // TODO FIXME thechnically not the first
				}
			},
			None => Address::nowhere()
		}
	}

	fn last_item_address(&self) -> Option<Address> {
		match self.root {
			Some(mut id) => loop {
				let node = self.node(id);
				let index = node.item_count();
				match node.child_id_opt(index) {
					Some(child_id) => id = child_id,
					None => return Some(Address::new(id, (index - 1).into()))
				}
			},
			None => None
		}
	}

	fn last_valid_address(&self) -> Address {
		match self.root {
			Some(mut id) => loop {
				let node = self.node(id);
				let index = node.item_count();
				match node.child_id_opt(index) {
					Some(child_id) => id = child_id,
					None => return Address::new(id, index.into())
				}
			},
			None => Address::nowhere()
		}
	}

	fn normalize(&self, mut addr: Address) -> Option<Address> {
		if addr.is_nowhere() {
			None
		} else {
			loop {
				let node = self.node(addr.id);
				if addr.offset >= node.item_count() {
					match node.parent() {
						Some(parent_id) => {
							addr.offset = self.node(parent_id).child_index(addr.id).unwrap().into();
							addr.id = parent_id;
						},
						None => return None
					}
				} else {
					return Some(addr)
				}
			}
		}
	}

	#[inline]
	fn leaf_address(&self, mut addr: Address) -> Address {
		if !addr.is_nowhere() {
			loop {
				let node = self.node(addr.id);
				match node.child_id_opt(addr.offset.unwrap()) { // TODO unwrap may fail here!
					Some(child_id) => {
						addr.id = child_id;
						addr.offset = self.node(child_id).item_count().into()
					},
					None => break
				}
			}
		}

		addr
	}

	/// Get the address of the item located before this address.
	#[inline]
	fn previous_item_address(&self, mut addr: Address) -> Option<Address> {
		if addr.is_nowhere() {
			return None
		}

		loop {
			let node = self.node(addr.id);

			match node.child_id_opt(addr.offset.unwrap()) { // TODO unwrap may fail here.
				Some(child_id) => {
					addr.offset = self.node(child_id).item_count().into();
					addr.id = child_id;
				},
				None => {
					loop {
						if addr.offset > 0 {
							addr.offset.decr();
							return Some(addr)
						}

						match self.node(addr.id).parent() {
							Some(parent_id) => {
								addr.offset = self.node(parent_id).child_index(addr.id).unwrap().into();
								addr.id = parent_id;
							},
							None => return None
						}
					}
				}
			}
		}
	}

	#[inline]
	fn previous_front_address(&self, mut addr: Address) -> Option<Address> {
		if addr.is_nowhere() {
			return None
		}

		loop {
			let node = self.node(addr.id);
			match addr.offset.value() {
				Some(offset) => {
					let index = if offset < node.item_count() {
						offset
					} else {
						node.item_count()
					};

					match node.child_id_opt(index) {
						Some(child_id) => {
							addr.offset = (self.node(child_id).item_count()).into();
							addr.id = child_id;
						},
						None => {
							addr.offset.decr();
							break
						}
					}
				},
				None => {
					match node.parent() {
						Some(parent_id) => {
							addr.offset = self.node(parent_id).child_index(addr.id).unwrap().into();
							addr.offset.decr();
							addr.id = parent_id;
							break
						},
						None => return None
					}
				}
			}
		}

		Some(addr)
	}

	/// Get the address of the item located after this address if any.
	#[inline]
	fn next_item_address(&self, mut addr: Address) -> Option<Address> {
		if addr.is_nowhere() {
			return None
		}

		let item_count = self.node(addr.id).item_count();
		if addr.offset < item_count {
			addr.offset.incr();
		} else if addr.offset > item_count {
			return None
		}

		// let original_addr_shifted = addr;

		loop {
			let node = self.node(addr.id);

			match node.child_id_opt(addr.offset.unwrap()) { // unwrap may fail here.
				Some(child_id) => {
					addr.offset = 0.into();
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
								addr.offset = self.node(parent_id).child_index(addr.id).unwrap().into();
								addr.id = parent_id;
							},
							None => {
								// return Some(original_addr_shifted)
								return None
							}
						}
					}
				}
			}
		}
	}

	#[inline]
	fn next_back_address(&self, mut addr: Address) -> Option<Address> {
		if addr.is_nowhere() {
			return None
		}

		loop {
			let node = self.node(addr.id);
			let index = match addr.offset.value() {
				Some(offset) => offset + 1,
				None => 0
			};

			if index <= node.item_count() {
				match node.child_id_opt(index) {
					Some(child_id) => {
						addr.offset = Offset::before();
						addr.id = child_id;
					},
					None => {
						addr.offset = index.into();
						break
					}
				}
			} else {
				match node.parent() {
					Some(parent_id) => {
						addr.offset = self.node(parent_id).child_index(addr.id).unwrap().into();
						addr.id = parent_id;
						break
					},
					None => {
						return None
					}
				}
			}
		}

		Some(addr)
	}

	#[inline]
	fn next_item_or_back_address(&self, mut addr: Address) -> Option<Address> {
		if addr.is_nowhere() {
			return None
		}

		let item_count = self.node(addr.id).item_count();
		if addr.offset < item_count {
			addr.offset.incr();
		} else if addr.offset > item_count {
			return None
		}

		let original_addr_shifted = addr;

		loop {
			let node = self.node(addr.id);

			match node.child_id_opt(addr.offset.unwrap()) { // TODO unwrap may fail here.
				Some(child_id) => {
					addr.offset = 0.into();
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
								addr.offset = self.node(parent_id).child_index(addr.id).unwrap().into();
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

	fn address_of<Q: ?Sized>(&self, key: &Q) -> Result<Address, Address> where K: Borrow<Q>, Q: Ord {
		match self.root {
			Some(id) => self.address_in(id, key),
			None => Err(Address::nowhere())
		}
	}

	fn address_in<Q: ?Sized>(&self, mut id: usize, key: &Q) -> Result<Address, Address> where K: Borrow<Q>, Q: Ord {
		loop {
			match self.node(id).offset_of(key) {
				Ok(offset) => {
					return Ok(Address { id, offset })
				},
				Err((offset, None)) => {
					return Err(Address::new(id, offset.into()))
				},
				Err((_, Some(child_id))) => {
					id = child_id;
				}
			}
		}
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

impl<K, V, C: ContainerMut<Node<K, V>>> BTreeExtMut<K, V> for BTreeMap<K, V, C> {
	#[inline]
	fn set_len(&mut self, new_len: usize) {
		self.len = new_len
	}

	#[inline]
	fn set_root_id(&mut self, id: Option<usize>) {
		self.root = id
	}

	#[inline]
	fn node_mut(&mut self, id: usize) -> &mut Node<K, V> {
		self.nodes.get_mut(id).unwrap()
	}

	#[inline]
	fn get_mut_in<'a>(&'a mut self, key: &K, mut id: usize) -> Option<&'a mut V> where K: Ord {
		// The borrow checker is unable to predict that `*self`
		// is not borrowed more that once at a time.
		// That's why we need this little unsafe pointer gymnastic.

		let value_ptr = loop {
			match self.node_mut(id).get_mut(key) {
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

	fn item_mut(&mut self, addr: Address) -> Option<&mut Item<K, V>> {
		self.node_mut(addr.id).item_mut(addr.offset)
	}

	fn insert_at(&mut self, addr: Address, item: Item<K, V>) -> Address {
		self.insert_exactly_at(self.leaf_address(addr), item, None)
	}

	fn insert_exactly_at(&mut self, addr: Address, item: Item<K, V>, opt_right_id: Option<usize>) -> Address {
		if addr.is_nowhere() {
			if self.is_empty() {
				let new_root = Node::leaf(None, item);
				let id = self.allocate_node(new_root);
				self.root = Some(id);
				self.len += 1;
				Address { id, offset: 0.into() }
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

	fn replace_at(&mut self, addr: Address, key: K, value: V) -> (K, V) {
		self.node_mut(addr.id).item_mut(addr.offset).unwrap().set(key, value)
	}

	fn replace_value_at(&mut self, addr: Address, value: V) -> V {
		self.node_mut(addr.id).item_mut(addr.offset).unwrap().set_value(value)
	}

	#[inline]
	fn remove_at(&mut self, addr: Address) -> Option<(Item<K, V>, Address)> {
		self.len -= 1;
		match self.node_mut(addr.id).leaf_remove(addr.offset) {
			Some(Ok(item)) => { // removed from a leaf.
				let addr = self.rebalance(addr.id, addr);
				Some((item, addr))
			},
			Some(Err(left_child_id)) => { // removed from an internal node.
				let new_addr = self.next_item_or_back_address(addr).unwrap();
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
			match self.node(id).offset_of(&key) {
				Ok(offset) => unsafe {
					let mut value = MaybeUninit::uninit();
					let item = self.node_mut(id).item_mut(offset).unwrap();
					std::mem::swap(&mut value, item.maybe_uninit_value_mut());
					let (opt_new_value, result) = action(Some(value.assume_init()));
					match opt_new_value {
						Some(new_value) => {
							let mut new_value = MaybeUninit::new(new_value);
							std::mem::swap(&mut new_value, item.maybe_uninit_value_mut());
						},
						None => {
							let (item, _) = self.remove_at(Address::new(id, offset)).unwrap();
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
						let leaf_addr = Address::new(id, offset.into());
						self.insert_exactly_at(leaf_addr, Item::new(key, new_value), None);
					}

					return result
				},
				Err((_, Some(child_id))) => {
					id = child_id;
				}
			}
		}
	}

	fn update_at<T, F>(&mut self, addr: Address, action: F) -> T where K: Ord, F: FnOnce(V) -> (Option<V>, T) {
		unsafe {
			let mut value = MaybeUninit::uninit();
			let item = self.node_mut(addr.id).item_mut(addr.offset).unwrap();
			std::mem::swap(&mut value, item.maybe_uninit_value_mut());
			let (opt_new_value, result) = action(value.assume_init());
			match opt_new_value {
				Some(new_value) => {
					let mut new_value = MaybeUninit::new(new_value);
					std::mem::swap(&mut new_value, item.maybe_uninit_value_mut());
				},
				None => {
					let (item, _) = self.remove_at(addr).unwrap();
					// item's value is NOT initialized here.
					// It must not be dropped.
					item.forget_value()
				}
			}

			return result
		}
	}

	#[inline]
	fn rebalance(&mut self, mut id: usize, mut addr: Address) -> Address {
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
							let offset = parent.child_index(id).unwrap().into();
							parent.insert(offset, median, Some(right_id));

							// new address.
							if addr.id == id {
								if addr.offset == median_offset {
									addr = Address { id: parent_id, offset }
								} else if addr.offset > median_offset {
									addr = Address {
										id: right_id,
										offset: (addr.offset.unwrap() - median_offset - 1).into()
									}
								}
							} else if addr.id == parent_id {
								if addr.offset >= offset {
									addr.offset.incr()
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
							self.node_mut(left_id).set_parent(Some(root_id));
							self.node_mut(right_id).set_parent(Some(root_id));

							// new address.
							if addr.id == id {
								if addr.offset == median_offset {
									addr = Address { id: root_id, offset: 0.into() }
								} else if addr.offset > median_offset {
									addr = Address {
										id: right_id,
										offset: (addr.offset.unwrap() - median_offset - 1).into()
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
											addr.offset = root.item_count().into()
										}
									},
									None => {
										addr = Address::nowhere()
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

	#[inline]
	fn remove_rightmost_leaf_of(&mut self, mut id: usize) -> (Item<K, V>, usize) {
		loop {
			match self.node_mut(id).remove_rightmost_leaf() {
				Ok(result) => return (result, id),
				Err(child_id) => {
					id = child_id;
				}
			}
		}
	}

	#[inline]
	fn allocate_node(&mut self, node: Node<K, V>) -> usize {
		let mut children: StaticVec<usize, M> = StaticVec::new();
		let id = self.nodes.insert(node);

		for child_id in self.node(id).children() {
			children.push(child_id)
		}

		for child_id in children {
			self.node_mut(child_id).set_parent(Some(id))
		}

		id
	}

	#[inline]
	fn release_node(&mut self, id: usize) -> Node<K, V> {
		self.nodes.remove(id)
	}
}
