//! Definitions of structs and enums from the device tree specification.
use endian_type::types::{u32_be, u64_be};
use num_derive::FromPrimitive;

/// Magic number used to denote the beginning of a device tree (as a native machine number).
pub const FDT_MAGIC: u32 = 0xd00d_feed;
/// Maximum length of a device tree node name (including null byte)
pub const MAX_NODE_NAME_LEN: usize = 31;

/// Definition of the parsed phandle as a native machine number
pub type Phandle = u32;

/// An enumeration of the tokens used to separate sections within the `dt_struct` section of the FDT.
#[derive(FromPrimitive)]
pub enum FdtTok {
    BeginNode = 0x1,
    EndNode = 0x2,
    Prop = 0x3,
    Nop = 0x4,
    End = 0x9,
}

/// The `fdt_header` (Flattened Device Tree Header) as described by the specification
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

/// The `fdt_prop_header` (Flattened Device Tree Property header) as described by the specification
#[repr(C)]
pub struct fdt_prop_header {
    /// Length of the property data
    pub len: u32_be,
    /// Offset of the property name string within the dt_strings section
    pub nameoff: u32_be,
}

#[repr(C)]
pub struct fdt_reserve_entry {
    /// Starting address of the reserved memory region
    pub address: u64_be,
    /// Size of the reserved memory region
    pub size: u64_be,
}
