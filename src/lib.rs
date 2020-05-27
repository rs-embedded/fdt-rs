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
#[macro_use]
extern crate cfg_if;
extern crate endian_type;
#[macro_use]
extern crate memoffset;

pub mod spec;
mod iters;
mod buf_util;

use core::convert::From;
use core::mem::size_of;
use buf_util::{SliceRead, SliceReadError};
use spec::{Phandle, fdt_header, FDT_MAGIC};

cfg_if! {
    if #[cfg(feature = "ascii")] {
        extern crate ascii;

        pub type StrError = ascii::AsAsciiStrError;
        pub type Str = ascii::AsciiStr;
        fn bytes_as_str(buf: &[u8]) -> Result<& Str, StrError> {
            ascii::AsciiStr::from_ascii(buf)
        }
    } else {

        pub type StrError = core::str::Utf8Error;
        pub type Str = str;
        fn bytes_as_str(buf: &[u8]) -> Result<& Str, StrError> {
            core::str::from_utf8(buf)
        }
    }
}

macro_rules! get_be32_field {
    ( $f:ident, $s:ident , $buf:expr ) => {
        $buf.read_be_u32(offset_of!($s, $f))
    };
}

/// An error describe parsing problems when creating device trees.
#[derive(Debug, Clone, Copy)]
pub enum DevTreeError {
    /// The magic number FDT_MAGIC was not found at the start of the
    /// structure.
    InvalidMagicNumber,

    /// Unable to safely read data from the given device tree using the supplied offset 
    InvalidOffset,

    /// The data format was not as expected at the given buffer offset
    ParseError,

    /// While trying to convert a string that was supposed to be ASCII, invalid
    /// `Str` sequences were encounter. 
    ///
    /// Note, the underlying type will differ based on use of the `ascii` feature.
    StrError(StrError),

    /// The device tree version is not supported by this library.
    VersionNotSupported,
    Eof,
}

impl From<SliceReadError> for DevTreeError {
    fn from(_: SliceReadError) -> DevTreeError {
        DevTreeError::ParseError
    }
}

impl From<StrError> for DevTreeError {
    fn from(e: StrError) -> DevTreeError {
        DevTreeError::StrError(e)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct DevTree<'a> {
    buf: &'a [u8],
}

/// # Safety
/// TODO
/// Retrieve the size of the device tree without providing a sized header.
///
/// A buffer of MIN_HEADER_SIZE is required.
#[inline]
pub unsafe fn read_totalsize(buf: &[u8]) -> Result<usize, DevTreeError> {
    verify_magic(buf)?;
    Ok(get_be32_field!(totalsize, fdt_header, buf)? as usize)
}

unsafe fn verify_magic(buf: &[u8]) -> Result<(), DevTreeError> {
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
            Err(DevTreeError::ParseError)
        } else {
            Ok(Self { buf })
        }
    }

    fn totalsize(&self) -> usize {
        unsafe {
            get_be32_field!(totalsize, fdt_header, self.buf).unwrap() as usize
        }
    }

    fn off_mem_rsvmap(&self) -> usize {
        unsafe {
            get_be32_field!(off_mem_rsvmap, fdt_header, self.buf).unwrap() as usize
        }
    }

    fn off_dt_struct(&self) -> usize {
        unsafe {
            get_be32_field!(off_dt_struct, fdt_header, self.buf).unwrap() as usize
        }
    }

    #[allow(dead_code)]
    fn off_dt_strings(&self) -> usize {
        unsafe {
            get_be32_field!(off_dt_strings, fdt_header, self.buf).unwrap() as usize
        }
    }

    /// # Safety
    /// TODO
    unsafe fn ptr_at<T>(&self, offset: usize) -> Result<*const T, DevTreeError> {
        if offset + size_of::<T>() > self.buf.len() {
            Err(DevTreeError::InvalidOffset)
        } else {
            Ok(self.buf.as_ptr().add(offset) as *const T)
        }
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
    pub name: Result<&'a Str, DevTreeError>,
    inner_iter: iters::DevTreeParseIter<'a>,
}

