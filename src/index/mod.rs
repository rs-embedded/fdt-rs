//! Performant device tree utils which use an index built over a parsed FDT.
//!
//! Utilites in this module operate on a [`DevTreeIndex`]. This index provides an efficient to
//! traverse index of a parsed device tree.
//!
//! The index may be built without an allocator. In order to build the index, only a single `[u8]`
//! buffer is required.
//!
//! # Background
//!
//! FDT's are a compact binary format; node names, and other information all in a single
//! datastructure. Utilites which parse this device tree on the fly will run slower than those
//! which operate on an optimized index. Some operations such as finding a node's parent may
//! require `O(n^2)` time. To avoid this issue, we provide this module and related utilites.
//!
//! # Examples
//!
//! The same [`IterableDevTree`] trait used to implement [`DevTree`] methods is also implemented by
//! the [`DevTreeIndex`]. Therefore [all examples in the base module][crate::base] may also be used
//! through the [`DevTreeIndex`].
//!
//! This module's implementations will be significantly more performant than the base
//! immplementations.
//!
//!
//! ## Initialization
//!
//! ```
//! # use fdt_rs::doctest::FDT;
//! use fdt_rs::prelude::*;
//! use fdt_rs::base::*;
//! use fdt_rs::index::*;
//!
//! // Get access to a flattened device tree buffer.
//! let fdt: &[u8] = FDT;
//!
//! // Create the device tree parser
//! let devtree = unsafe { DevTree::new(fdt) }
//!     .expect("Buffer does not contain a device tree.");
//!
//! // Get the layout required to build an index
//! let layout = DevTreeIndex::get_layout(&devtree)
//!     .expect("Failed to parse DTB - it is invalid.");
//!
//! // Allocate memory for the index.
//! //
//! // This could be performed without a dynamic allocation
//! // if we allocated a static buffer or want to provide a
//! // raw buffer into uninitialized memory.
//! let mut vec = vec![0u8; layout.size() + layout.align()];
//! let raw_slice = vec.as_mut_slice();
//!
//! // Create the index of the device tree.
//! let index = DevTreeIndex::new(devtree, raw_slice).unwrap();
//!
//! ```
//!
#[cfg(doc)]
use crate::doctest::*;

#[doc(hidden)]
pub mod item;
#[doc(hidden)]
pub mod node;
#[doc(hidden)]
pub mod prop;
#[doc(hidden)]
pub mod tree;

pub mod iters;

#[doc(inline)]
pub use item::DevTreeIndexItem;
#[doc(inline)]
pub use node::DevTreeIndexNode;
#[doc(inline)]
pub use prop::DevTreeIndexProp;
#[doc(inline)]
pub use tree::DevTreeIndex;
