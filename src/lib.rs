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
use core::str::Utf8Error;

use endian_type::types::{u32_be, u64_be};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

const FDT_MAGIC: u32 = 0xd00d_feed;

/// An error describe parsing problems when creating device trees.
#[derive(Debug)]
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

macro_rules! get_be32_field {
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
    fdt: &'a DevTree<'a>,
}

impl<'a> FdtReserveEntryIter<'a> {
    pub(self) fn new(fdt: &'a DevTree) -> Self {
        Self {
            offset: fdt.off_mem_rsvmap(),
            fdt,
        }
    }

    fn read(&self) -> Result<&'a fdt_reserve_entry, DevTreeError> {
        unsafe {
            // TODO alignment not guarunteed.
            if self.offset + size_of::<fdt_reserve_entry>() > self.fdt.buf.len() {
                Err(DevTreeError::InvalidLength)
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

pub struct DevTreeNode<'a> {
    pub name: &'a str,
    prop_offset: usize,
    #[allow(dead_code)]
    fdt: &'a DevTree<'a>,
}

impl<'a> DevTreeNode<'a> {
    fn props(&'a self) -> DevTreeNodePropIter<'a> {
        DevTreeNodePropIter::new(self)
    }
}

#[derive(Clone, Debug)]
pub struct DevTreeNodeIter<'a> {
    offset: usize,
    fdt: &'a DevTree<'a>,
}

impl<'a> DevTreeNodeIter<'a> {
    pub(self) fn new(fdt: &'a DevTree) -> Self {
        Self {
            offset: fdt.off_dt_struct(),
            fdt,
        }
    }
}

impl<'a> Iterator for DevTreeNodeIter<'a> {
    type Item = DevTreeNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match step_parse_device_tree(self.offset, self.fdt) {
                Ok(ParsedItem::Prop(p)) => {
                    self.offset = p.new_offset;
                }
                Ok(ParsedItem::Node(n)) => {
                    self.offset = n.new_offset;
                    return Some(Self::Item {
                        fdt: self.fdt,
                        name: n.name.unwrap(),
                        prop_offset: n.new_offset,
                    })
                }
                Err(DevTreeError::Eof) => return None,
                Err(e) => panic!("Unexpected condition: {:?}", e),
            }
        }
    }
}

struct ParsedNode<'a> {
    /// Offset of the property value within the FDT buffer.
    new_offset: usize,
    name: Result<&'a str, Utf8Error>,
}
struct ParsedProp<'a> {
    new_offset: usize,
    /// Offset of the property value within the FDT buffer.
    value_offset: usize,
    header: &'a fdt_prop_header,
}

enum ParsedItem<'a> {
    Node(ParsedNode<'a>),
    Prop(ParsedProp<'a>),
}

fn step_parse_device_tree<'a>(
    mut offset: usize,
    fdt: &'a DevTree,
) -> Result<ParsedItem<'a>, DevTreeError> {
    unsafe {
        // Assert because we should end before the FDT_END occurs
        //
        // Since the intent of this library is to execute in a safety context, we might want to
        // just return None and perform this as a separate method.
        assert!(fdt.totalsize() > offset);
        loop {
            let fdt_val = fdt.buf.read_be_u32(offset)?;
            let fdt_tok = FromPrimitive::from_u32(fdt_val);
            offset += size_of::<u32_be>();

            match fdt_tok {
                Some(FdtTok::BeginNode) => {
                    let name = fdt.buf.read_bstring0(offset)?;

                    // Move to next u32 alignment after the str (including null byte).
                    offset += name.len() + 1;
                    // Align back to u32.
                    offset += fdt.buf.as_ptr().add(offset).align_offset(size_of::<u32_be>());

                    return Ok(ParsedItem::Node(ParsedNode {
                        name: core::str::from_utf8(name),
                        new_offset: offset,
                    }));
                }
                Some(FdtTok::Prop) => {
                    if offset + size_of::<fdt_reserve_entry>() > fdt.buf.len() {
                        panic!("");
                    }
                    let header = transmute::<*const u8, *const fdt_prop_header>(
                        fdt.buf.as_ptr().add(offset),
                    );
                    let prop_len = u32::from((*header).len);

                    offset += (prop_len as usize) + size_of::<fdt_prop_header>();
                    let value_offset = offset;

                    // Align back to u32.
                    offset += fdt
                        .buf
                        .as_ptr()
                        .add(offset)
                        .align_offset(size_of::<u32_be>());
                    return Ok(ParsedItem::Prop(ParsedProp {
                        header: &*header,
                        new_offset: offset,
                        value_offset,
                    }));
                }
                Some(FdtTok::EndNode) => {}
                Some(FdtTok::Nop) => {}
                Some(FdtTok::End) => {
                    return Err(DevTreeError::Eof);
                }
                None => {
                    panic!("Unknown FDT Token Value {:}", fdt_val);
                }
            }
        }
    }
}

pub type Phandle = u32;

pub struct DevTreeProp<'a> {
    pub name: &'a str,
    pub length: usize,
    pub node: &'a DevTreeNode<'a>,
}

pub struct DevTreeNodePropIter<'a> {
    offset: usize,
    pub node: &'a DevTreeNode<'a>,
}

impl<'a> DevTreeNodePropIter<'a> {
    fn new(node: &'a DevTreeNode) -> Self {
        Self {
            offset: node.prop_offset, // FIXME Nee proprty offset from this
            node: node,
        }
    }
}

impl<'a> Iterator for DevTreeNodePropIter<'a> {
    type Item = DevTreeProp<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        match step_parse_device_tree(self.offset, self.node.fdt) {
            Ok(ParsedItem::Prop(p)) => {
                self.offset = p.new_offset;

                Some(DevTreeProp {
                name: "todo - look up in string table",
                length: u32::from(p.header.len) as usize,
                node: self.node,
            })},
            Ok(ParsedItem::Node(_)) => {
                // If we hit a new node, we're done.
                None
            }
            Err(DevTreeError::Eof) => None,
            Err(e) => panic!("Unexpected condition: {:?}", e),
        }
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
    pub fn reserved_entries(&self) -> FdtReserveEntryIter {
        FdtReserveEntryIter::new(self)
    }

    /// An iterator over the Dev Tree "5.3 Memory Reservation Blocks"
    #[inline]
    #[must_use]
    pub fn nodes(&self) -> DevTreeNodeIter {
        DevTreeNodeIter::new(self)
    }

    pub fn parse(_offset: &mut usize) {}
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
                println!("{}", node.name);
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
                println!("{}", node.name);
                for prop in node.props() {
                    println!("\t{}", prop.name);
                }
            }
        }

        // Wait until the end to drop in since we alias the address.
        std::mem::drop(vec);
    }
}
