use core::str::from_utf8;

use crate::prelude::*;

use crate::base::DevTree;
use crate::error::DevTreeError;
use crate::spec::Phandle;

#[cfg(doc)]
use crate::base::DevTreeProp;

pub trait PropReader<'dt> {
    type NodeType;

    /// Returns the buffer associtated with the property's data.
    #[doc(hidden)]
    fn propbuf(&self) -> &'dt [u8];

    /// Returns offset of this property's name in the device tree buffer.
    #[doc(hidden)]
    fn nameoff(&self) -> usize;

    #[doc(hidden)]
    fn fdt(&self) -> &DevTree<'dt>;

    /// Returns the name of the property within the device tree.
    #[inline]
    fn name(&self) -> Result<&'dt str, DevTreeError> {
        PropTraitWrap(self).get_prop_str()
    }

    /// Returns the length of the property value within the device tree
    #[inline]
    #[must_use]
    fn length(&self) -> usize {
        self.propbuf().len()
    }

    /// Returns the node which this property is contained within.
    fn node(&self) -> Self::NodeType;

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
    /// This method will access memory using [`core::ptr::read_unaligned`]; therefore an unaligned
    /// offset may be provided.
    ///
    /// This method will *not* panic.
    #[inline]
    unsafe fn get_u32(&self, offset: usize) -> Result<u32, DevTreeError> {
        self.propbuf()
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
    /// See the safety note of [`PropReader::get_u32`]
    #[inline]
    unsafe fn get_u64(&self, offset: usize) -> Result<u64, DevTreeError> {
        self.propbuf()
            .read_be_u64(offset)
            .or(Err(DevTreeError::InvalidOffset))
    }

    /// A Phandle is simply defined as a u32 value, as such this method performs the same action as
    /// [`self.get_u32`]
    ///
    /// # Safety
    ///
    /// See the safety note of [`PropReader::get_u32`]
    #[inline]
    unsafe fn get_phandle(&self, offset: usize) -> Result<Phandle, DevTreeError> {
        self.propbuf()
            .read_be_u32(offset)
            .or(Err(DevTreeError::InvalidOffset))
    }

    /// Returns the string property as a string if it can be parsed as one.
    /// # Safety
    ///
    /// See the safety note of [`PropReader::get_u32`]
    #[inline]
    unsafe fn get_str(&self) -> Result<&'dt str, DevTreeError> {
        self.get_str_at(0)
    }

    /// Returns the `str` at the given offset within the property.
    /// # Safety
    ///
    /// See the safety note of [`PropReader::get_u32`]
    #[inline]
    unsafe fn get_str_at(&self, offset: usize) -> Result<&'dt str, DevTreeError> {
        match PropTraitWrap(self).get_string(offset, true) {
            // Note, unwrap invariant is safe.
            // get_string returns Some(s) when second opt is true
            Ok((_, s)) => Ok(s.unwrap()),
            Err(e) => Err(e),
        }
    }

    /// # Safety
    ///
    /// See the safety note of [`PropReader::get_u32`]
    #[inline]
    unsafe fn get_str_count(&self) -> Result<usize, DevTreeError> {
        PropTraitWrap(self).iter_str_list(None)
    }

    /// Fills the supplied slice of references with [`str`] slices parsed from the given property.
    /// If parsing is successful, the number of parsed strings will be returned.
    ///
    /// If an error occurred while parsing one or more of the strings an [`Err`] of type
    /// [`DevTreeError`] will be returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use fdt_rs::doctest::*;
    /// # let (index, _) = doctest_index();
    ///
    /// // Find a node that is a compatible property.
    /// // (It should have a string value.)
    /// let compatible_prop = index.props().find(|prop|  {
    ///     if let Ok(name) = prop.name() {
    ///         return name == "compatible";
    ///     }
    ///     false
    /// }).unwrap();
    ///
    /// let mut str_list: [Option<&str>; 3] = [None; 3];
    ///
    /// unsafe {
    ///     assert_eq!(1, compatible_prop.get_strlist(&mut str_list).unwrap());
    ///     assert!(str_list[0].is_some());
    /// }
    ///
    ///
    /// ```
    ///
    ///
    /// # Safety
    ///
    /// See the safety note of [`PropReader::get_u32`]
    #[inline]
    unsafe fn get_strlist(&self, list: &mut [Option<&'dt str>]) -> Result<usize, DevTreeError> {
        PropTraitWrap(self).iter_str_list(Some(list))
    }

    /// Returns this property's data as a raw slice
    ///
    /// # Safety
    ///
    /// See the safety note of [`PropReader::get_u32`]
    #[inline]
    unsafe fn get_raw(&self) -> &'dt [u8] {
        self.propbuf()
    }
}

struct PropTraitWrap<'r, T: ?Sized>(&'r T);

impl<'r, 'dt: 'r, T: PropReader<'dt> + ?Sized> PropTraitWrap<'r, T> {
    fn get_prop_str(&self) -> Result<&'dt str, DevTreeError> {
        unsafe {
            let str_offset = self.0.fdt().off_dt_strings() + self.0.nameoff();
            let name = self.0.fdt().buf().read_bstring0(str_offset)?;
            Ok(from_utf8(name)?)
        }
    }

    /// # Safety
    ///
    /// See the safety note of [`PropReader::get_u32`]
    unsafe fn get_string(
        &self,
        offset: usize,
        parse: bool,
    ) -> Result<(usize, Option<&'dt str>), DevTreeError> {
        match self.0.propbuf().read_bstring0(offset) {
            Ok(res_u8) => {
                // Include null byte
                let len = res_u8.len() + 1;

                if parse {
                    match from_utf8(res_u8) {
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
    /// See the safety note of [`PropReader::get_u32`]
    unsafe fn iter_str_list(
        &self,
        mut list_opt: Option<&mut [Option<&'dt str>]>,
    ) -> Result<usize, DevTreeError> {
        let mut offset = 0;
        for count in 0.. {
            if offset == self.0.length() {
                return Ok(count);
            }

            let (len, s) = self.get_string(offset, list_opt.is_some())?;
            offset += len;

            if let Some(list) = list_opt.as_deref_mut() {
                // Note, unwrap invariant is safe.
                // get_string returns Some(s) if we ask it to parse and it returns Ok
                (*list)[count] = Some(s.unwrap());
            };
        }
        // Unreachable due to infinite for loop.
        unreachable!();
    }
}
