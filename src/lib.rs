#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate core;

extern crate endian_type;

use core::convert::From;
use core::iter::DoubleEndedIterator;
use core::mem::size_of;
use endian_type::types::*;

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

#[repr(C)]
pub struct fdt_reserve_entry {
    pub address: u64_be,
    pub size: u64_be,
}

pub struct FdtReserveEntryItr<'a> {
    curr_addr: usize,
    fdt: &'a FdtBlob,
}

impl<'a> FdtReserveEntryItr<'a> {
    pub(self) fn new(fdt: &'a FdtBlob) -> Self {
        Self {
            curr_addr: Self::entry_base(fdt),
            fdt: fdt,
        }
    }

    fn entry_base(fdt: &FdtBlob) -> usize {
        u32::from(fdt.header().off_dt_struct) as usize + fdt.base()
    }

    fn self_entry_base(&self) -> usize {
        Self::entry_base(self.fdt)
    }
}

impl<'a> DoubleEndedIterator for FdtReserveEntryItr<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        unsafe {
            let ptr = &*(self.curr_addr as *const fdt_reserve_entry);
            if self.curr_addr < self.self_entry_base() {
                return None;
            } else {
                self.curr_addr -= size_of::<fdt_reserve_entry>();
                return Some(ptr);
            }
        }
    }
}

impl<'a> Iterator for FdtReserveEntryItr<'a> {
    type Item = &'a fdt_reserve_entry;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let ptr = &*(self.curr_addr as *const fdt_reserve_entry);
            if self.fdt.totalsize() > (self.curr_addr - self.fdt.base()) {
                return None;
            } else if ptr.size == 0.into() && ptr.address == 0.into() {
                return None;
            } else {
                self.curr_addr += size_of::<fdt_reserve_entry>();
                return Some(ptr);
            }
        }
    }
}

pub struct FdtBlob {
    base: *const fdt_header,
}

impl FdtBlob {
    pub unsafe fn new(base: usize) -> Result<FdtBlob, ()> {
        let header = base as *const fdt_header;

        if (*header).magic != 0xd00dfeed.into() {
            return Err(());
        }

        Ok(FdtBlob {
            base: base as *const fdt_header,
        })
    }

    fn header(&self) -> &fdt_header {
        unsafe { &*self.base as &fdt_header }
    }

    pub fn base(&self) -> usize {
        self.base as usize
    }

    pub fn totalsize(&self) -> usize {
        return u32::from(self.header().totalsize) as usize;
    }

    /// An iterator over the Device Tree "5.3 Memory Reservation Blocks"
    pub fn reserved_entries(&self) -> FdtReserveEntryItr {
        return FdtReserveEntryItr::new(self);
    }
}

// Utilities to offer:
// - Iterate by compatible
// - Find node by compatible

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        use std::env::current_exe;
        use std::fs::File;
        use std::io::Read;
        use std::path::Path;

        let mut file = File::open("test/riscv64-virt.dtb").unwrap();
        let mut vec: Vec<u8> = Vec::new();
        let _ = file.read_to_end(&mut vec).unwrap();

        unsafe {
            let blob = crate::FdtBlob::new(vec.as_ptr() as usize).unwrap();
            assert!(blob.reserved_entries().count() == 0);
        }

        // Wait until the end to drop in since we alias the address.
        std::mem::drop(vec);
    }
}
