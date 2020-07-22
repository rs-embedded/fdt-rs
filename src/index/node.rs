use core::str::from_utf8;

use super::iters::{DevTreeIndexIter, DevTreeIndexNodePropIter, DevTreeIndexNodeSiblingIter};
use super::tree::{DTINode, DevTreeIndex};
use crate::error::DevTreeError;

#[derive(Clone)]
pub struct DevTreeIndexNode<'a, 'i: 'a, 'dt: 'i> {
    index: &'a DevTreeIndex<'i, 'dt>,
    pub(super) node: &'a DTINode<'i, 'dt>,
}

impl<'a, 'i: 'a, 'dt: 'i> PartialEq for DevTreeIndexNode<'a, 'i, 'dt> {
    fn eq(&self, other: &Self) -> bool {
        self.index as *const DevTreeIndex == other.index as *const DevTreeIndex
            && self.node as *const DTINode == other.node as *const DTINode
    }
}

impl<'a, 'i: 'a, 'dt: 'i> DevTreeIndexNode<'a, 'i, 'dt> {
    pub(super) fn new(index: &'a DevTreeIndex<'i, 'dt>, node: &'a DTINode<'i, 'dt>) -> Self {
        Self { node, index }
    }

    pub fn index(&self) -> &'a DevTreeIndex<'i, 'dt> {
        self.index
    }

    pub fn name(&self) -> Result<&'dt str, DevTreeError> {
        from_utf8(self.node.name).map_err(DevTreeError::StrError)
    }

    pub fn siblings(&self) -> DevTreeIndexNodeSiblingIter<'a, 'i, 'dt> {
        DevTreeIndexNodeSiblingIter::from(DevTreeIndexIter::from_node(self.clone()))
    }

    pub fn props(&self) -> DevTreeIndexNodePropIter<'a, 'i, 'dt> {
        DevTreeIndexNodePropIter(DevTreeIndexIter::from_node(self.clone()))
    }

    pub fn parent(&self) -> Option<Self> {
        self.node.parent().map(|par| Self::new(self.index, par))
    }

    pub fn children(&self) -> DevTreeIndexNodeSiblingIter<'a, 'i, 'dt> {
        match self.node.first_child() {
            Some(child) => DevTreeIndexNodeSiblingIter::from(DevTreeIndexIter::from_node_include(
                DevTreeIndexNode::new(self.index, child),
            )),
            None => DevTreeIndexNodeSiblingIter::from(DevTreeIndexIter::new_dead_iter(self.index)),
        }
    }

    /// Returns true if `self` is a parent of the other [`DevTreeIndexNode`]
    pub fn is_parent_of(&self, other: &Self) -> bool {
        if let Some(parent) = &other.parent() {
            return parent == self;
        }
        false
    }

    /// Returns true if `self` is a sibling of the other [`DevTreeIndexNode`]
    pub fn is_sibling_of(&self, other: &Self) -> bool {
        other.parent() == self.parent()
    }
}
