extern crate fdt_rs;

use fdt_rs::base::DevTree;
use fdt_rs::error::{DevTreeError, Result};
use fdt_rs::index::DevTreeIndex;
use fdt_rs::prelude::*;

use criterion::{criterion_group, criterion_main, Criterion};

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

#[test]
fn node_prop_iter() {
    unsafe {
        let blob = DevTree::new(FDT).unwrap();
        let mut node_iter = blob.nodes();
        while let Some(node) = node_iter.next().unwrap() {
            let mut prop_iter = node.props();
            while let Some(prop) = prop_iter.next().unwrap() {
                if prop.length() > 0 {
                    if let Ok(i) = prop.get_str_count() {
                        if i == 0 {
                            continue;
                        }
                        assert!(i < 64);
                        let mut vec: &mut [Option<&str>] = &mut [None; 64];
                        if prop.get_strlist(&mut vec).is_err() {
                            continue;
                        }

                        let mut iter = vec.iter();

                        while let Some(Some(s)) = iter.next() {
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

fn test_fdt_dfs<'dt>(idx: &FdtIndex<'dt>) {
    let iter = idx.index.fdt().nodes();
    let mut pair_iter = iter.clone().zip(FBI(DFS_NODES.iter()));
    while let Some((node, expected)) = pair_iter.next().unwrap() {
        assert_eq!(node.name().unwrap(), *expected);
    }
    assert!(iter.count().unwrap() == DFS_NODES.len());
}

fn benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("sample-size-example");

    group
        .significance_level(0.01)
        .sample_size(100)
        .measurement_time(core::time::Duration::new(10, 0));

    let idx = get_fdt_index();

    group.bench_function("Raw DFS", |b| b.iter(|| test_fdt_dfs(&idx)));

    group.bench_function("Index DFS", |b| {
        b.iter(|| index_tests::test_index_dfs(&idx))
    });

    group.bench_function("Index Prop Iter", |b| {
        b.iter(|| index_tests::test_prop_iteration(&idx))
    });

    group.bench_function("Index Root Prop Iter", |b| {
        b.iter(|| index_tests::test_root_prop_iteration(&idx))
    });

    group.finish();
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
