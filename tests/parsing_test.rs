extern crate fdt_rs;

use core::mem::size_of;
use fdt_rs::Str;

use std::fs::{metadata, File};
use std::io::Read;
use std::path::PathBuf;

use fdt_rs::DevTree;

#[repr(align(4))] struct _Wrapper<T>(T);
pub const FDT: &[u8] = &_Wrapper(*include_bytes!("riscv64-virt.dtb")).0;

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
            println!("{}", node.name().unwrap());
            for prop in node.props() {
                println!("\t{}", prop.name().unwrap());
                if prop.length() == size_of::<u32>() {
                    //println!("\t\t0x{:x}", prop.get_u32(0).unwrap());
                }
                if prop.length() > 0 {
                    let i = prop.get_str_count();
                    if i.is_ok() {
                        if i.unwrap() == 0 {
                            break;
                        }
                        let mut vec: Vec<Option<&Str>> = vec![None; i.unwrap()];
                        prop.get_strlist(&mut vec).unwrap();

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
fn find_first_compatible() {
    use fdt_rs::DevTreeItem;
    unsafe {
        let blob = DevTree::new(FDT).unwrap();
        // TODO
        assert!(blob.find(|item| 
            Ok(match item {
                DevTreeItem::Prop(p) => p.name()? == "compatible",
                _ => false,
            })
        ).is_some());
    }
}
