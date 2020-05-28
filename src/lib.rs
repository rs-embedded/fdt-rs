//! A flattened device tree parser for embedded, low memory, and safety-critical no-std environment
//!
//! * This device tree parser uses zero-allocation
//! * Remains safe even in the event of an invalid device tree
//! * Never performs misaligned reads
//!
//! ## Features
//!
//! This crate can be used without the standard library (`#![no_std]`) by disabling
//! the default `std` feature. To use `no-std` place the following in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies.fdt-rs]
//! version = "0.1"
//! default-features = false
//! # features = ["ascii"]    # <--- Uncomment if you wish to use the ascii crate for str's
//! ```
//!
//! Embeded software may not require the use of utf8 strings. For memory and processing constrained
//! environments ASCII may be suitable. For this reason, this crate supports the use of either
//! ascii or standard rust utf-8 `str`  types.
//!
//! Enabling the `"ascii"` feature will configure the `Str` type returned by string accessor
//! methods to be of type `AsciiStr` provided by the
//! [ascii crate](https://docs.rs/ascii/1.0.0/ascii/).
//!
#![deny(clippy::all, clippy::cargo)]
#![allow(clippy::as_conversions)]

#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "std")]
extern crate core;
#[macro_use]
extern crate cfg_if;
extern crate endian_type_rs as endian_type;
#[macro_use]
extern crate memoffset;

mod buf_util;
pub mod iters;
pub mod spec;

use buf_util::{SliceRead, SliceReadError};
use core::convert::From;
use core::mem::size_of;
use spec::{fdt_header, Phandle, FDT_MAGIC};

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

#[inline]
const fn is_aligned<T>(offset: usize) -> bool {
    offset % size_of::<T>() == 0
}

#[inline]
const fn verify_offset_aligned<T>(offset: usize) -> Result<usize, DevTreeError> {
    let i: [Result<usize, DevTreeError>; 2] = [Err(DevTreeError::ParseError), Ok(offset)];
    i[is_aligned::<T>(offset) as usize]
}

/// An error describe parsing problems when creating device trees.
#[derive(Debug, Clone, Copy)]
pub enum DevTreeError {
    /// The magic number FDT_MAGIC was not found at the start of the
    /// structure.
    InvalidMagicNumber,

    /// Unable to safely read data from the given device tree using the supplied offset
    InvalidOffset,

    /// The data was not formatted as expected.  This likely indicates an error in the Device Tree
    /// we're parsing.
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

/// A parseable Flattened Device Tree.
///
/// This parser was written according to the v0.3 specification provided at
/// https://www.devicetree.org/
#[derive(Copy, Clone, Debug)]
pub struct DevTree<'a> {
    buf: &'a [u8],
}

impl<'a> DevTree<'a> {
    pub const MIN_HEADER_SIZE: usize = size_of::<fdt_header>();
    /// Verify the magic header of a Device Tree buffer
    ///
    /// # Safety
    ///
    /// Callers of this method the must guarantee the following:
    /// - The passed buffer is 32-bit aligned.
    ///
    /// The passed byte buffer will be interpreted as a Flattened Device Tree. For this reason this API
    /// is marked unsafe.
    #[inline]
    pub unsafe fn verify_magic(buf: &[u8]) -> Result<(), DevTreeError> {
        if get_be32_field!(magic, fdt_header, buf)? != FDT_MAGIC {
            Err(DevTreeError::InvalidMagicNumber)
        } else {
            Ok(())
        }
    }

    /// Using the provided byte slice this method will:
    ///
    /// 1. Verify that the slice begins with the magic Device Tree header
    /// 2. Return the reported `totalsize` field of the Device Tree header
    ///
    /// When one must parse a Flattened Device Tree, it's possible that the actual size of the device
    /// tree may be unknown. For that reason, this method can be called before constructing the
    /// [`DevTree`].
    ///
    /// Once known, the user should resize the raw byte slice to this function's return value and
    /// pass that slice to [`DevTree::new()`].
    ///
    /// # Example
    ///
    /// ```
    /// let size = DevTree::read_totalsize(buf).unwrap();
    /// let buf = buf[..size];
    /// let dt = DevTree::new(buf).unwrap();
    /// ```
    ///
    /// # Safety
    ///
    /// Callers of this method the must guarantee the following:
    /// - The passed buffer is 32-bit aligned.
    /// - The passed buffer is of at least [`DevTree::MIN_HEADER_SIZE`] bytes in length
    ///
    /// The passed byte buffer will be interpreted as a Flattened Device Tree. For this reason this API
    /// is marked unsafe.
    #[inline]
    pub unsafe fn read_totalsize(buf: &[u8]) -> Result<usize, DevTreeError> {
        assert!(
            verify_offset_aligned::<u32>(buf.as_ptr() as usize).is_ok(),
            "Unaligned buffer provided"
        );
        Self::verify_magic(buf)?;
        Ok(get_be32_field!(totalsize, fdt_header, buf)? as usize)
    }

