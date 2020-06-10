//! This module provides a collection of iterative parsers of the buf provided to initialze
//! a [`DevTree`].
use core::mem::size_of;
use core::num::NonZeroUsize;

use num_traits::FromPrimitive;

use super::buf_util::SliceRead;
use super::spec;
use super::spec::{fdt_prop_header, fdt_reserve_entry, FdtTok};
use super::{DevTree, DevTreeError, DevTreeItem, DevTreeNode, DevTreeProp};
use crate::bytes_as_str;

#[derive(Clone)]
pub struct DevTreeReserveEntryIter<'a> {
    offset: usize,
    fdt: &'a DevTree<'a>,
}

#[repr(transparent)]
struct AssociatedOffset<'a> (usize, core::marker::PhantomData<&'a [u8]>);

impl<'a> AssociatedOffset<'a> {
    fn new(val: usize, buf: &'a[u8]) -> Self {
        // NOTE: Doesn't even check alignment. 
        // (Both size and alignment must be guarunteed elsewhere.)
        assert!(val < buf.len());
        Self (
            val,
            core::marker::PhantomData,
        )
    }
}

impl<'a> DevTreeReserveEntryIter<'a> {
    pub(crate) fn new(fdt: &'a DevTree) -> Self {
        Self {
            offset: fdt.off_mem_rsvmap(),
            fdt,
        }
    }

    /// Return the current offset as a fdt_reserve_entry reference.
    ///
    /// # Safety
    ///
    /// The caller must verify that the current offset of this iterator is 32-bit aligned.
    /// (Each field is 32-bit aligned and they may be read individually.)
    unsafe fn read(&self) -> Result<&'a fdt_reserve_entry, DevTreeError> {
        Ok(&*self.fdt.ptr_at(self.offset)?)
    }
}

impl<'a> Iterator for DevTreeReserveEntryIter<'a> {
    type Item = &'a fdt_reserve_entry;
    fn next(&mut self) -> Option<Self::Item> {
        if self.offset > self.fdt.totalsize() {
            None
        } else {
            // We guaruntee the read will be aligned to 32 bytes because:
            // - We construct with guarunteed 32-bit aligned offset
            // - We always increment by an aligned amount
            let ret = unsafe { self.read().unwrap() };

            if ret.address == 0.into() && ret.size == 0.into() {
                return None;
            }
            self.offset += size_of::<fdt_reserve_entry>();
            Some(ret)
        }
    }
}

/// An iterator over all [`DevTreeItem`] objects.
#[derive(Clone)]
pub struct DevTreeIter<'a> {
    // TODO Replace with an associated offset
    offset: usize,
    current_prop_parent_off: Option<NonZeroUsize>,
    pub(crate) fdt: &'a DevTree<'a>,
}

impl<'a> DevTreeIter<'a> {
    pub(crate) fn new(fdt: &'a DevTree) -> Self {
        Self {
            offset: fdt.off_dt_struct(),
            current_prop_parent_off: None,
            fdt,
        }
    }

