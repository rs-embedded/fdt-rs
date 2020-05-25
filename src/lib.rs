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

use endian_type::types::{u32_be, u64_be};
use num_derive::FromPrimitive;    
use num_traits::FromPrimitive;

const FDT_MAGIC: u32 = 0xd00d_feed;

/// An error describe parsing problems when creating device trees.
#[derive(Debug)]
pub enum DeviceTreeError {
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
}

impl From<SliceReadError> for DeviceTreeError {
    fn from(e: SliceReadError) -> DeviceTreeError {
        DeviceTreeError::SliceReadError(e)
    }
}

#[derive(FromPrimitive)]
enum FdtTok {
    BeginNode = 0x1,
    EndNode = 0x2,
    Prop = 0x3,
    Nop = 0x4,
    End = 0x9,
}

// As defined by the spec.
#[repr(C)]
struct fdt_header {
    magic: u32_be,
    totalsize: u32_be,
    off_dt_struct: u32_be,
    off_dt_strings: u32_be,
    off_mem_rsvmap: u32_be,
    version: u32_be,
    last_comp_version: u32_be,
    boot_cpuid_phys: u32_be,
    size_dt_strings: u32_be,
    size_dt_struct: u32_be,
}

macro_rules! get_be32 {
    ( $f:ident, $s:ident , $buf:expr ) => {
        $buf.read_be_u32(offset_of!($s, $f))
    };
}

#[repr(C)]
struct fdt_prop_header {
    len: u32_be,
    nameoff: u32_be,
}

#[repr(C)]
pub struct fdt_reserve_entry {
    pub address: u64_be,
    pub size: u64_be,
}

#[derive(Clone, Debug)]
pub struct FdtReserveEntryIter<'a> {
    offset: usize,
    fdt: &'a DeviceTree<'a>,
}

impl<'a> FdtReserveEntryIter<'a> {
    pub(self) fn new(fdt: &'a DeviceTree) -> Self {
        Self {
            offset: fdt.off_mem_rsvmap(),
            fdt,
        }
    }

    fn read(&self) -> Result<&'a fdt_reserve_entry, DeviceTreeError> {
        unsafe {
            // TODO alignment not guarunteed.
            if self.offset + size_of::<fdt_reserve_entry>() > self.fdt.buf.len() {
                Err(DeviceTreeError::InvalidLength)
            } else {
                Ok(transmute(self.fdt.buf.as_ptr().add(self.offset)))
            }
        }
    }
}

impl<'a> Iterator for FdtReserveEntryIter<'a> {
    type Item = &'a fdt_reserve_entry;
    fn next(&mut self) -> Option<Self::Item> {
        if self.offset > self.fdt.totalsize() {
            None
        } else {
            let ret = self.read().unwrap();
            if ret.address == 0.into() && ret.size == 0.into() {
                return None;
            }

            self.offset += size_of::<fdt_reserve_entry>();
            Some(ret)
        }
    }
}

pub struct DeviceTreeNode<'a> {
    pub name: &'a str,
    #[allow(dead_code)]
    fdt: &'a DeviceTree<'a>,
}

#[derive(Clone, Debug)]
pub struct DeviceTreeNodeIter<'a> {
    offset: usize,
    fdt: &'a DeviceTree<'a>,
}

impl<'a> DeviceTreeNodeIter<'a> {
    pub(self) fn new(fdt: &'a DeviceTree) -> Self {
        Self {
            offset: fdt.off_dt_struct(),
            fdt,
        }
    }
}