    /// Construct the parseable DevTree object from the provided byte slice.
    ///
    /// # Safety
    ///
    /// Callers of this method the must guarantee the following:
    /// - The passed buffer is 32-bit aligned.
    /// - The passed buffer is exactly the length returned by `Self::read_totalsize()`
    ///
    ///
    #[inline]
    pub unsafe fn new(buf: &'a [u8]) -> Result<Self, DevTreeError> {
        if Self::read_totalsize(buf)? < buf.len() {
            Err(DevTreeError::ParseError)
        } else {
            let ret = Self { buf };
            // Verify required alignment before returning.
            verify_offset_aligned::<u32>(ret.off_mem_rsvmap())?;
            verify_offset_aligned::<u32>(ret.off_dt_struct())?;
            Ok(ret)
        }
    }

    /// Returns the totalsize field of the Device Tree
    #[inline]
    #[must_use]
    pub fn totalsize(&self) -> usize {
        unsafe { get_be32_field!(totalsize, fdt_header, self.buf).unwrap() as usize }
    }

    /// Returns the of rsvmap offset field of the Device Tree
    #[inline]
    #[must_use]
    pub fn off_mem_rsvmap(&self) -> usize {
        unsafe { get_be32_field!(off_mem_rsvmap, fdt_header, self.buf).unwrap() as usize }
    }

    /// Returns the of dt_struct offset field of the Device Tree
    #[inline]
    #[must_use]
    pub fn off_dt_struct(&self) -> usize {
        unsafe { get_be32_field!(off_dt_struct, fdt_header, self.buf).unwrap() as usize }
    }

    /// Returns the of dt_strings offset field of the Device Tree
    #[inline]
    #[must_use]
    pub fn off_dt_strings(&self) -> usize {
        unsafe { get_be32_field!(off_dt_strings, fdt_header, self.buf).unwrap() as usize }
    }

    /// Returns a typed `*const T` to the given offset in the Device Tree buffer.
    ///
    /// # Safety
    ///
    /// Due to the unsafe nature of re-interpretation casts this method is unsafe.  This method
    /// will verify that enough space to fit type T remains within the buffer.
    ///
    /// The caller must verify that the pointer is not misaligned before it is dereferenced.
    #[inline]
    unsafe fn ptr_at<T>(&self, offset: usize) -> Result<*const T, DevTreeError> {
        if offset + size_of::<T>() > self.buf.len() {
            Err(DevTreeError::InvalidOffset)
        } else {
            Ok(self.buf.as_ptr().add(offset) as *const T)
        }
    }

    /// Returns an iterator over the Dev Tree "5.3 Memory Reservation Blocks"
    #[inline]
    #[must_use]
    pub fn reserved_entries(&self) -> iters::DevTreeReserveEntryIter {
        iters::DevTreeReserveEntryIter::new(self)
    }

    /// Returns an iterator over [`DevTreeNode`] objects
    #[inline]
    #[must_use]
    pub fn nodes(&self) -> iters::DevTreeNodeIter {
        iters::DevTreeNodeIter::new(self)
    }

    /// Returns an iterator over objects within the [`DevTreeItem`] enum
    #[inline]
    #[must_use]
    pub fn items(&self) -> iters::DevTreeIter {
        iters::DevTreeIter::new(self)
    }

