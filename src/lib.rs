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
use core::mem::size_of;

use num_traits::FromPrimitive;

mod iters;
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
    Utf8Error(core::str::Utf8Error),

    /// The device tree version is not supported by this library.
    VersionNotSupported,
    Eof,
}

impl From<SliceReadError> for DevTreeError {
    fn from(e: SliceReadError) -> DevTreeError {
        DevTreeError::SliceReadError(e)
    }
}

impl From<core::str::Utf8Error> for DevTreeError {
    fn from(e: core::str::Utf8Error) -> DevTreeError {
        DevTreeError::Utf8Error(e)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct DevTree<'a> {
    buf: &'a [u8],
}

/// # Safety
/// TODO
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

    /// # Safety
    /// TODO
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

    /// # Safety
    /// TODO
    unsafe fn ptr_at<T>(&self, offset: usize) -> *const T {
        self.buf.as_ptr().add(offset) as *const T
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
    propbuf: &'a [u8],
    nameoff: usize,
}

impl<'a> DevTreeProp<'a> {
    pub fn name(&self) -> Result<&'a str, DevTreeError> {
        self.get_prop_str()
    }

    fn get_prop_str(&self) -> Result<&'a str, DevTreeError> {
        let str_offset = self.iter.fdt.off_dt_strings() + self.nameoff;
        let name = self.iter.fdt.buf.read_bstring0(str_offset)?;
        Ok(core::str::from_utf8(name)?)
    }

    pub fn length(&self) -> usize {
        self.propbuf.len()
    }

    /// # Safety
    /// TODO
    pub unsafe fn get_u32(&self, offset: usize) -> Result<u32, DevTreeError> {
        self.propbuf
            .read_be_u32(offset)
            .or(Err(DevTreeError::InvalidLength))
    }

    /// # Safety
    /// TODO
    pub unsafe fn get_u64(&self, offset: usize) -> Result<u64, DevTreeError> {
        self.propbuf
            .read_be_u64(offset)
            .or(Err(DevTreeError::InvalidLength))
    }

    /// # Safety
    /// TODO
    pub unsafe fn get_phandle(&self, offset: usize) -> Result<Phandle, DevTreeError> {
        self.propbuf
            .read_be_u32(offset)
            .or(Err(DevTreeError::InvalidLength))
    }

    /// # Safety
    /// TODO
    pub unsafe fn get_str(&'a self, offset: usize) -> Result<&'a str, DevTreeError> {
        let dummy = "";
        let mut _s = dummy;

        match self.get_string(offset, Some(&mut _s)) {
            Ok(_) => {
                assert!(dummy != _s);
                Ok(_s)
            }
            Err(e) => Err(e),
        }
    }

    /// # Safety
    /// TODO
    pub unsafe fn get_str_count(&self) -> Result<usize, DevTreeError> {
        self.iter_str_list(None)
    }

    /// # Safety
    /// TODO
    pub unsafe fn get_strlist(&'a self, list: &mut [&'a str]) -> Result<usize, DevTreeError> {
        self.iter_str_list(Some(list))
    }

    /// # Safety
    /// TODO
    pub unsafe fn get_raw(&self) -> &'a [u8] {
        self.propbuf
    }

    /// # Safety
    /// TODO
    unsafe fn get_string(
        &'a self,
        offset: usize,
        opt_str: Option<&mut &'a str>,
    ) -> Result<usize, DevTreeError> {
        match self.propbuf.read_bstring0(offset) {
            Ok(res_u8) => {
                if res_u8.is_empty() {
                    return Err(DevTreeError::InvalidLength);
                }

                // Include null byte
                let len = res_u8.len() + 1;

                match opt_str {
                    Some(s) => match str::from_utf8(res_u8) {
                        Ok(parsed_s) => {
                            *s = parsed_s;
                            Ok(len)
                        }
                        Err(e) => Err(e.into()),
                    },
                    None => Ok(len),
                }
            }
            Err(e) => Err(e.into()),
        }
    }

    /// # Safety
    /// TODO
    unsafe fn iter_str_list(
        &'a self,
        mut list_opt: Option<&mut [&'a str]>,
    ) -> Result<usize, DevTreeError> {
        let mut _s = "";
        let mut offset = 0;
        for count in 0.. {
            if offset == self.length() {
                return Ok(count);
            }

            let s = match &mut list_opt {
                Some(list) => {
                    if list.len() > count {
                        Some(&mut list[count])
                    } else {
                        None
                    }
                }
                None => None,
            };
            offset += self.get_string(offset, s)?;
        }
        // For some reason infinite for loops need unreachable.
        unreachable!();
    }
}

#[cfg(test)]
mod tests {
    use core::mem::size_of;
    use std::fs::File;
    use std::io::Read;

    #[test]
    fn reserved_entries_iter() {
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
        let mut file = File::open("test/riscv64-virt.dtb").unwrap();
        let mut vec: Vec<u8> = Vec::new();

        let dummy = "";

        let _ = file.read_to_end(&mut vec).unwrap();

        unsafe {
            let blob = crate::DevTree::new(vec.as_slice()).unwrap();
            for node in blob.nodes() {
                println!("{}", node.name.unwrap());
                for prop in node.props() {
                    println!("\t{}", prop.name().unwrap());
                    if prop.length() == size_of::<u32>() {
                        //println!("\t\t0x{:x}", prop.get_u32(0).unwrap());
                    }
                    if prop.length() > 0 {
                        let i = prop.get_str_count();
                        if i.is_ok() {
                            if i.unwrap() == 0 {
                                break;
                            }
                            let mut vec: Vec<&str> = vec![&dummy; i.unwrap()];
                            prop.get_strlist(&mut vec).unwrap();
                            for s in vec {
                                print!("\t\t{} ", s);
                                if s == dummy {
                                    break;
                                }
                            }
                            println!();
                        }
                    }
                }
            }
        }

        // Wait until the end to drop in since we alias the address.
        std::mem::drop(vec);
    }
}
