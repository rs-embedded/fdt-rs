//! Low level flattened device tree parsing functions.
//!

use core::mem::size_of;

use num_traits::FromPrimitive;

use crate::base::DevTree;
use crate::error::{DevTreeError, Result};
use crate::priv_util::SliceRead;
use crate::spec::{fdt_prop_header, FdtTok, MAX_NODE_NAME_LEN};

use fallible_iterator::FallibleIterator;

/// This function implements the logic to tokenize the device tree's main structure block.
///
/// This function will return the next [`ParsedTok`] if one exists. If it succeeds in parsing
/// a token, `off` will be incremented to the start of the next token within `buf`.
///
/// # Safety
///
/// 1. The provided buffer must contain a device tree structure block.
///
/// 2. The given offset into the buffer, `off`, must be u32 aligned.
///
///    If this function returns `Ok(Some(_))`, the offset is guaranteed to be u32 aligned.  This
///    means that as long as this function is initially called with an aligned offset, this
///    function may be iteratively called without checking the offset's alignment again.
///
pub unsafe fn next_devtree_token<'a>(
    buf: &'a [u8],
    off: &mut usize,
) -> Result<Option<ParsedTok<'a>>> {
    // These are guaranteed.
    // We only produce associated offsets that are aligned to 32 bits and within the buffer.
    debug_assert!(buf.as_ptr().add(*off) as usize % size_of::<u32>() == 0);
    debug_assert!(buf.len() > (*off + size_of::<u32>()));

    let fdt_tok_val = buf.unsafe_read_be_u32(*off)?;
    *off += size_of::<u32>();

    match FromPrimitive::from_u32(fdt_tok_val) {
        Some(FdtTok::BeginNode) => {
            // Read the name (or return an error if the device tree is incorrectly formatted).
            let name = buf.nread_bstring0(*off, MAX_NODE_NAME_LEN - 1)?;

            // Move to the end of name (adding null byte).
            *off += name.len() + 1;
            // Per spec - align back to u32.
            *off += buf.as_ptr().add(*off).align_offset(size_of::<u32>());

            Ok(Some(ParsedTok::BeginNode(ParsedBeginNode { name })))
        }
        Some(FdtTok::Prop) => {
            // Get the memory we'll use as the header
            let header_slice = buf
                .get(*off..*off + size_of::<fdt_prop_header>())
                .ok_or(DevTreeError::ParseError)?;
            // Re-interpret the data as a fdt_header.
            //
            // We already checked length.
            // We statically verify alignment by ensuring pointer alignment matches known u32 alignment.
            assert_eq_align!(fdt_prop_header, u32);
            #[allow(clippy::cast_ptr_alignment)]
            let header = &*(header_slice.as_ptr() as *const fdt_prop_header);
            let prop_len = u32::from(header.len) as usize;

            // Move offset past prop header
            *off += size_of::<fdt_prop_header>();
            // Create a slice using the offset
            let prop_buf = buf
                .get(*off..*off + prop_len)
                .ok_or(DevTreeError::ParseError)?;

            // Move the offset past the prop data.
            *off += prop_buf.len();
            // Align back to u32.
            *off += buf.as_ptr().add(*off).align_offset(size_of::<u32>());

            let name_offset = u32::from(header.nameoff) as usize;
            if name_offset > buf.len() {
                return Err(DevTreeError::ParseError);
            }
            let name_offset = name_offset;

            Ok(Some(ParsedTok::Prop(ParsedProp {
                name_offset,
                prop_buf,
            })))
        }
        Some(FdtTok::EndNode) => Ok(Some(ParsedTok::EndNode)),
        Some(FdtTok::Nop) => Ok(Some(ParsedTok::Nop)),
        Some(FdtTok::End) => Ok(None),
        None => {
            // Invalid token
            Err(DevTreeError::ParseError)
        }
    }
}

pub struct ParsedBeginNode<'a> {
    pub name: &'a [u8],
}

pub struct ParsedProp<'a> {
    pub prop_buf: &'a [u8],
    pub name_offset: usize,
}

/// Enumeration of all tokens within a device tree's structure block.
pub enum ParsedTok<'a> {
    BeginNode(ParsedBeginNode<'a>),
    EndNode,
    Prop(ParsedProp<'a>),
    Nop,
}

pub struct DevTreeParseIter<'r, 'dt: 'r> {
    pub offset: usize,
    pub fdt: &'r DevTree<'dt>,
}

impl<'r, 'dt: 'r> DevTreeParseIter<'r, 'dt> {
    pub(crate) fn new(fdt: &'r DevTree<'dt>) -> Self {
        Self {
            offset: fdt.off_dt_struct(),
            fdt,
        }
    }
}

impl<'dt, 'a: 'dt> FallibleIterator for DevTreeParseIter<'dt, 'a> {
    type Error = DevTreeError;
    type Item = ParsedTok<'a>;

    fn next(&mut self) -> Result<Option<Self::Item>> {
        // Safe because we're passing an unmodified (by us) offset.
        // next_devtree_token guaruntees alignment and out-of-bounds won't occur.
        unsafe { next_devtree_token(self.fdt.buf(), &mut self.offset) }
    }
}
