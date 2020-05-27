use core::mem::size_of;
use core::ptr::read_unaligned;

#[derive(Debug, Copy, Clone)]
pub enum SliceReadError {
    UnexpectedEndOfInput,
}

pub type SliceReadResult<T> = Result<T, SliceReadError>;

pub trait SliceRead {
    unsafe fn unsafe_read_be_u32(&self, pos: usize) -> SliceReadResult<u32>;
    unsafe fn unsafe_read_be_u64(&self, pos: usize) -> SliceReadResult<u64>;
    unsafe fn read_be_u32(&self, pos: usize) -> SliceReadResult<u32>;
    unsafe fn read_be_u64(&self, pos: usize) -> SliceReadResult<u64>;
    unsafe fn read_bstring0(&self, pos: usize) -> SliceReadResult<&[u8]>;
}

macro_rules! unchecked_be_read {
    ( $buf:ident, $type:ident , $off:expr ) => {
        (if $off + size_of::<$type>() > $buf.len() {
            Err(SliceReadError::UnexpectedEndOfInput)
        } else {
            Ok((*($buf.as_ptr().add($off) as *const $type)).to_be())
        })
    };
}

macro_rules! be_read {
    ( $buf:ident, $type:ident , $off:expr ) => {
        (if $off + size_of::<$type>() > $buf.len() {
            Err(SliceReadError::UnexpectedEndOfInput)
        } else {
            // We explicitly read unaligned.
            #[allow(clippy::cast_ptr_alignment)]
            Ok((read_unaligned::<$type>($buf.as_ptr().add($off) as *const $type)).to_be())
        })
    };
}

impl<'a> SliceRead for &'a [u8] {
    #[inline]
    unsafe fn unsafe_read_be_u32(&self, pos: usize) -> SliceReadResult<u32> {
        unchecked_be_read!(self, u32, pos)
    }

    #[inline]
    unsafe fn unsafe_read_be_u64(&self, pos: usize) -> SliceReadResult<u64> {
        unchecked_be_read!(self, u64, pos)
    }

    #[inline]
    unsafe fn read_be_u32(&self, pos: usize) -> SliceReadResult<u32> {
        be_read!(self, u32, pos)
    }

    #[inline]
    unsafe fn read_be_u64(&self, pos: usize) -> SliceReadResult<u64> {
        be_read!(self, u64, pos)
    }

    #[inline]
    unsafe fn read_bstring0(&self, pos: usize) -> SliceReadResult<&[u8]> {
        for i in pos..self.len() {
            if self[i] == 0 {
                return Ok(&self[pos..i]);
            }
        }
        Err(SliceReadError::UnexpectedEndOfInput)
    }
}
