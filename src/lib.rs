#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate core;

extern crate endian_type;
extern crate ascii;

extern crate num;
#[macro_use]
extern crate num_derive;

use core::convert::From;
use core::iter::DoubleEndedIterator;
use core::mem::size_of;
use endian_type::types::*;

use num::FromPrimitive;

#[derive(FromPrimitive)]
enum FdtTok {
    BeginNode = 0x1,
    EndNode = 0x2,
    Prop = 0x3,
    Nop =0x4,
    End = 0x9,
}

//impl FdtTok {
//    fn as_be(self) -> u32_be {
//        u32_be::new(self as u32)
//    }
//}

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
    curr_addr: usize,
    fdt: &'a FdtBlob,
}

impl<'a> FdtReserveEntryIter<'a> {
    pub(self) fn new(fdt: &'a FdtBlob) -> Self {
        Self {
            curr_addr: Self::fdt_entry_base(fdt),
            fdt: fdt,
        }
    }

    fn fdt_entry_base(fdt: &FdtBlob) -> usize {
        u32::from(fdt.header().off_mem_rsvmap) as usize + fdt.base()
    }

    fn entry_base(&self) -> usize {
        Self::fdt_entry_base(self.fdt)
    }
}

impl<'a> DoubleEndedIterator for FdtReserveEntryIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        unsafe {
            let ptr = &*(self.curr_addr as *const fdt_reserve_entry);
            if self.curr_addr < self.entry_base() {
                return None;
            } else {
                self.curr_addr -= size_of::<fdt_reserve_entry>();
                return Some(ptr);
            }
        }
    }
}

impl<'a> Iterator for FdtReserveEntryIter<'a> {
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

#[derive(Clone, Debug)]
pub struct FdtNodeIter<'a> {
    curr_addr: usize,
    fdt: &'a FdtBlob,
}

pub struct FdtNode<'a> {
    pub name: &'a str,
    fdt: &'a FdtBlob,
}

impl<'a> FdtNodeIter<'a> {
    pub(self) fn new(fdt: &'a FdtBlob) -> Self {
        Self {
            curr_addr: Self::entry_base(fdt),
            fdt: fdt,
        }
    }

    fn entry_base(fdt: &FdtBlob) -> usize {
        u32::from(fdt.header().off_dt_struct) as usize + fdt.base()
    }

    //fn self_entry_base(&self) -> usize {
    //    Self::entry_base(self.fdt)
    //}
}

impl<'a> DoubleEndedIterator for FdtNodeIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        todo!();
    }
}

impl<'a> Iterator for FdtNodeIter<'a> {
    type Item = FdtNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            // Assert because we should end before the FDT_END occurs
            //
            // Since the intent of this library is to execute in a safety context, we might want to
            // just return None and perform this as a separate method.
            assert!(self.fdt.totalsize() > (self.curr_addr - self.fdt.base()));
            loop {
                let ptr = *(self.curr_addr as *const u32_be);
                match FromPrimitive::from_u32(u32::from(ptr)) {
                    Some(FdtTok::BeginNode) => {
                        // Advance the node by sizeof tok
                        self.curr_addr += size_of::<u32_be>();
                        let c_str_start = self.curr_addr;

                        let mut c_str_end: usize = 0;
                        let mut broke: bool = false;
                        for c_addr in self.curr_addr..self.fdt.end() {
                            if *(c_addr as *const u8) == 0 {
                                 // Include the null byte.
                                c_str_end = c_addr + 1;
                                broke = true;
                                break;
                            }
                        };
                        if !broke {
                            panic!("Unable to find the end of the FdtNode name.")
                        }

                        // Move to next u32 alignment after the str.
                        let c_str_size = c_str_end - c_str_start;
                        self.curr_addr += c_str_size;
                        self.curr_addr += (self.curr_addr as *const u8).align_offset(size_of::<u32_be>());


                        let str_slice = core::slice::from_raw_parts(c_str_start as *const u8, c_str_size);
                        return Some( FdtNode {
                            name: core::str::from_utf8(str_slice).unwrap(), 
                            fdt: self.fdt
                        });
                    },
                    Some(FdtTok::EndNode) => {
                        // Advance the node by sizeof tok
                        self.curr_addr += size_of::<u32_be>();
                        continue;
                    },
                    Some(FdtTok::Prop) => {
                        // Advance the node by sizeof tok + sizeof prop
                        self.curr_addr += size_of::<u32_be>();
                        let tmp = u32::from((*(self.curr_addr as *const fdt_prop_header)).len) as usize;
                        self.curr_addr += tmp;
                        self.curr_addr += size_of::<fdt_prop_header>();
                        self.curr_addr += (self.curr_addr as *const u8).align_offset(size_of::<u32_be>());
                        continue;
                    },
                    Some(FdtTok::Nop) => {
                        // Advance the node by sizeof tok
                        self.curr_addr += size_of::<u32_be>();
                        continue;
                    },
                    Some(FdtTok::End) => {
                        return None;
                    },
                    None => {
                        panic!("Unknown FDT Token Value {:}", u32::from(ptr));
                    },
                };
            };

        }
    }
}


#[derive(Copy, Clone, Debug)]
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
        u32::from(self.header().totalsize) as usize
    }

    /// Return the one past the last address of the fdt
    pub(self) fn end(&self) -> usize {
        self.base as usize + self.totalsize()
    }

    /// An iterator over the Device Tree "5.3 Memory Reservation Blocks"
    pub fn reserved_entries(&self) -> FdtReserveEntryIter {
        FdtReserveEntryIter::new(self)
    }

    /// An iterator over the Device Tree "5.3 Memory Reservation Blocks"
    pub fn nodes(&self) -> FdtNodeIter {
        FdtNodeIter::new(self)
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
            let blob = crate::FdtBlob::new(vec.as_ptr() as usize).unwrap();
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
            let blob = crate::FdtBlob::new(vec.as_ptr() as usize).unwrap();
            for node in blob.nodes() {
                println!("{}", node.name);
            }
            assert!(blob.nodes().count() == 27);
        }

        // Wait until the end to drop in since we alias the address.
        std::mem::drop(vec);
    }
}
