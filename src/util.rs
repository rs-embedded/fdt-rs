use core::mem::size_of;
use core::ptr::read_unaligned;
pub use core::{convert, fmt, option, result};

#[derive(Debug, Copy, Clone)]
pub enum SliceReadError {
    UnexpectedEndOfInput,
}

pub type SliceReadResult<T> = Result<T, SliceReadError>;

pub trait SliceRead {
    fn read_be_u32(&self, pos: usize) -> SliceReadResult<u32>;
    fn read_be_u64(&self, pos: usize) -> SliceReadResult<u64>;
    fn read_bstring0(&self, pos: usize) -> SliceReadResult<&[u8]>;
}

impl<'a> SliceRead for &'a [u8] {
    #[inline]
    fn read_be_u32(&self, pos: usize) -> SliceReadResult<u32> {
        // check size is valid
        if pos + size_of::<u32>() > self.len() {
            return Err(SliceReadError::UnexpectedEndOfInput);
        }

        // We explicitly read unaligned.
        #[allow(clippy::cast_ptr_alignment)]
        unsafe {
            Ok(read_unaligned::<u32>(self.as_ptr().add(pos) as *const u32).to_be())
        }
    }

    #[inline]
    fn read_be_u64(&self, pos: usize) -> SliceReadResult<u64> {
        // check size is valid
        if pos + size_of::<u64>() > self.len() {
            return Err(SliceReadError::UnexpectedEndOfInput);
        }

        // We explicitly read unaligned.
        #[allow(clippy::cast_ptr_alignment)]
        unsafe {
            Ok(read_unaligned::<u64>(self.as_ptr().add(pos) as *const u64).to_be())
        }
    }

    #[inline]
    fn read_bstring0(&self, pos: usize) -> SliceReadResult<&[u8]> {
        let mut cur = pos;
        while cur < self.len() {
            if self[cur] == 0 {
                return Ok(&self[pos..cur]);
            }
            cur += 1;
        }

        Err(SliceReadError::UnexpectedEndOfInput)
    }
}
