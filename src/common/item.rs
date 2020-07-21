use crate::prelude::*;

pub trait UnwrappableDevTreeItem<'dt> {
    type TreeProp: PropReader<'dt>;
    // TODO lands this should be defined to Self::TreeProp::NodeType.
    // feature(associated_type_defaults)
    // https://github.com/rust-lang/rust/issues/29661
    type TreeNode;

    fn node(self) -> Option<Self::TreeNode>;
    fn prop(self) -> Option<Self::TreeProp>;
}
