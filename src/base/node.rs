#[cfg(doc)]
use super::*;

use crate::base::iters::{DevTreeIter, DevTreeNodePropIter};
use crate::error::Result;

/// A handle to a Device Tree Node within the device tree.
#[derive(Clone)]
pub struct DevTreeNode<'a, 'dt: 'a> {
    pub(super) name: Result<&'dt str>,
    pub(super) parse_iter: DevTreeIter<'a, 'dt>,
}

impl<'a, 'dt: 'a> PartialEq for DevTreeNode<'a, 'dt> {
    fn eq(&self, other: &Self) -> bool {
        self.parse_iter == other.parse_iter
    }
}

impl<'a, 'dt: 'a> DevTreeNode<'a, 'dt> {
    /// Returns the name of the `DevTreeNode` (including unit address tag)
    #[inline]
    pub fn name(&'a self) -> Result<&'dt str> {
        self.name
    }

    /// Returns an iterator over this node's children [`DevTreeProp`]
    #[must_use]
    pub fn props(&self) -> DevTreeNodePropIter<'a, 'dt> {
        DevTreeNodePropIter(self.parse_iter.clone())
    }

    /// Returns the next [`DevTreeNode`] object with the provided compatible device tree property
    /// or `None` if none exists.
    ///
    /// # Example
    ///
    /// The following example iterates through all nodes with compatible value "virtio,mmio"
    /// and prints each node's name.
    ///
    /// TODO
    pub fn find_next_compatible_node(&self, string: &str) -> Result<Option<DevTreeNode<'a, 'dt>>> {
        self.parse_iter.clone().next_compatible_node(string)
    }
}
