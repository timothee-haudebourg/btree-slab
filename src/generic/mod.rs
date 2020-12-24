//! Generic B-Tree types.
//!
//! Types defined in this modules are independant of the actual storage type.
pub mod node;
pub use node::Node;

pub mod map;
pub use map::BTreeMap;