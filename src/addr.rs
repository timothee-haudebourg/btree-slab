use std::fmt;

/// Item location in a BTreeMap.
///
/// ## Validity
/// An item adress `addr` is *valid* in a given BTreeMap if it `addr.id` refers to an existing
/// node and if `addr.offset` is **less or equal** to the number of items in the node.
/// If `addr.offset` is equal to the number of items in the node then it doesn't actually refer
/// to an existing item in the node,
/// but it is a valid position to insert a new item with `BTreeExt::insert_at`.
/// We say that `addr` is *occupied* if it points to an actual item
/// (`addr.offset` less than the number of items in the node).
///
/// ## Safety
/// It is not safe to use an address `addr` in which `addr.id` is not the identifier of any node
/// currently used by the tree.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ItemAddr {
	/// Identifier of the node.
	pub id: usize,
	pub offset: usize
}

impl ItemAddr {
	#[inline]
	pub fn new(id: usize, offset: usize) -> ItemAddr {
		ItemAddr {
			id, offset
		}
	}

	#[inline]
	pub fn nowhere() -> ItemAddr {
		ItemAddr {
			id: std::usize::MAX,
			offset: 0
		}
	}

	#[inline]
	pub fn is_nowhere(&self) -> bool {
		self.id == std::usize::MAX
	}
}

impl fmt::Display for ItemAddr {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "@{}:{}", self.id, self.offset)
	}
}

impl fmt::Debug for ItemAddr {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "@{}:{}", self.id, self.offset)
	}
}
