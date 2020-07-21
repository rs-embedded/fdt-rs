use core::alloc::Layout;
use core::marker::PhantomData;
use core::mem::{align_of, size_of};
use core::ptr::null_mut;

use crate::prelude::*;

use super::iters::{
    DevTreeIndexCompatibleNodeIter, DevTreeIndexIter, DevTreeIndexNodeIter, DevTreeIndexPropIter,
};
use super::DevTreeIndexNode;
use crate::base::item::DevTreeItem;
use crate::base::iters::DevTreeIter;
use crate::base::parse::{DevTreeParseIter, ParsedBeginNode, ParsedProp, ParsedTok};
use crate::base::DevTree;
use crate::error::DevTreeError;

unsafe fn aligned_ptr_in<T>(buf: &mut [u8], offset: usize) -> Result<*mut T, DevTreeError> {
    // Get the aligned offset
    let ptr = buf.as_ptr().add(offset);
    let aligned_offset = offset + ptr.align_offset(align_of::<T>());

    let t_slice_ref = buf
        .get_mut(aligned_offset..aligned_offset + size_of::<T>())
        .ok_or(DevTreeError::NotEnoughMemory)?;
    Ok(t_slice_ref.as_mut_ptr() as *mut T)
}

pub(super) struct DTIProp<'dt> {
    pub propbuf: &'dt [u8],
    pub nameoff: usize,
}

#[derive(Debug)]
pub struct DevTreeIndex<'i, 'dt: 'i> {
    fdt: DevTree<'dt>,
    root: *const DTINode<'i, 'dt>,
}

struct DTIBuilder<'i, 'dt: 'i> {
    buf: &'i mut [u8],
    cur_node: *mut DTINode<'i, 'dt>,
    prev_new_node: *mut DTINode<'i, 'dt>,
    front_off: usize,

    // Devtree Props may only occur before child nodes.
    // We'll call this the "node_header".
    in_node_header: bool,
}

pub(super) struct DTINode<'i, 'dt: 'i> {
    parent: *const Self,
    first_child: *const Self,
    // `next` is either
    // 1. the next sibling node
    // 2. the next node in DFS (some higher up node)
    // It is 1 if (*next).parent == self.parent, otherwise it is 2.
    next: *const Self,
    pub(super) name: &'dt [u8],

    // NOTE: We store props like C arrays. Props are a packed array after each node.
    // This is the number of props after this node in memory.
    pub(super) num_props: usize,
    _index: PhantomData<&'i u8>,
}

impl<'i, 'dt: 'i> DTINode<'i, 'dt> {
    pub unsafe fn prop_unchecked(&self, idx: usize) -> &'i DTIProp<'dt> {
        // Get the pointer to the props after ourself.
        let prop_ptr = (self as *const Self).add(1) as *const DTIProp;
        &*prop_ptr.add(idx)
    }

    pub fn first_child(&self) -> Option<&'i DTINode<'i, 'dt>> {
        unsafe { self.first_child.as_ref() }
    }

    pub fn next_dfs(&self) -> Option<&'i DTINode<'i, 'dt>> {
        unsafe { self.first_child().or_else(|| self.next.as_ref()) }
    }

    pub fn next_sibling(&self) -> Option<&'i DTINode<'i, 'dt>> {
        unsafe {
            self.next.as_ref().and_then(|next| {
                if next.parent == self.parent {
                    return Some(next);
                }
                None
            })
        }
    }

    pub fn parent(&self) -> Option<&'i DTINode<'i, 'dt>> {
        unsafe { self.parent.as_ref() }
    }
}

