use core::str::from_utf8;

use super::iters::{DevTreeIndexIter, DevTreeIndexNodePropIter, DevTreeIndexNodeSiblingIter};
use super::tree::{DTINode, DevTreeIndex};
use crate::error::DevTreeError;

#[derive(Clone)]
pub struct DevTreeIndexNode<'a, 'i: 'a, 'dt: 'i> {
    index: &'a DevTreeIndex<'i, 'dt>,
    pub(super) node: &'a DTINode<'i, 'dt>,
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
}
