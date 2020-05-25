use endian_type::types::{u32_be, u64_be};
use num_derive::FromPrimitive;

pub const FDT_MAGIC: u32 = 0xd00d_feed;
#[derive(FromPrimitive)]
pub enum FdtTok {
    BeginNode = 0x1,
    EndNode = 0x2,
    Prop = 0x3,
    Nop = 0x4,
    End = 0x9,
}

// TODO: Remove the dependency on the list/big endian crate. We'll simply use these definitons to
// provide offsets for our buf reader.

// As defined by the spec.
#[repr(C)]
pub struct fdt_header {
    pub magic: u32_be,
    pub totalsize: u32_be,
    pub off_dt_struct: u32_be,
    pub off_dt_strings: u32_be,
    pub off_mem_rsvmap: u32_be,
    pub version: u32_be,
    pub last_comp_version: u32_be,
    pub boot_cpuid_phys: u32_be,
    pub size_dt_strings: u32_be,
    pub size_dt_struct: u32_be,
}

#[repr(C)]
pub struct fdt_prop_header {
    pub len: u32_be,
    pub nameoff: u32_be,
}

#[repr(C)]
pub struct fdt_reserve_entry {
    pub address: u64_be,
    pub size: u64_be,
}