impl<'a> Iterator for DeviceTreeNodeIter<'a> {
    type Item = DeviceTreeNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO Unwraps should be moved out.
        // We should store errors with the iterator.
        unsafe {
            // Assert because we should end before the FDT_END occurs
            //
            // Since the intent of this library is to execute in a safety context, we might want to
            // just return None and perform this as a separate method.
            assert!(self.fdt.totalsize() > self.offset);

            loop {
                let fdt_val = self.fdt.buf.read_be_u32(self.offset).unwrap();
                let fdt_tok = FromPrimitive::from_u32(fdt_val);
                self.offset += size_of::<u32_be>();
                match fdt_tok {
                    Some(FdtTok::BeginNode) => {
                        let name = self.fdt.buf.read_bstring0(self.offset).unwrap();

                        // Move to next u32 alignment after the str (including null byte).
                        self.offset += name.len() + 1;
                        // Align back to u32.
                        self.offset += self.fdt.buf.as_ptr().add(self.offset)
                            .align_offset(size_of::<u32_be>());

                        return Some(DeviceTreeNode {
                            name: core::str::from_utf8(name).unwrap(),
                            fdt: self.fdt,
                        });
                    }
                    Some(FdtTok::Prop) => {
                        if self.offset + size_of::<fdt_reserve_entry>() > self.fdt.buf.len() {
                            panic!("");
                        }
                        let prop_len =  u32::from((*transmute::<*const u8, *const fdt_prop_header>(
                                self.fdt.buf.as_ptr().add(self.offset))).len);

                        self.offset += (prop_len as usize) + size_of::<fdt_prop_header>();
                        // Align back to u32.
                        self.offset += self.fdt.buf.as_ptr().add(self.offset)
                                       .align_offset(size_of::<u32_be>());
                        continue;
                    }
                    Some(FdtTok::EndNode) => {
                        continue;
                    }
                    Some(FdtTok::Nop) => {
                        continue;
                    }
                    Some(FdtTok::End) => {
                        return None;
                    }
                    None => {
                        panic!("Unknown FDT Token Value {:}", fdt_val);
                    }
                };

            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct DeviceTree<'a> {
    buf: &'a [u8],
}

/// Retrive the size of the device tree without providing a sized header.
///
/// A buffer of MIN_HEADER_SIZE is required.
pub unsafe fn read_totalsize(buf: &[u8]) -> Result<usize, DeviceTreeError> {
    verify_magic(buf)?;
    Ok(get_be32!(totalsize, fdt_header, buf)? as usize)
}

fn verify_magic(buf: &[u8]) -> Result<(), DeviceTreeError> {
    if get_be32!(magic, fdt_header, buf)? != FDT_MAGIC {
        Err(DeviceTreeError::InvalidMagicNumber)
    } else {
        Ok(())
    }
}


impl<'a> DeviceTree<'a> {
    pub const MIN_HEADER_SIZE: usize = size_of::<fdt_header>();

    pub unsafe fn new(buf: &'a [u8]) -> Result<Self, DeviceTreeError> {
        if read_totalsize(buf)? < buf.len() {
            Err(DeviceTreeError::InvalidLength)
        } else {
            Ok(Self { buf })
        }
    }

    fn totalsize(&self) -> usize {
        get_be32!(totalsize, fdt_header, self.buf).unwrap() as usize
    }
    fn off_mem_rsvmap(&self) -> usize {
        get_be32!(off_mem_rsvmap, fdt_header, self.buf).unwrap() as usize
    }
    fn off_dt_struct(&self) -> usize {
        get_be32!(off_dt_struct, fdt_header, self.buf).unwrap() as usize
    }
    #[allow(dead_code)]
    fn off_dt_strings(&self) -> usize {
        get_be32!(off_dt_strings, fdt_header, self.buf).unwrap() as usize
    }

    /// An iterator over the Device Tree "5.3 Memory Reservation Blocks"
    #[inline]
    #[must_use]
    pub fn reserved_entries(&self) -> FdtReserveEntryIter {
        FdtReserveEntryIter::new(self)
    }

    /// An iterator over the Device Tree "5.3 Memory Reservation Blocks"
    #[inline]
    #[must_use]
    pub fn nodes(&self) -> DeviceTreeNodeIter {
        DeviceTreeNodeIter::new(self)
    }
}

// Utilities to offer:
// - Iterate by compatible
// - Find node by compatible

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
            let blob = crate::DeviceTree::new(vec.as_slice()).unwrap();
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
            let blob = crate::DeviceTree::new(vec.as_slice()).unwrap();
            for node in blob.nodes() {
                println!("{}", node.name);
            }
            assert!(blob.nodes().count() == 27);
        }

        // Wait until the end to drop in since we alias the address.
        std::mem::drop(vec);
    }
}
