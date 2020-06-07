//! This module provides a collection of iterative parsers of the buf provided to initialze
//! a [`DevTree`].
use core::mem::size_of;
use core::num::NonZeroUsize;

use num_traits::FromPrimitive;

use super::buf_util::SliceRead;
use super::spec::{fdt_prop_header, fdt_reserve_entry, FdtTok};
use super::{DevTree, DevTreeError, DevTreeItem, DevTreeNode, DevTreeProp};
use super::spec;
use crate::{bytes_as_str};

#[derive(Clone)]
pub struct DevTreeReserveEntryIter<'a> {
    offset: usize,
    fdt: &'a DevTree<'a>,
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
    offset: usize,
    current_node_offset: Option<NonZeroUsize>,
    pub(crate) fdt: &'a DevTree<'a>,
}

impl<'a> DevTreeIter<'a> {
    pub(crate) fn new(fdt: &'a DevTree) -> Self {
        Self {
            offset: fdt.off_dt_struct(),
            current_node_offset: None,
            fdt,
        }
    }

    fn current_node_itr(&self) -> Option<DevTreeIter<'a>> {
        match self.current_node_offset {
            Some(offset) => Some(DevTreeIter {
                fdt: self.fdt,
                current_node_offset: self.current_node_offset,
                offset: offset.get()}) ,
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
        let iter = self.clone();
        let mut iter = DevTreeNodeIter::from(iter);
        if iter.next().is_some() {
            let mut iter = DevTreePropIter::from(iter.0);
            if let Some((compatible_prop, _)) = iter.find(|prop| unsafe {
                Ok((prop.name()? == "compatible") && (prop.get_str(0)? == string))
            }) {
                return Some(compatible_prop.parent());
            }
        }
        None
    }

    // Inlined because higher-order interators may ignore results
    #[inline]
    fn next_devtree_token(&mut self) -> Result<Option<DevTreeItem<'a>>, DevTreeError> {
        unsafe {
            loop {
                // Verify alignment.
                assert!(self.offset % size_of::<u32>() == 0);
                let starting_offset = self.offset;

                // The size will be checked when reads are performed.
                // (We manage this internally so this will never fail.)
                let fdt_tok_val = self.fdt.buf.unsafe_read_be_u32(self.offset)?;
                let fdt_tok = FromPrimitive::from_u32(fdt_tok_val);
                self.offset += size_of::<u32>();

                match fdt_tok {
                    Some(FdtTok::BeginNode) => {
                        // Unchecked is guarunteed safe.
                        // We're accessing past address zero of a device tree.
                        self.current_node_offset = Some(NonZeroUsize::new_unchecked(starting_offset));

                        let name = self.fdt.buf.nread_bstring0(self.offset, spec::MAX_NODE_NAME_LEN - 1)?;

                        // Move to the end of str (adding for null byte).
                        self.offset += name.len() + 1;
                        // Per spec - align back to u32.
                        self.offset += self.fdt.buf.as_ptr().add(self.offset).align_offset(size_of::<u32>());

                        return Ok(Some(DevTreeItem::Node(DevTreeNode {
                            name: bytes_as_str(name).map_err(|e| e.into()),
                            parse_iter: self.clone(),
                        })));
                    }
                    Some(FdtTok::Prop) => {
                        let header: *const fdt_prop_header = self.fdt.ptr_at(self.offset)?;
                        let prop_len = u32::from((*header).len) as usize;

                        self.offset += size_of::<fdt_prop_header>();
                        let propbuf = &self.fdt.buf[self.offset..self.offset + prop_len];
                        self.offset += propbuf.len();

                        // Align back to u32.
                        self.offset += self.fdt.buf.as_ptr().add(self.offset).align_offset(size_of::<u32>());

                        // We saw a property before ever seeing a node.
                        let parent = match self.current_node_itr() {
                            Some(parent) => parent,
                            None => return Err(DevTreeError::ParseError),
                        };

                        return Ok(Some(DevTreeItem::Prop( DevTreeProp {
                            parent_iter: parent, // FIXME
                            nameoff: u32::from((*header).nameoff) as usize,
                            propbuf,
                        })));
                    }
                    Some(FdtTok::EndNode) => {}
                    Some(FdtTok::Nop) => {}
                    Some(FdtTok::End) => return Ok(None),
                    None => {
                        // Invalid token
                        return Err(DevTreeError::ParseError);
                    }
                }
            }
        }
    }
}

impl<'a> Iterator for DevTreeIter<'a> {
    type Item = DevTreeItem<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let res = self.next_devtree_token();
        if let Ok(Some(res)) = res {
            return Some(res);
        }
        None
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
