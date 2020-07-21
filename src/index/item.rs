use crate::prelude::*;

use super::{DevTreeIndexNode, DevTreeIndexProp};

#[derive(Clone)]
pub enum DevTreeIndexItem<'a, 'i: 'a, 'dt: 'i> {
    Node(DevTreeIndexNode<'a, 'i, 'dt>),
    Prop(DevTreeIndexProp<'a, 'i, 'dt>),
}

impl<'a, 'i: 'a, 'dt: 'i> UnwrappableDevTreeItem<'dt> for DevTreeIndexItem<'a, 'i, 'dt> {
    type TreeNode = DevTreeIndexNode<'a, 'i, 'dt>;
    type TreeProp = DevTreeIndexProp<'a, 'i, 'dt>;
    #[inline]
    fn node(self) -> Option<Self::TreeNode> {
        match self {
            DevTreeIndexItem::Node(node) => Some(node),
            _ => None,
        }
    }

    #[inline]
    fn prop(self) -> Option<Self::TreeProp> {
        match self {
            DevTreeIndexItem::Prop(prop) => Some(prop),
            _ => None,
        }
    }
}
