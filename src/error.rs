//! Errors reported by this library

#[cfg(doc)]
use crate::index::DevTreeIndex;

use crate::priv_util::SliceReadError;
use core::str::Utf8Error;
use core::fmt;
use core::result;

/// An error describe parsing problems when creating device trees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevTreeError {
    InvalidParameter(&'static str),

    /// The magic number FDT_MAGIC was not found at the start of the
    /// structure.
    InvalidMagicNumber,

    /// Unable to safely read data from the given device tree using the supplied offset
    InvalidOffset,

    /// The data was not formatted as expected.  This likely indicates an error in the Device Tree
    /// we're parsing.
    ParseError,

    /// While trying to convert a string that was supposed to be ASCII, invalid
    /// `str` sequences were encounter.
    StrError(Utf8Error),

    /// There wasn't enough memory to create a [`DevTreeIndex`].
    NotEnoughMemory,
}

impl From<SliceReadError> for DevTreeError {
    fn from(_: SliceReadError) -> DevTreeError {
        DevTreeError::ParseError
    }
}

impl From<Utf8Error> for DevTreeError {
    fn from(e: Utf8Error) -> DevTreeError {
        DevTreeError::StrError(e)
    }
}

/// The result of a parse.
pub type Result<T> = core::result::Result<T, DevTreeError>;

impl fmt::Display for DevTreeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> result::Result<(), fmt::Error> {
        match *self {
            DevTreeError::InvalidParameter(err) => write!(f, "Invalid paramter supplied: {}", err),
            DevTreeError::InvalidOffset => write!(f, "Invalid offset provided."),

            DevTreeError::InvalidMagicNumber => write!(f, "Device tree contains invalid magic number."),
            DevTreeError::ParseError => write!(f, "Failed to parse device tree. It is invalid."),
            DevTreeError::StrError(utf_err) => write!(f, "Failed to parse device tree string: {}", utf_err),

            DevTreeError::NotEnoughMemory => write!(f, "Unable to fit device tree index into the provided buffer."),
        }
    }
}
