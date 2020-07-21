//! A flattened device tree (FDT) parser for embedded, low memory, and safety-critical no-std
//! environments.
//!
//! Includes the following features:
//!
//! * [Low-level FDT parsing utilities to build your own library](base::parse)
//! * [Simple utilites based on in-order parsing of the FDT](base)
//! * [Performant utilities which leverage an index built over the FDT](index)
//!
//! ## Features
//!
//! This crate can be used without the standard library (`#![no_std]`) by disabling
//! the default `std` feature. To use `no-std` place the following in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies.fdt-rs]
//! version = "x"
//! default-features = false
//! ```
//!
//! ## Examples
//!
//!
#![deny(clippy::all, clippy::cargo)]
#![allow(clippy::as_conversions)]
// Test the readme if using nightly.
#![cfg_attr(RUSTC_IS_NIGHTLY, feature(external_doc))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate core;
extern crate endian_type_rs as endian_type;
#[macro_use]
extern crate memoffset;
#[macro_use]
extern crate static_assertions;
extern crate fallible_iterator;
extern crate unsafe_unwrap;

pub mod base;
pub mod error;
pub mod index;
pub mod prelude;
pub mod spec;

#[doc(hidden)]
pub mod common;

pub(crate) mod priv_util;

// When the doctest feature is enabled, add these utility functions.
#[cfg(any(feature = "doctest", doc))]
#[doc(hidden)]
pub mod doctest {
    pub use crate::base::*;
    pub use crate::index::*;
    pub use crate::prelude::*;

    // Include the readme for doctests
    // https://doc.rust-lang.org/rustdoc/documentation-tests.html#include-items-only-when-collecting-doctests
    #[cfg(RUSTC_IS_NIGHTLY)]
    #[doc(include = "../README.md")]
    pub struct ReadmeDoctests;

    #[repr(align(4))]
    struct _Wrapper<T>(T);
    pub const FDT: &[u8] = &_Wrapper(*include_bytes!("../tests/riscv64-virt.dtb")).0;

    pub fn doctest_index<'i, 'dt: 'i>() -> (DevTreeIndex<'i, 'dt>, Vec<u8>) {
        // Create the device tree parser
        let devtree = unsafe { DevTree::new(FDT) }.unwrap();

        // Get the layout required to build an index
        let layout = DevTreeIndex::get_layout(&devtree).unwrap();

        // Allocate memory for the index.
        //
        // This could be performed without a dynamic allocation
        // if we allocated a static buffer or want to provide a
        // raw buffer into uninitialized memory.
        let mut vec = vec![0u8; layout.size() + layout.align()];
        let (p, s) = (vec.as_mut_ptr(), vec.len());
        unsafe {
            let vec_copy = core::slice::from_raw_parts_mut(p, s);
            (DevTreeIndex::new(devtree, vec_copy).unwrap(), vec)
        }
    }
}
