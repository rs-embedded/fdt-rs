pub use core::{convert, fmt, option, result, str};

#[inline]
pub fn align(val: usize, to: usize) -> usize {
    val + (to - (val % to)) % to
}

#[derive(Debug, Copy, Clone)]
pub enum SliceReadError {
    UnexpectedEndOfInput,
}

pub type SliceReadResult<T> = Result<T, SliceReadError>;

pub trait SliceRead {
    fn read_be_u32(&self, pos: usize) -> SliceReadResult<u32>;
    fn read_be_u64(&self, pos: usize) -> SliceReadResult<u64>;
    fn read_bstring0(&self, pos: usize) -> SliceReadResult<&[u8]>;
    fn read_bstring0_bounded(&self, pos: usize, len: usize) -> SliceReadResult<&[u8]>;
    fn subslice(&self, start: usize, len: usize) -> SliceReadResult<&[u8]>;
}

impl<'a> SliceRead for &'a [u8] {
    fn read_be_u32(&self, pos: usize) -> SliceReadResult<u32> {
        // check size is valid
        if !(pos + 4 <= self.len()) {
            return Err(SliceReadError::UnexpectedEndOfInput);
        }

        Ok((self[pos] as u32) << 24
            | (self[pos + 1] as u32) << 16
            | (self[pos + 2] as u32) << 8
            | (self[pos + 3] as u32))
    }

    fn read_be_u64(&self, pos: usize) -> SliceReadResult<u64> {
        // check size is valid
        if !(pos + 8 <= self.len()) {
            return Err(SliceReadError::UnexpectedEndOfInput);
        }

        Ok((self[pos] as u64) << 56
            | (self[pos + 1] as u64) << 48
            | (self[pos + 2] as u64) << 40
            | (self[pos + 3] as u64) << 32
            | (self[pos + 4] as u64) << 24
            | (self[pos + 5] as u64) << 16
            | (self[pos + 6] as u64) << 8
            | (self[pos + 7] as u64))
    }

    fn read_bstring0_bounded(&self, pos: usize, len: usize) -> SliceReadResult<&[u8]> {
        let mut cur = pos;
        let end = pos + len;
        while cur < self.len() {
            if cur >=  end {
                return Err(SliceReadError::UnexpectedEndOfInput);
            }
            if self[cur] == 0 {
                return Ok(&self[pos..cur]);
            }
            cur += 1;
        }

        Err(SliceReadError::UnexpectedEndOfInput)
    }

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

    fn subslice(&self, start: usize, end: usize) -> SliceReadResult<&[u8]> {
        if !(end < self.len()) {
            return Err(SliceReadError::UnexpectedEndOfInput);
        }

        Ok(&self[start..end])
    }
}