    fn current_node_itr(&self) -> Option<DevTreeIter<'a>> {
        match self.current_prop_parent_off {
            Some(offset) => Some(DevTreeIter {
                fdt: self.fdt,
                current_prop_parent_off: self.current_prop_parent_off,
                offset: offset.get(),
            }),
            None => None,
        }
    }

    /// Returns the next [`DevTreeNode`] found in the Device Tree
    pub fn next_node(&mut self) -> Option<DevTreeNode<'a>> {
        loop {
            match self.next() {
                Some(DevTreeItem::Node(n)) => return Some(n),
                Some(_) => {
                    continue;
                }
                _ => return None,
            }
        }
    }

    /// Returns the next [`DevTreeProp`] found in the Device Tree (regardless if it occurs on
    /// a different [`DevTreeNode`]
    pub fn next_prop(&mut self) -> Option<DevTreeProp<'a>> {
        loop {
            match self.next() {
                Some(DevTreeItem::Prop(p)) => return Some(p),
                // Return if a new node or an EOF.
                Some(DevTreeItem::Node(_)) => continue,
                _ => return None,
            }
        }
    }

    /// Returns the next [`DevTreeProp`] on the current node within in the Device Tree
    pub fn next_node_prop(&mut self) -> Option<DevTreeProp<'a>> {
        match self.next() {
            Some(DevTreeItem::Prop(p)) => Some(p),
            // Return if a new node or an EOF.
            _ => None,
        }
    }

    /// See the documentation of [`DevTree::find`]
    #[inline]
    pub fn find<F>(&mut self, predicate: F) -> Option<(DevTreeItem<'a>, Self)>
    where
        F: Fn(&DevTreeItem) -> Result<bool, DevTreeError>,
    {
        while let Some(i) = self.next() {
            if let Ok(true) = predicate(&i) {
                return Some((i, self.clone()));
            }
        }
        None
    }

    /// Returns the next [`DevTreeNode`] object with the provided compatible device tree property
    /// or `None` if none exists.
    #[inline]
    pub fn find_next_compatible_node(&self, string: &crate::Str) -> Option<DevTreeNode<'a>> {
        // Create a clone and turn it into a node iterator
        let mut iter = DevTreeNodeIter::from(self.clone());
        // If there is another node
        if iter.next().is_some() {
            // Iterate through its properties looking for the compatible string.
            let mut iter = DevTreePropIter::from(iter.0);
            if let Some((compatible_prop, _)) = iter.find(|prop| unsafe {
                Ok((prop.name()? == "compatible") && (prop.get_str()? == string))
            }) {
                return Some(compatible_prop.parent());
            }
        }
        None
    }

    fn next_devtree_item(&mut self) -> Option<DevTreeItem<'a>> {
        loop {
            let mut offset = AssociatedOffset::new(self.offset, self.fdt.buf);
            let res = unsafe { next_devtree_token(self.fdt.buf, &mut offset) };
            let ret = match res {
                Ok(Some(ParsedTok::BeginNode(node))) => {
                    self.current_prop_parent_off = unsafe {
                        Some(NonZeroUsize::new_unchecked(self.offset))
                    };
                    let mut next_iter = self.clone();
                    next_iter.offset = offset.0;
                    Some(DevTreeItem::Node(DevTreeNode {
                        parse_iter: next_iter,
                        name: bytes_as_str(node.name).map_err(|e| e.into()),
                    }))
                },
                Ok(Some(ParsedTok::Prop(prop))) =>  {
                    // Prop must come after a node.
                    let prev_node = match self.current_node_itr() {
                        Some(n) => n,
                        None => return None, // Devtree error - end iteration
                    };
                    Some(DevTreeItem::Prop(DevTreeProp {
                        parent_iter: prev_node,
                        propbuf: prop.prop_buf,
                        nameoff: prop.name_offset.0,
                    }))
                },
                Ok(Some(ParsedTok::EndNode)) => {
                    // The current node has ended. 
                    // No properties may follow until the next node starts.
                    self.current_prop_parent_off = None;
                    None
                },
                Ok(Some(_)) => None,
                _ => return None,
            };

            self.offset = offset.0;
            if ret.is_some() {
                return ret;
            }
        }
    }
}

impl<'a> Iterator for DevTreeIter<'a> {
    type Item = DevTreeItem<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.next_devtree_item()
    }
}

/// An interator over [`DevTreeNode`] objects in the [`DevTree`]
#[derive(Clone)]
pub struct DevTreeNodeIter<'a>(DevTreeIter<'a>);

impl<'a> DevTreeNodeIter<'a> {
    pub(crate) fn new(fdt: &'a DevTree) -> Self {
        Self(DevTreeIter::new(fdt))
    }

    /// See the documentation of [`DevTree::find_node`]
    #[inline]
    pub fn find<F>(&mut self, predicate: F) -> Option<(DevTreeNode<'a>, Self)>
    where
        F: Fn(&DevTreeNode) -> Result<bool, DevTreeError>,
    {
        while let Some(i) = self.next() {
            if let Ok(true) = predicate(&i) {
                return Some((i, self.clone()));
            }
        }
        None
    }
}

impl<'a> Iterator for DevTreeNodeIter<'a> {
    type Item = DevTreeNode<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_node()
    }
}

impl<'a> From<DevTreeIter<'a>> for DevTreeNodeIter<'a> {
    fn from(iter: DevTreeIter<'a>) -> Self {
        Self(iter)
    }
}

/// An interator over [`DevTreeProp`] objects in the [`DevTree`]
#[derive(Clone)]
pub struct DevTreePropIter<'a>(DevTreeIter<'a>);

impl<'a> DevTreePropIter<'a> {
    pub(crate) fn new(fdt: &'a DevTree) -> Self {
        Self(DevTreeIter::new(fdt))
    }

    /// See the documentation of [`DevTree::find_prop`]
    #[inline]
    pub fn find<F>(&mut self, predicate: F) -> Option<(DevTreeProp<'a>, Self)>
    where
        F: Fn(&DevTreeProp) -> Result<bool, DevTreeError>,
    {
        while let Some(i) = self.next() {
            if let Ok(true) = predicate(&i) {
                return Some((i, self.clone()));
            }
        }
        None
    }
}

impl<'a> Iterator for DevTreePropIter<'a> {
    type Item = DevTreeProp<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_prop()
    }
}

