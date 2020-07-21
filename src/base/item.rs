use crate::prelude::*;

use crate::base::{DevTreeNode, DevTreeProp};

/// An enum which contains either a [`DevTreeNode`] or a [`DevTreeProp`]
#[derive(Clone)]
pub enum DevTreeItem<'a, 'dt: 'a> {
    Node(DevTreeNode<'a, 'dt>),
    Prop(DevTreeProp<'a, 'dt>),
}

impl<'a, 'dt: 'a> UnwrappableDevTreeItem<'dt> for DevTreeItem<'a, 'dt> {
    type TreeNode = DevTreeNode<'a, 'dt>;
    type TreeProp = DevTreeProp<'a, 'dt>;

    #[inline]
    fn node(self) -> Option<Self::TreeNode> {
        match self {
            DevTreeItem::Node(node) => Some(node),
            _ => None,
        }
    }

    #[inline]
    fn prop(self) -> Option<Self::TreeProp> {
        match self {
            DevTreeItem::Prop(prop) => Some(prop),
            _ => None,
        }
    }
}
