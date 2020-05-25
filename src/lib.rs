#![deny(
     clippy::all,
     //clippy::cargo,
 )]
#![allow(clippy::as_conversions)]
#![allow(clippy::print_stdout)]
#![allow(clippy::implicit_return)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate core;
extern crate endian_type;
#[macro_use]
extern crate memoffset;

pub mod util;
use core::str;
use util::{SliceRead, SliceReadError};

use core::convert::From;
use core::mem::{size_of, transmute};

use num_traits::FromPrimitive;

pub mod iters;
pub mod spec;

use spec::*;

macro_rules! get_be32_field {
    ( $f:ident, $s:ident , $buf:expr ) => {
        $buf.read_be_u32(offset_of!($s, $f))
    };
}

pub type Phandle = u32;

/// An error describe parsing problems when creating device trees.
#[derive(Debug, Clone, Copy)]
pub enum DevTreeError {
    /// The magic number FDT_MAGIC was not found at the start of the
    /// structure.
    InvalidMagicNumber,
    /// TODO
    InvalidLength,
    /// Failed to read data from slice.
    SliceReadError(SliceReadError),

    /// The data format was not as expected at the given buffer offset
    ParseError(usize),

    /// While trying to convert a string that was supposed to be ASCII, invalid
    /// utf8 sequences were encounted
    Utf8Error,

    /// The device tree version is not supported by this library.
    VersionNotSupported,
    Eof,
}

impl From<SliceReadError> for DevTreeError {
    fn from(e: SliceReadError) -> DevTreeError {
        DevTreeError::SliceReadError(e)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct DevTree<'a> {
    buf: &'a [u8],
}

/// Retrive the size of the device tree without providing a sized header.
///
/// A buffer of MIN_HEADER_SIZE is required.
pub unsafe fn read_totalsize(buf: &[u8]) -> Result<usize, DevTreeError> {
    verify_magic(buf)?;
    Ok(get_be32_field!(totalsize, fdt_header, buf)? as usize)
}

fn verify_magic(buf: &[u8]) -> Result<(), DevTreeError> {
    if get_be32_field!(magic, fdt_header, buf)? != FDT_MAGIC {
        Err(DevTreeError::InvalidMagicNumber)
    } else {
        Ok(())
    }
}

impl<'a> DevTree<'a> {
    pub const MIN_HEADER_SIZE: usize = size_of::<fdt_header>();

    pub unsafe fn new(buf: &'a [u8]) -> Result<Self, DevTreeError> {
        if read_totalsize(buf)? < buf.len() {
            Err(DevTreeError::InvalidLength)
        } else {
            Ok(Self { buf })
        }
    }

    fn totalsize(&self) -> usize {
        get_be32_field!(totalsize, fdt_header, self.buf).unwrap() as usize
    }

    fn off_mem_rsvmap(&self) -> usize {
        get_be32_field!(off_mem_rsvmap, fdt_header, self.buf).unwrap() as usize
    }

    fn off_dt_struct(&self) -> usize {
        get_be32_field!(off_dt_struct, fdt_header, self.buf).unwrap() as usize
    }

    #[allow(dead_code)]
    fn off_dt_strings(&self) -> usize {
        get_be32_field!(off_dt_strings, fdt_header, self.buf).unwrap() as usize
    }

    /// An iterator over the Dev Tree "5.3 Memory Reservation Blocks"
    #[inline]
    #[must_use]
    pub fn reserved_entries(&self) -> iters::DevTreeReserveEntryIter {
        iters::DevTreeReserveEntryIter::new(self)
    }

    /// An iterator over the Dev Tree "5.3 Memory Reservation Blocks"
    #[inline]
    #[must_use]
    pub fn nodes(&self) -> iters::DevTreeNodeIter {
        iters::DevTreeNodeIter::new(self)
    }

    pub fn parse(_offset: &mut usize) {}
}

pub struct DevTreeNode<'a> {
    pub name: Result<&'a str, DevTreeError>,
    inner_iter: iters::DevTreeParseIter<'a>,
}

impl<'a> DevTreeNode<'a> {
    fn new(name: Result<&'a str, DevTreeError>, inner_iter: iters::DevTreeParseIter<'a>) -> Self {
        Self { name, inner_iter }
    }
    pub fn props(&'a self) -> iters::DevTreeNodePropIter<'a> {
        iters::DevTreeNodePropIter::new(self)
    }
}

pub struct DevTreeProp<'a> {
    iter: iters::DevTreeParseIter<'a>,
    //TODO
    #[allow(dead_code)]
    propoff: usize,
    #[allow(dead_code)]
    length: usize,
    nameoff: usize,
}

impl<'a> DevTreeProp<'a> {
    pub fn name(&self) -> Result<&'a str, DevTreeError> {
        self.iter.get_prop_str(self.nameoff)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn reserved_entries_iter() {
        use std::env::current_exe;
        use std::fs::File;
        use std::io::Read;
        use std::path::Path;

        let mut file = File::open("test/riscv64-virt.dtb").unwrap();
        let mut vec: Vec<u8> = Vec::new();
        let _ = file.read_to_end(&mut vec).unwrap();

        unsafe {
            let blob = crate::DevTree::new(vec.as_slice()).unwrap();
            assert!(blob.reserved_entries().count() == 0);
        }

        // Wait until the end to drop in since we alias the address.
        std::mem::drop(vec);
    }

    #[test]
    fn nodes_iter() {
        use std::env::current_exe;
        use std::fs::File;
        use std::io::Read;
        use std::path::Path;

        let mut file = File::open("test/riscv64-virt.dtb").unwrap();
        let mut vec: Vec<u8> = Vec::new();
        let _ = file.read_to_end(&mut vec).unwrap();

        unsafe {
            let blob = crate::DevTree::new(vec.as_slice()).unwrap();
            for node in blob.nodes() {
                println!("{}", node.name.unwrap());
            }
            assert!(blob.nodes().count() == 27);
        }

        // Wait until the end to drop in since we alias the address.
        std::mem::drop(vec);
    }

    #[test]
    fn node_prop_iter() {
        use std::env::current_exe;
        use std::fs::File;
        use std::io::Read;
        use std::path::Path;

        let mut file = File::open("test/riscv64-virt.dtb").unwrap();
        let mut vec: Vec<u8> = Vec::new();
        let _ = file.read_to_end(&mut vec).unwrap();

        unsafe {
            let blob = crate::DevTree::new(vec.as_slice()).unwrap();
            for node in blob.nodes() {
                println!("{}", node.name.unwrap());
                for prop in node.props() {
                    println!("\t{}", prop.name().unwrap());
                }
            }
        }

        // Wait until the end to drop in since we alias the address.
        std::mem::drop(vec);
    }
}