impl<'a> From<DevTreeIter<'a>> for DevTreePropIter<'a> {
    fn from(iter: DevTreeIter<'a>) -> Self {
        Self(iter)
    }
}

/// An interator over [`DevTreeProp`] objects on a single node within the [`DevTree`]
#[derive(Clone)]
pub struct DevTreeNodePropIter<'a>(DevTreeIter<'a>);

impl<'a> DevTreeNodePropIter<'a> {
    pub(crate) fn new(node: &'a DevTreeNode) -> Self {
        Self(node.parse_iter.clone())
    }

    /// See the documentation of [`DevTree::find_prop`]
    #[inline]
    pub fn find<F>(&mut self, predicate: F) -> Option<(DevTreeProp<'a>, Self)>
    where
        F: Fn(&DevTreeProp) -> bool,
    {
        while let Some(i) = self.next() {
            if predicate(&i) {
                return Some((i, self.clone()));
            }
        }
        None
    }
}

impl<'a> Iterator for DevTreeNodePropIter<'a> {
    type Item = DevTreeProp<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_node_prop()
    }
}

impl<'a> From<DevTreeIter<'a>> for DevTreeNodePropIter<'a> {
    fn from(iter: DevTreeIter<'a>) -> Self {
        Self(iter)
    }
}


struct ParsedBeginNode<'a> {
    name: &'a [u8],
}

struct ParsedProp<'a> {
    prop_buf: &'a [u8],
    name_offset: AssociatedOffset<'a>,
}

enum ParsedTok<'a> {
    BeginNode(ParsedBeginNode<'a>),
    EndNode,
    Prop(ParsedProp<'a>),
    Nop,
}

#[inline]
unsafe fn nread_bstring0(buf: &[u8], pos: usize, len: usize) -> crate::buf_util::SliceReadResult<&[u8]> {
    let end = core::cmp::min(len + pos, buf.len());
    for i in pos..end {
        if buf[i] == 0 {
            return Ok(&buf[pos..i]);
        }
    }
    Err(crate::buf_util::SliceReadError::UnexpectedEndOfInput)
}

unsafe fn next_devtree_token<'a>(buf: &'a [u8], off: &mut AssociatedOffset<'a>) -> Result<Option<ParsedTok<'a>>, DevTreeError> {
    // These are guarunteed.
    // We only produce associated offsets that are aligned to 32 bits and within the buffer.
    assert!(buf.as_ptr().add(off.0) as usize % size_of::<u32>() == 0);
    assert!(buf.len() > (off.0 + size_of::<u32>()));

    let fdt_tok_val = buf.unsafe_read_be_u32(off.0)?;
    off.0 += size_of::<u32>();

    match FromPrimitive::from_u32(fdt_tok_val) {
        Some(FdtTok::BeginNode) => {
            // Read the name (or return an error if the device tree is incorrectly formatted).
            //let name = buf.nread_bstring0(off.0, spec::MAX_NODE_NAME_LEN - 1)?;
            let name = nread_bstring0(buf, off.0, spec::MAX_NODE_NAME_LEN - 1)?;

            // Move to the end of name (adding null byte).
            off.0 += name.len() + 1;
            // Per spec - align back to u32.
            off.0 += buf.as_ptr().add(off.0).align_offset(size_of::<u32>());

            Ok(Some(ParsedTok::BeginNode(ParsedBeginNode {
                name,
            })))
        }
        Some(FdtTok::Prop) => {
            // Re-interpret the data as a fdt_header
            let header = core::mem::transmute::<&u8, &fdt_prop_header>(&buf[off.0]);
            // Get length from header
            let prop_len = u32::from((*header).len) as usize;

            // Move past the header to the data;
            off.0 += size_of::<fdt_prop_header>();
            // Create a slice using the offset
            let prop_buf = &buf[off.0..off.0 + prop_len];
            // Move the offset past the prop data.
            off.0 += prop_buf.len();
            // Align back to u32.
            off.0 += buf.as_ptr().add(off.0).align_offset(size_of::<u32>());

            let name_offset = u32::from(header.nameoff) as usize;
            if name_offset > buf.len() {
                return Err(DevTreeError::ParseError);
            }
            let name_offset = AssociatedOffset::new(name_offset, buf);

            Ok(Some(ParsedTok::Prop(ParsedProp {
                name_offset,
                prop_buf,
            })))
        }
        Some(FdtTok::EndNode) => {
            Ok(Some(ParsedTok::EndNode))
        }
        Some(FdtTok::Nop) => {
            Ok(Some(ParsedTok::Nop))
        }
        Some(FdtTok::End) => Ok(None),
        None => {
            // Invalid token
            Err(DevTreeError::ParseError)
        }
    }
}
