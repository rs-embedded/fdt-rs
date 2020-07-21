use crate::prelude::*;

use crate::base::parse::ParsedProp;
use crate::base::DevTree;

use super::tree::{DTINode, DTIProp, DevTreeIndex};
use super::DevTreeIndexNode;

/// A wrapper around a device tree property within a [`DevTreeIndex`].
///
/// Most desired methods are available through the [`PropReader`] trait.
#[derive(Clone)]
pub struct DevTreeIndexProp<'a, 'i: 'a, 'dt: 'i> {
    pub index: &'a DevTreeIndex<'i, 'dt>,
    node: &'a DTINode<'i, 'dt>,
    prop: &'a DTIProp<'dt>,
}

impl<'r, 'a: 'r, 'i: 'a, 'dt: 'i> DevTreeIndexProp<'a, 'i, 'dt> {
    pub(super) fn new(
        index: &'a DevTreeIndex<'i, 'dt>,
        node: &'a DTINode<'i, 'dt>,
        prop: &'a DTIProp<'dt>,
    ) -> Self {
        Self { index, node, prop }
    }
}

impl<'a, 'i: 'a, 'dt: 'i> PropReader<'dt> for DevTreeIndexProp<'a, 'i, 'dt> {
    type NodeType = DevTreeIndexNode<'a, 'i, 'dt>;

    #[inline]
    fn propbuf(&self) -> &'dt [u8] {
        self.prop.propbuf
    }

    #[inline]
    fn nameoff(&self) -> usize {
        self.prop.nameoff
    }

    #[inline]
    fn fdt(&self) -> &DevTree<'dt> {
        &self.index.fdt()
    }

    fn node(&self) -> DevTreeIndexNode<'a, 'i, 'dt> {
        DevTreeIndexNode::new(self.index, self.node)
    }
}

impl<'dt> From<&ParsedProp<'dt>> for DTIProp<'dt> {
    fn from(prop: &ParsedProp<'dt>) -> Self {
        Self {
            propbuf: prop.prop_buf,
            nameoff: prop.name_offset,
        }
    }
}