    /// Map the supplied predicate over the [`DevTreeItem`] enum.
    ///
    /// If the predicate returns `true`, Some(([`DevTreeItem`], [`iters::DevTreeIter`])) will be returned.
    /// The [`iters::DevTreeIter`] may be used to continue searching through the tree.
    ///
    /// The predicate function may return true to simply terminate the search.
    ///
    /// # Example
    ///
    /// ```
    /// fn is_uart_compatible(item: &DevTreeItem) -> Bool {
    ///     match item {
    ///         DevTreeItem::Prop(p) => {
    ///         (p.name().unwrap() == "compatible") && p.get_str(0) == "ns16550a")
    ///         },
    ///         _ => false,
    ///     }
    /// }
    ///
    /// let devtree = DevTree::new(buf).unwrap();
    ///
    /// // Print the names of all compatible uarts
    /// if let Some((compatible_prop, mut iter)) = devtree.find(is_uart_compatible) {
    ///     println!(compatible_prop.parent().name()?);
    /// }
    ///
    /// // Continue the search and keep printing their names.
    /// while let Some((compatible_prop, mut iter)) = iter.find(is_uart_compatible) {
    ///     println!(compatible_prop.parent().name()?);
    /// }
    /// ```
    ///
    #[inline]
    pub fn find<F>(&'a self, predicate: F) -> Option<(DevTreeItem<'a>, iters::DevTreeIter<'a>)>
    where
        F: Fn(&DevTreeItem) -> bool,
    {
        iters::DevTreeIter::new(self).find(predicate)
    }

    /// Map the supplied predicate over all [`DevTreeProp`] objects
    ///
    /// If the predicate returns `true`, Some(([`DevTreeProp`], [`iters::DevTreePropIter`])) will be returned.
    /// The [`iters::DevTreePropIter`] may be used to continue searching through the tree.
    ///
    /// # Example
    ///
    /// ```
    /// let devtree = DevTree::new(buf).unwrap();
    ///
    /// // Print the name of a compatible node
    /// if let Some((compatible_prop, _)) = devtree.find_prop(|prop|
    ///     (prop.name == "compatible") && (p.get_str(0) == "ns16550a")) {
    ///     println!(compatible_prop.parent().name()?);
    /// }
    /// ```
    ///
    #[inline]
    pub fn find_prop<F>(&'a self, predicate: F) -> Option<(DevTreeProp<'a>, iters::DevTreePropIter<'a>)>
    where
        F: Fn(&DevTreeProp) -> bool,
    {
        iters::DevTreePropIter::new(self).find(predicate)
    }

    /// Map the supplied predicate over all [`DevTreeNode`] objects
    ///
    /// If the predicate returns `true`, Some(([`DevTreeItem`], [`iters::DevTreeNodeIter`])) will be returned.
    /// The [`iters::DevTreeNodeIter`] may be used to continue searching through the tree.
    #[inline]
    pub fn find_node<F>(&'a self, predicate: F) -> Option<(DevTreeNode<'a>, iters::DevTreeNodeIter<'a>)>
    where
        F: Fn(&DevTreeNode) -> bool,
    {
        iters::DevTreeNodeIter::new(self).find(predicate)
    }
}

/// An enum which contains either a [`DevTreeNode`] or a [`DevTreeProp`]
#[derive(Clone)]
pub enum DevTreeItem<'a> {
    Node(DevTreeNode<'a>),
    Prop(DevTreeProp<'a>),
}

/// A handle to a Device Tree Node within the device tree.
#[derive(Clone)]
pub struct DevTreeNode<'a> {
    name: Result<&'a Str, DevTreeError>,
    parse_iter: iters::DevTreeIter<'a>,
}

impl<'a> DevTreeNode<'a> {
    /// Returns the name of the `DevTreeNode` (including unit address tag)
    #[inline]
    pub fn name(&'a self) -> Result<&'a Str, DevTreeError> {
        self.name
    }

    /// Returns an iterator over this node's children [`DevTreeProp`]
    #[inline]
    #[must_use]
    pub fn props(&'a self) -> iters::DevTreeNodePropIter<'a> {
        iters::DevTreeNodePropIter::new(self)
    }
}

/// A handle to a [`DevTreeNode`]'s Device Tree Property
#[derive(Clone)]
pub struct DevTreeProp<'a> {
    parent_iter: iters::DevTreeIter<'a>,
    propbuf: &'a [u8],
    nameoff: usize,
}

impl<'a> DevTreeProp<'a> {
    /// Returns the name of the property within the device tree.
    #[inline]
    pub fn name(&self) -> Result<&'a Str, DevTreeError> {
        self.get_prop_str()
    }

    /// Returns the node which this property is attached to
    #[inline]
    #[must_use]
    pub fn parent(&self) -> DevTreeNode {
        self.parent_iter.clone().next_node().unwrap()
    }

    /// Returns the length of the property value within the device tree
    #[inline]
    #[must_use]
    pub fn length(&self) -> usize {
        self.propbuf.len()
    }

    /// Read a big-endian [`u32`] from the provided offset in this device tree property's value.
    /// Convert the read value into the machines' native [`u32`] format and return it.
    ///
    /// If an offset which would cause this read to access memory outside of this property's value
    /// an [`Err`] containing [`DevTreeError::InvalidOffset`] will be returned.
    ///
    /// # Safety
    ///
    /// Device Tree Properties are not strongly typed therefore any dereference could return
    /// unexpected data.
    ///
    /// This method will access memory using [`core::ptr::read_unaligned`], therefore an unaligned
    /// offset may be provided.
    ///
    /// This method will *not* panic.
    #[inline]
    pub unsafe fn get_u32(&self, offset: usize) -> Result<u32, DevTreeError> {
        self.propbuf
            .read_be_u32(offset)
            .or(Err(DevTreeError::InvalidOffset))
    }

    /// Read a big-endian [`u64`] from the provided offset in this device tree property's value.
    /// Convert the read value into the machines' native [`u64`] format and return it.
    ///
    /// If an offset which would cause this read to access memory outside of this property's value
    /// an [`Err`] containing [`DevTreeError::InvalidOffset`] will be returned.
    ///
    /// # Safety
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
    #[inline]
    pub unsafe fn get_u64(&self, offset: usize) -> Result<u64, DevTreeError> {
        self.propbuf
            .read_be_u64(offset)
            .or(Err(DevTreeError::InvalidOffset))
    }

    /// A Phandle is simply defined as a u32 value, as such this method performs the same action as
    /// [`self.get_u32`]
    ///
    /// # Safety
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
    #[inline]
    pub unsafe fn get_phandle(&self, offset: usize) -> Result<Phandle, DevTreeError> {
        self.propbuf
            .read_be_u32(offset)
            .or(Err(DevTreeError::InvalidOffset))
    }

    /// Returns the string at the given offset if it can be parsed
    /// # Safety
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
    #[inline]
    pub unsafe fn get_str(&'a self, offset: usize) -> Result<&'a Str, DevTreeError> {
        match self.get_string(offset, true) {
            // Note, unwrap invariant is safe.
            // get_string returns Some(s) when second opt is true
            Ok((_, s)) => Ok(s.unwrap()),
            Err(e) => Err(e),
        }
    }

    /// # Safety
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
    #[inline]
    pub unsafe fn get_str_count(&self) -> Result<usize, DevTreeError> {
        self.iter_str_list(None)
    }

    /// # Safety
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
    #[inline]
    pub unsafe fn get_strlist(
        &'a self,
        list: &mut [Option<&'a Str>],
    ) -> Result<usize, DevTreeError> {
        self.iter_str_list(Some(list))
    }

    /// # Safety
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
    #[inline]
    pub unsafe fn get_raw(&self) -> &'a [u8] {
        self.propbuf
    }

    fn get_prop_str(&self) -> Result<&'a Str, DevTreeError> {
        unsafe {
            let str_offset = self.parent_iter.fdt.off_dt_strings() + self.nameoff;
            let name = self.parent_iter.fdt.buf.read_bstring0(str_offset)?;
            Ok(bytes_as_str(name)?)
        }
    }

    /// # Safety
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
    unsafe fn get_string(
        &'a self,
        offset: usize,
        parse: bool,
    ) -> Result<(usize, Option<&'a Str>), DevTreeError> {
        match self.propbuf.read_bstring0(offset) {
            Ok(res_u8) => {
                if res_u8.is_empty() {
                    return Err(DevTreeError::InvalidOffset);
                }

                // Include null byte
                let len = res_u8.len() + 1;

                if parse {
                    match bytes_as_str(res_u8) {
                        Ok(s) => Ok((len, Some(s))),
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
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
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
