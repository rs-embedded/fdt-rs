extern crate fdt_rs;

use fdt_rs::base::DevTree;
use fdt_rs::error::{DevTreeError, Result};
use fdt_rs::index::DevTreeIndex;
use fdt_rs::prelude::*;

/// Fallible Basic Iterator
///
/// A simple wrapper around a normal iterator which will return Ok(Option<I::Item>)
struct FBI<I: Iterator>(pub I);
impl<I> FallibleIterator for FBI<I>
where
    I: Iterator,
{
    type Item = I::Item;
    type Error = DevTreeError;

    fn next(&mut self) -> Result<Option<I::Item>> {
        Ok(self.0.next())
    }
}

#[repr(align(4))]
struct _Wrapper<T>(T);
pub const FDT: &[u8] = &_Wrapper(*include_bytes!("../tests/riscv64-virt.dtb")).0;
static DFS_NODES: &[&str] = &[
    "", // Root
    "flash@20000000",
    "rtc@101000",
    "chosen",
    "uart@10000000",
    "poweroff",
    "reboot",
    "test@100000",
    "virtio_mmio@10008000",
    "virtio_mmio@10007000",
    "virtio_mmio@10006000",
    "virtio_mmio@10005000",
    "virtio_mmio@10004000",
    "virtio_mmio@10003000",
    "virtio_mmio@10002000",
    "virtio_mmio@10001000",
    "cpus",
    "cpu-map",
    "cluster0",
    "core0",
    "cpu@0",
    "interrupt-controller",
    "memory@80000000",
    "soc",
    "pci@30000000",
    "interrupt-controller@c000000",
    "clint@2000000",
];

pub struct FdtIndex<'dt> {
    index: DevTreeIndex<'dt, 'dt>,
    _vec: Vec<u8>,
}

fn get_fdt_index<'dt>() -> FdtIndex<'dt> {
    unsafe {
        let devtree = DevTree::new(FDT).unwrap();
        let layout = DevTreeIndex::get_layout(&devtree).unwrap();
        let mut vec = vec![0u8; layout.size() + layout.align()];
        let slice = core::slice::from_raw_parts_mut(vec.as_mut_ptr(), vec.len());
        FdtIndex {
            index: DevTreeIndex::new(devtree, slice).unwrap(),
            _vec: vec,
        }
    }
}

#[test]
fn test_readsize_advice() {
    unsafe {
        let size = DevTree::read_totalsize(FDT).unwrap();
        assert!(size == FDT.len());
        let _blob = DevTree::new(FDT).unwrap();
    }
}

#[test]
fn reserved_entries_iter() {
    unsafe {
        let blob = DevTree::new(FDT).unwrap();
        assert!(blob.reserved_entries().count() == 0);
    }
}

#[test]
fn nodes_iter() {
    unsafe {
        let blob = DevTree::new(FDT).unwrap();
        let iter = blob.nodes();
        let mut pair_iter = iter.clone().zip(FBI(DFS_NODES.iter()));
        while let Some((node, expected)) = pair_iter.next().unwrap() {
            assert_eq!(node.name().unwrap(), *expected);
        }
        assert!(iter.count().unwrap() == DFS_NODES.len());
    }
}

// Test that comparision of props works as expected.
#[test]
fn verify_prop_comparisions() {
    unsafe {
        let blob = DevTree::new(FDT).unwrap();

        let props_iter_1 = blob.props();
        let props_iter_2 = blob.props();

        let mut pair_iter = props_iter_1.zip(props_iter_2);
        while let Some((prop_1, prop_2)) = pair_iter.next().unwrap() {
            assert!(prop_1 == prop_2);
        }

        let mut props_iter_1 = blob.props();
        let props_iter_2 = blob.props();

        // Mess up the lock step iteration, every prop should be different
        let _ = props_iter_1.next().unwrap();

        let mut pair_iter = props_iter_1.zip(props_iter_2);
        while let Some((prop_1, prop_2)) = pair_iter.next().unwrap() {
            assert!(prop_1 != prop_2);
        }
    }
}

// Test that comparision of props works as expected.
#[test]
fn get_prop_node() {
    unsafe {
        let blob = DevTree::new(FDT).unwrap();

        let prop = blob.props().next().unwrap().unwrap();
        let node = prop.node();

        assert_eq!(node.name().unwrap(), "");
    }
}

// Test that comparision of props works as expected.
#[test]
fn get_memory_prop_node() {
    unsafe {
        let blob = DevTree::new(FDT).unwrap();

        let mem_prop = blob
            .props()
            .find(|p| Ok(p.name()? == "device_type" && p.str()? == "memory"))
            .unwrap()
            .expect("Unable to find memory node.");
        let mem_node = mem_prop.node();

        let _ = mem_node
            .props()
            .find(|p| Ok(p.name()? == "reg"))
            .unwrap()
            .expect("Device tree memory node missing 'reg' prop.");
    }
}

// Test that comparision of nodes works as expected.
#[test]
fn verify_node_comparisions() {
    unsafe {
        let blob = DevTree::new(FDT).unwrap();

        let nodes_iter_1 = blob.nodes();
        let nodes_iter_2 = blob.nodes();

        let mut pair_iter = nodes_iter_1.zip(nodes_iter_2);
        while let Some((node_1, node_2)) = pair_iter.next().unwrap() {
            assert!(node_1 == node_2);
        }

        let mut nodes_iter_1 = blob.nodes();
        let nodes_iter_2 = blob.nodes();

        // Mess up the lock step iteration, every node should be different
        let _ = nodes_iter_1.next().unwrap();

        let mut pair_iter = nodes_iter_1.zip(nodes_iter_2);
        while let Some((node_1, node_2)) = pair_iter.next().unwrap() {
            assert!(node_1 != node_2);
        }
    }
}