impl<'a> DevTreeNode<'a> {
    fn new(name: Result<&'a Str, DevTreeError>, inner_iter: iters::DevTreeParseIter<'a>) -> Self {
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
    pub fn name(&self) -> Result<&'a Str, DevTreeError> {
        self.get_prop_str()
    }

    fn get_prop_str(&self) -> Result<&'a Str, DevTreeError> {
        unsafe {
            let str_offset = self.iter.fdt.off_dt_strings() + self.nameoff;
            let name = self.iter.fdt.buf.read_bstring0(str_offset)?;
            Ok(bytes_as_str(name)?)
        }
    }

    pub fn length(&self) -> usize {
        self.propbuf.len()
    }

    /// # Safety
    /// TODO
    pub unsafe fn get_u32(&self, offset: usize) -> Result<u32, DevTreeError> {
        self.propbuf
            .read_be_u32(offset)
            .or(Err(DevTreeError::InvalidOffset))
    }

    /// # Safety
    /// TODO
    pub unsafe fn get_u64(&self, offset: usize) -> Result<u64, DevTreeError> {
        self.propbuf
            .read_be_u64(offset)
            .or(Err(DevTreeError::InvalidOffset))
    }

    /// # Safety
    /// TODO
    pub unsafe fn get_phandle(&self, offset: usize) -> Result<Phandle, DevTreeError> {
        self.propbuf
            .read_be_u32(offset)
            .or(Err(DevTreeError::InvalidOffset))
    }

    /// # Safety
    /// TODO
    pub unsafe fn get_str(&'a self, offset: usize) -> Result<&'a Str, DevTreeError> {
        match self.get_string(offset, true) {
            // Note, unwrap invariant is safe.
            // get_string returns Some(s) when second opt is true
            Ok((_, s)) => Ok(s.unwrap()),
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
    pub unsafe fn get_strlist(&'a self, list: &mut [Option<&'a Str>]) -> Result<usize, DevTreeError> {
        self.iter_str_list(Some(list))
    }

    /// # Safety
    /// TODO
    pub unsafe fn get_raw(&self) -> &'a [u8] {
        self.propbuf
    }

    /// # Safety
    /// TODO
    unsafe fn get_string( &'a self, offset: usize, parse: bool) -> Result<(usize, Option<&'a Str>), DevTreeError> {
        match self.propbuf.read_bstring0(offset) {
            Ok(res_u8) => {
                if res_u8.is_empty() {
                    return Err(DevTreeError::InvalidOffset);
                }

                // Include null byte
                let len = res_u8.len() + 1;

                if parse {
                    match bytes_as_str(res_u8) {
                        Ok(s) => {
                            Ok((len, Some(s)))
                        }
                        Err(e) => Err(e.into()),
                    }
                } else {
                    Ok((len, None))
                }
            }
            Err(e) => Err(e.into()),
        }
    }

    /// # Safety
    /// TODO
    unsafe fn iter_str_list(
        &'a self,
        mut list_opt: Option<&mut [Option<&'a Str>]>,
    ) -> Result<usize, DevTreeError> {
        let mut offset = 0;
        for count in 0.. {
            if offset == self.length() {
                return Ok(count);
            }

            let (len, s) = self.get_string(offset, list_opt.is_some())?;
            offset += len;

            if let Some(list) = list_opt.as_deref_mut() {
                // Note, unwrap invariant is safe.
                // get_string returns Some(s) if list_opt is Some(list)
                (*list)[count] = Some(s.unwrap());
            };
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
    use crate::Str;

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
                            let mut vec: Vec<Option<&Str>> = vec![None; i.unwrap()];
                            prop.get_strlist(&mut vec).unwrap();

                            let mut iter = vec.iter();

                            while let Some(Some(s)) = iter.next() {
                                print!("\t\t{} ", s);
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
