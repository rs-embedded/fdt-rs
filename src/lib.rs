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
#[cfg(all(any(feature = "doctest", doc), feature = "std"))]
#[doc(hidden)]
pub mod doctest {
    pub use crate::base::*;
    pub use crate::index::*;
    pub use crate::prelude::*;

    // from https://github.com/rust-lang/cargo/issues/383#issuecomment-720873790
    macro_rules! external_doc_test {
        ($x:expr) => {
            #[doc = $x]
            extern "C" {}
        };
    }

    external_doc_test!(include_str!("../README.md"));

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