#[test]
fn node_prop_iter() {
    unsafe {
        let blob = DevTree::new(FDT).unwrap();
        let mut node_iter = blob.nodes();
        while let Some(node) = node_iter.next().unwrap() {
            let mut prop_iter = node.props();
            while let Some(prop) = prop_iter.next().unwrap() {
                if prop.length() > 0 {
                    if let Ok(i) = prop.iter_str().count() {
                        if i == 0 {
                            continue;
                        }
                        assert!(i < 64);

                        let mut iter = prop.iter_str();
                        while let Some(s) = iter.next().unwrap() {
                            let _ = s;
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn next_compatible_finds_initial_node() {
    unsafe {
        let fdt = DevTree::new(FDT).unwrap();
        let node = fdt
            .compatible_nodes("riscv-virtio")
            .next()
            .unwrap()
            .unwrap();
        assert!(node.name().unwrap() == ""); // Root node has no "name"
    }
}

#[test]
fn next_compatible_finds_final_node() {
    unsafe {
        let fdt = DevTree::new(FDT).unwrap();
        let node = fdt
            .compatible_nodes("riscv,clint0")
            .next()
            .unwrap()
            .unwrap();
        assert!(node.name().unwrap() == "clint@2000000");
    }
}

#[test]
fn find_all_compatible() {
    unsafe {
        let devtree = DevTree::new(FDT).unwrap();
        let compat = "virtio,mmio";
        let exp = "virtio_mmio@1000";
        let mut count = 0;
        let exp_count = 8;

        if let Some(mut cur) = devtree.root().unwrap() {
            while let Some(node) = cur.find_next_compatible_node(compat).unwrap() {
                count += 1;
                // Verify the prefix matches.
                // (ascii doesn't have startswith)
                assert!(node.name().unwrap()[0..exp.len()] == *exp);
                cur = node;
                assert!(count <= exp_count);
            }
        }
        assert!(count == exp_count);
    }
}

pub mod index_tests {
    use super::*;

    // Test that we can create an index from a valid device tree
    #[test]
    fn create_index() {
        let _ = get_fdt_index();
    }

    // Test that our index get_layout returns a usable layout size.
    #[test]
    fn create_sized_index() {
        unsafe {
            let devtree = DevTree::new(FDT).unwrap();
            let layout = DevTreeIndex::get_layout(&devtree).unwrap();
            let mut vec = vec![0u8; layout.size() + layout.align()];
            DevTreeIndex::new(devtree, vec.as_mut_slice()).unwrap();
        }
    }

    // Test that an invalid buffer size results in NotEnoughMemory on index allocation.
    #[test]
    fn expect_create_index_layout_fails_with_invalid_layout() {
        unsafe {
            let devtree = DevTree::new(FDT).unwrap();
            let layout = DevTreeIndex::get_layout(&devtree).unwrap();
            let mut vec = vec![0u8; layout.size() - 1];
            DevTreeIndex::new(devtree, vec.as_mut_slice()).expect_err("Expected failure.");
        }
    }

    // Test DFS iteration using a DevTreeIndex.
    #[test]
    fn dfs_iteration() {
        let idx = get_fdt_index();
        test_index_dfs(&idx);
    }

    // Test that iteration over children works as expected.
    #[test]
    fn verify_root_children_iteration() {
        let idx = get_fdt_index();
        let root = idx.index.root();
        assert_eq!(root.children().count(), 18);
    }

    // Test that comparision of nodes works as expected.
    #[test]
    fn verify_root_children_comparisions() {
        let idx = get_fdt_index();
        let root = idx.index.root();

        let mut prev = None;
        for child in root.children() {
            assert!(root.is_parent_of(&child));
            if let Some(prev_child) = &prev {
                assert!(child.is_sibling_of(prev_child));
                assert!(prev_child != &child);
            }
            prev = Some(child);
        }
    }

    #[test]
    fn get_memory_prop_node() {
        let idx = get_fdt_index();

        let mem_prop = idx
            .index
            .props()
            .find(|p| p.name() == Ok("device_type") && p.str() == Ok("memory"))
            .expect("Unable to find memory node.");
        let mem_node = mem_prop.node();

        let _ = mem_node
            .props()
            .find(|p| p.name() == Ok("reg"))
            .expect("Device tree memory node missing 'reg' prop.");
    }

    // Test iteration over the root nodes props.
    #[test]
    fn root_prop_iteration() {
        let idx = get_fdt_index();
        test_root_prop_iteration(&idx);
    }

    #[test]
    fn test_prop_iteration_() {
        test_prop_iteration(&get_fdt_index());
    }

    pub fn test_prop_iteration<'dt>(idx: &FdtIndex<'dt>) {
        let iter = idx.index.props();
        assert_eq!(iter.count(), 105);
    }

    pub fn test_root_prop_iteration<'dt>(idx: &FdtIndex<'dt>) {
        let root_props = &["#address-cells", "#size-cells", "compatible", "model"];

        let iter = idx.index.root().props();
        for (node, expected) in iter.clone().zip(root_props) {
            assert_eq!(node.name().unwrap(), *expected);
        }
        assert!(iter.count() == root_props.len());
    }

    pub fn test_index_dfs<'dt>(idx: &FdtIndex<'dt>) {
        let iter = idx.index.nodes();
        for (node, expected) in iter.clone().zip(DFS_NODES) {
            assert_eq!(node.name().unwrap(), *expected);
        }
        assert_eq!(iter.count(), DFS_NODES.len());
    }
}
