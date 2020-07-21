use crate::prelude::*;

use super::tree::DTINode;
use super::{DevTreeIndex, DevTreeIndexItem, DevTreeIndexNode, DevTreeIndexProp};
//use crate::error::{Result};

/***********************************/
/***********  Node Siblings  *******/
/***********************************/

#[derive(Clone)]
pub struct DevTreeIndexNodeSiblingIter<'a, 'i: 'a, 'dt: 'i>(DevTreeIndexIter<'a, 'i, 'dt>);

impl<'a, 'i: 'a, 'dt: 'i> From<DevTreeIndexIter<'a, 'i, 'dt>>
    for DevTreeIndexNodeSiblingIter<'a, 'i, 'dt>
{
    fn from(iter: DevTreeIndexIter<'a, 'i, 'dt>) -> Self {
        Self(iter)
    }
}

impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexNodeSiblingIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexNode<'a, 'i, 'dt>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_sibling()
    }
}

/***********************************/
/***********  Items      ***********/
/***********************************/

#[derive(Clone)]
pub struct DevTreeIndexIter<'a, 'i: 'a, 'dt: 'i> {
    pub index: &'a DevTreeIndex<'i, 'dt>,
    node: Option<&'a DTINode<'i, 'dt>>,
    prop_idx: usize,
    initial_node_returned: bool,
}

#[derive(Clone)]
pub struct DevTreeIndexNodeIter<'a, 'i: 'a, 'dt: 'i>(pub DevTreeIndexIter<'a, 'i, 'dt>);
impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexNodeIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexNode<'a, 'i, 'dt>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_node()
    }
}

#[derive(Clone)]
pub struct DevTreeIndexPropIter<'a, 'i: 'a, 'dt: 'i>(pub DevTreeIndexIter<'a, 'i, 'dt>);
impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexPropIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexProp<'a, 'i, 'dt>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_prop()
    }
}

#[derive(Clone)]
pub struct DevTreeIndexNodePropIter<'a, 'i: 'a, 'dt: 'i>(pub DevTreeIndexIter<'a, 'i, 'dt>);
impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexNodePropIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexProp<'a, 'i, 'dt>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_node_prop()
    }
}

#[derive(Clone)]
pub struct DevTreeIndexCompatibleNodeIter<'s, 'a, 'i: 'a, 'dt: 'i> {
    pub iter: DevTreeIndexIter<'a, 'i, 'dt>,
    pub string: &'s str,
}
impl<'s, 'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexCompatibleNodeIter<'s, 'a, 'i, 'dt> {
    type Item = DevTreeIndexNode<'a, 'i, 'dt>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next_compatible_node(self.string)
    }
}

impl<'a, 'i: 'a, 'dt: 'i> DevTreeIndexIter<'a, 'i, 'dt> {
    pub(super) fn new(index: &'a DevTreeIndex<'i, 'dt>) -> Self {
        let mut this = Self::from_node(index.root());
        this.initial_node_returned = false;
        this
    }

    pub fn from_node(node: DevTreeIndexNode<'a, 'i, 'dt>) -> Self {
        Self {
            index: node.index(),
            initial_node_returned: true,
            node: Some(node.node),
            prop_idx: 0,
        }
    }

    pub fn next_sibling(&mut self) -> Option<DevTreeIndexNode<'a, 'i, 'dt>> {
        self.node.map(|node| {
            let cur = DevTreeIndexNode::new(self.index, node);
            self.node = node.next_sibling();
            cur
        })
    }

    pub fn next_devtree_item(&mut self) -> Option<DevTreeIndexItem<'a, 'i, 'dt>> {
        self.node.and_then(|cur_node| {
            // Check if we've returned the first current node.
            if !self.initial_node_returned {
                self.initial_node_returned = true;
                return Some(DevTreeIndexItem::Node(DevTreeIndexNode::new(
                    self.index, cur_node,
                )));
            }

            // First iterate through any properties if there are some available.
            if self.prop_idx < cur_node.num_props {
                // Unsafe OK, we just checked the length of props.
                let prop = unsafe { cur_node.prop_unchecked(self.prop_idx) };

                self.prop_idx += 1;
                return Some(DevTreeIndexItem::Prop(DevTreeIndexProp::new(
                    self.index, &cur_node, prop,
                )));
            }

            self.prop_idx = 0;

            // Otherwise move on to the next node.
            self.node = cur_node.next_dfs();
            self.node
                .map(|cur_node| DevTreeIndexItem::Node(DevTreeIndexNode::new(self.index, cur_node)))
        })
    }

    pub fn next_prop(&mut self) -> Option<DevTreeIndexProp<'a, 'i, 'dt>> {
        loop {
            match self.next() {
                Some(item) => {
                    if let Some(prop) = item.prop() {
                        return Some(prop);
                    }
                    // Continue if a new node.
                    continue;
                }
                _ => return None,
            }
        }
    }

    pub fn next_node(&mut self) -> Option<DevTreeIndexNode<'a, 'i, 'dt>> {
        loop {
            match self.next() {
                Some(item) => {
                    if let Some(node) = item.node() {
                        return Some(node);
                    }
                    // Continue if a new prop.
                    continue;
                }
                _ => return None,
            }
        }
    }

    pub fn next_node_prop(&mut self) -> Option<DevTreeIndexProp<'a, 'i, 'dt>> {
        match self.next() {
            // Return if a new node or an EOF.
            Some(item) => item.prop(),
            _ => None,
        }
    }

    pub fn next_compatible_node(&mut self, string: &str) -> Option<DevTreeIndexNode<'a, 'i, 'dt>> {
        // If there is another node, advance our iterator to that node.
        self.next_node().and_then(|_| {
            // Iterate through all remaining properties in the tree looking for the compatible
            // string.
            while let Some(prop) = self.next_prop() {
                unsafe {
                    if prop.name().ok()? == "compatible" && prop.get_str().ok()? == string {
                        return Some(prop.node());
                    }
                }
            }
            None
        })
    }
}

impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexItem<'a, 'i, 'dt>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_devtree_item()
    }
}