impl<'i, 'dt: 'i> DTIBuilder<'i, 'dt> {
    fn allocate_aligned_ptr<T>(&mut self) -> Result<*mut T, DevTreeError> {
        unsafe {
            let ptr = aligned_ptr_in::<T>(self.buf, self.front_off)?;
            self.front_off = ptr.add(1) as usize - self.buf.as_ptr() as usize;
            Ok(ptr)
        }
    }

    pub fn parsed_node(&mut self, node: &ParsedBeginNode<'dt>) -> Result<(), DevTreeError> {
        unsafe {
            self.in_node_header = true;

            let new_ptr = self.allocate_aligned_ptr::<DTINode>()?;
            let parent = self.cur_node;

            // Write the data
            *new_ptr = DTINode {
                parent,

                // set by the next node we create
                first_child: null_mut(),
                // set by the next node we create
                next: null_mut(),

                name: node.name,
                num_props: 0,
                _index: PhantomData,
            };

            if !parent.is_null() {
                debug_assert!(
                    !self.prev_new_node.is_null(),
                    "cur_node should not have been initialized without also intializing \
                    prev_new_node"
                );

                (*self.prev_new_node).next = new_ptr;
                if !(*parent).next.is_null() {
                    let prev_sibling = (*parent).next as *mut DTINode;
                    (*prev_sibling).next = new_ptr;
                }
                (*parent).next = new_ptr;

                // If this new node is the first node that follows the current one, it is the current's
                // first child.
                if (*parent).first_child.is_null() {
                    (*parent).first_child = new_ptr;
                }
            }

            // Save the new node ptr.
            self.cur_node = new_ptr;
            self.prev_new_node = new_ptr;
        }

        Ok(())
    }

    pub fn parsed_prop(&mut self, prop: &ParsedProp<'dt>) -> Result<(), DevTreeError> {
        if !self.in_node_header {
            return Err(DevTreeError::ParseError);
        }

        unsafe {
            let new_ptr = self.allocate_aligned_ptr::<DTIProp>()?;
            (*self.cur_node).num_props += 1;
            *new_ptr = DTIProp::from(prop);
        }

        Ok(())
    }

    pub fn parsed_end_node(&mut self) -> Result<(), DevTreeError> {
        // There were more EndNode tokens than BeginNode ones.
        if self.cur_node.is_null() {
            return Err(DevTreeError::ParseError);
        }
        // Unsafe is Ok.
        // Lifetime : self.cur_node is a pointer into a buffer with the same lifetime as self
        // Alignment: parsed_node verifies alignment when creating self.cur_node
        // NonNull  : We check that self.cur_node is non-null above
        // Mutability: We cast from a *const to a *mut.
        //             We're the only thread which has access to the buffer at this time, so this
        //             is thread-safe.
        unsafe {
            // Change the current node back to the parent.
            self.cur_node = (*self.cur_node).parent as *mut DTINode;
        }

        // We are no longer in a node header.
        // We are either going to see a new node next or parse another end_node.
        self.in_node_header = false;

        Ok(())
    }
}

