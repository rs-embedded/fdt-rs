//! Basic device tree parsing utils that operate directly on the FDT.
//!
//! # Overview
//!
//! This module provides basic utilities which can operate on a FDT through in-order parsing.
//! These utilites will simply parse the device tree on the fly.
//!
//! See the [`crate::index`] module for more advanced and performant utilites.
//!
//! # Examples
//!
//! ## Initialization
//!
//! ```
//! # use fdt_rs::doctest::FDT;
//! use fdt_rs::prelude::*;
//! use fdt_rs::base::*;
//!
//! // Get access to a flattened device tree buffer.
//! let fdt: &[u8] = FDT;
//!
//! // Create the device tree parser
//! let devtree = unsafe { DevTree::new(fdt) }
//!     .expect("Buffer does not contain a device tree.");
//! ```
//!
//! ## Compatible Search
//!
//! Find all [`DevTreeNode`] objects which have their `compatible` property defined as
//! `"ns16550a"`:
//! ```
//! # use fdt_rs::doctest::*;
//! # let (index, _) = doctest_index();
//! // Get the compatible node iterator
//! let mut iter = index.compatible_nodes("ns16550a");
//!
//! // Get a signle node from that iterator
//! let node = iter.next().expect("No node found!");
//!
//! // Iterate through all remaining nodes
//! for node in iter {
//!     println!{"Found node: {}", node.name().unwrap()};
//! }
//! ```
//!
//! ## Custom Search
//!
//! Find all [`DevTreeNode`] objects which have their `compatible` property defined as
//! `"ns16550a"`:
//! ```
//! # use fdt_rs::doctest::*;
//! # let (index, _) = doctest_index();
//! // Get the compatible node iterator
//! let mut iter = index.compatible_nodes("ns16550a");
//!
//! // Get a signle node from that iterator
//! let node = iter.next().expect("No node found!");
//!
//! // Iterate through all remaining nodes
//! for node in iter {
//!     println!{"Found node: {}", node.name().unwrap()};
//! }
//! ```

#[doc(hidden)]
pub mod item;
#[doc(hidden)]
pub mod node;
#[doc(hidden)]
pub mod prop;
#[doc(hidden)]
pub mod tree;

pub mod iters;
pub mod parse;

#[doc(inline)]
pub use item::*;
#[doc(inline)]
pub use node::*;
#[doc(inline)]
pub use prop::*;
#[doc(inline)]
pub use tree::*;
