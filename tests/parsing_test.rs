extern crate fdt_rs;

use core::mem::size_of;
use fdt_rs::DevTree;
use fdt_rs::Str;

#[repr(align(4))]
struct _Wrapper<T>(T);
pub const FDT: &[u8] = &_Wrapper(*include_bytes!("riscv64-virt.dtb")).0;

#[macro_use]
extern crate cfg_if;
cfg_if! {
    if #[cfg(feature = "ascii")] {
        fn str_from_static(string: &str) -> &Str {
            Str::from_ascii(string).unwrap()
        }
    } else {
        fn str_from_static(string: &str) -> &Str {
            string
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
        for node in blob.nodes() {
            println!("{}", node.name().unwrap());
        }
        assert!(blob.nodes().count() == 27);
    }
}

#[test]
fn node_prop_iter() {
    unsafe {
        let blob = DevTree::new(FDT).unwrap();
        for node in blob.nodes() {
            for prop in node.props() {
                if prop.length() == size_of::<u32>() {
                    //println!("\t\t0x{:x}", prop.get_u32(0).unwrap());
                }
                if prop.length() > 0 {
                    if let Ok(i) = prop.get_str_count() {
                        println!("{}", i);
                        if i == 0 {
                            continue;
                        }
                        assert!(i < 64);
                        let mut vec: &mut [Option<&Str>] = &mut [None; 64];
                        if prop.get_strlist(&mut vec).is_err() {
                            continue;
                        }

                        let mut iter = vec.iter();

                        while let Some(Some(s)) = iter.next() {
                            print!("\t\t{} ", s);
                        }
                        println!();
                    }
                }
            }
        }
    }
}

#[test]
fn find_first_compatible_works_on_initial_node() {
    unsafe {
        let fdt = DevTree::new(FDT).unwrap();
        let node = fdt
            .find_first_compatible_node(str_from_static("riscv-virtio"))
            .unwrap();
        assert!(node.name().unwrap() == ""); // Root node has no "name"
    }
}

#[test]
fn find_first_compatible_works_on_final_node() {
    unsafe {
        let fdt = DevTree::new(FDT).unwrap();
        let node = fdt
            .find_first_compatible_node(str_from_static("riscv,clint0"))
            .unwrap();
        assert!(node.name().unwrap() == "clint@2000000");
    }
}
#[test]
fn find_all_compatible() {
    unsafe {
        let devtree = DevTree::new(FDT).unwrap();
        let compat = str_from_static("virtio,mmio");
        let exp = str_from_static("virtio_mmio@1000");
        let mut count = 0;
        let exp_count = 8;

        if let Some(mut cur) = devtree.root() {
            while let Some(node) = cur.find_next_compatible_node(compat) {
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