impl<'i, 'dt: 'i> DevTreeIndex<'i, 'dt> {
    // Note: Our parsing method is unsafe - particularly due to its use of pointer arithmetic.
    //
    // We decide this is worth it for the following reasons:
    // - It requires no allocator.
    // - It has incredibly low overhead.
    //   - This parsing method only requires a single allocation. (The buffer given as buf)
    //   - This parsing method only requires a single iteration over the FDT.
    // - It is very easy to test in isolation; parsing is entirely enclosed to this module.
    unsafe fn init_builder<'a>(
        buf: &'i mut [u8],
        iter: &mut DevTreeParseIter<'a, 'dt>,
    ) -> Result<DTIBuilder<'i, 'dt>, DevTreeError> {
        let mut builder = DTIBuilder {
            front_off: 0,
            buf,
            cur_node: null_mut(),
            prev_new_node: null_mut(),
            in_node_header: false,
        };

        while let Some(tok) = iter.next()? {
            match tok {
                ParsedTok::BeginNode(node) => {
                    builder.parsed_node(&node)?;
                    return Ok(builder);
                }
                ParsedTok::Nop => continue,
                _ => break,
            }
        }
        Err(DevTreeError::ParseError)
    }

    pub fn get_layout(fdt: &'i DevTree<'dt>) -> Result<Layout, DevTreeError> {
        // Size may require alignment of DTINode.
        let mut size = 0usize;

        // We assert this because it makes size calculations easier.
        // We don't have to worry about re-aligning between props and nodes.
        // If they didn't have the same alignment, we would have to keep track
        // of the last node and re-align depending on the last seen type.
        //
        // E.g. If we saw one node, two props, and then two nodes:
        //
        // size = \
        // align_of::<DTINode> + size_of::<DTINode>
        // + align_of::<DTIProp> + size_of::<DTIProp>
        // + size_of::<DTIProp>
        // + size_of::<DTIProp>
        // + align_of::<DTINode> + size_of::<DTINode>
        // + size_of::<DTINode>
        const_assert_eq!(align_of::<DTINode>(), align_of::<DTIProp>());

        let mut iter = DevTreeIter::new(fdt);
        while let Some(item) = iter.next()? {
            match item {
                DevTreeItem::Node(_) => size += size_of::<DTINode>(),
                DevTreeItem::Prop(_) => size += size_of::<DTIProp>(),
            }
        }

        // Unsafe okay.
        // - Size is not likely to be usize::MAX. (There's no way we find that many nodes.)
        // - Align is a result of align_of, so it will be a non-zero power of two
        unsafe {
            Ok(Layout::from_size_align_unchecked(
                size,
                align_of::<DTINode>(),
            ))
        }
    }

    pub fn new(fdt: DevTree<'dt>, buf: &'i mut [u8]) -> Result<Self, DevTreeError> {
        let mut iter = DevTreeParseIter::new(&fdt);

        let mut builder = unsafe { Self::init_builder(buf, &mut iter) }?;

        let this = Self {
            fdt,
            root: builder.cur_node,
        };

        // The builder should have setup a root node or returned an Err.
        debug_assert!(!this.root.is_null());

        // The buffer will be split into two parts, front and back:
        //
        // Front will be used as a temporary work section to  build the nodes as we parse them.
        // The back will be used to save completely parsed nodes.
        while let Some(item) = iter.next()? {
            match item {
                ParsedTok::BeginNode(node) => {
                    builder.parsed_node(&node)?;
                }
                ParsedTok::Prop(prop) => {
                    builder.parsed_prop(&prop)?;
                }
                ParsedTok::EndNode => {
                    builder.parsed_end_node()?;
                }
                ParsedTok::Nop => continue,
            }
        }
        Ok(this)
    }

    pub fn root(&self) -> DevTreeIndexNode<'_, 'i, 'dt> {
        // Unsafe OK. The root node always exits.
        unsafe { DevTreeIndexNode::new(self, &*self.root) }
    }

    pub fn fdt(&self) -> &DevTree<'dt> {
        &self.fdt
    }

    #[must_use]
    pub fn nodes(&self) -> DevTreeIndexNodeIter<'_, 'i, 'dt> {
        DevTreeIndexNodeIter(self.items())
    }

    #[must_use]
    pub fn props(&self) -> DevTreeIndexPropIter<'_, 'i, 'dt> {
        DevTreeIndexPropIter(self.items())
    }

    #[must_use]
    pub fn items(&self) -> DevTreeIndexIter<'_, 'i, 'dt> {
        DevTreeIndexIter::new(self)
    }

    pub fn compatible_nodes<'a, 's>(
        &'a self,
        string: &'s str,
    ) -> DevTreeIndexCompatibleNodeIter<'s, 'a, 'i, 'dt> {
        DevTreeIndexCompatibleNodeIter {
            iter: self.items(),
            string,
        }
    }

    #[must_use]
    pub fn buf(&self) -> &'dt [u8] {
        self.fdt.buf()
    }
}
