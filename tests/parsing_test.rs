extern crate fdt_rs;

use core::mem::size_of;
use fdt_rs::Str;

use std::fs::{metadata, File};
use std::io::Read;
use std::path::PathBuf;

use fdt_rs::DevTree;

unsafe fn read_dtb() -> (Vec<u8>, &'static mut [u8]) {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("tests/riscv64-virt.dtb");
    println!("{}", d.display());

    let mut file = File::open(&d).unwrap();
    let len = metadata(&d).unwrap().len() as usize;
    let mut vec: Vec<u8> = vec![0u8; len + size_of::<u32>() * 2];

    let (_, buf, _) = vec.align_to_mut::<u32>();
    let p = buf.as_mut_ptr();
    let mut buf = std::slice::from_raw_parts_mut(p as *mut u8, len);

    file.read_exact(&mut buf).unwrap();
    (vec, buf)
}

#[test]
fn test_readsize_advice() {
    unsafe {
        let (vec, buf) = read_dtb();
        let size = DevTree::read_totalsize(&buf).unwrap();
        let buf = &buf[..size];
        let _blob = DevTree::new(buf).unwrap();
        std::mem::drop(vec);
    }
}

#[test]
fn reserved_entries_iter() {
    unsafe {
        let (vec, buf) = read_dtb();
        let blob = DevTree::new(buf).unwrap();
        assert!(blob.reserved_entries().count() == 0);
        std::mem::drop(vec);
    }
}

#[test]
fn nodes_iter() {
    unsafe {
        let (vec, buf) = read_dtb();
        let blob = DevTree::new(buf).unwrap();
        for node in blob.nodes() {
            println!("{}", node.name().unwrap());
        }
        assert!(blob.nodes().count() == 27);
        std::mem::drop(vec);
    }
}

#[test]
fn node_prop_iter() {
    unsafe {
        let (vec, buf) = read_dtb();
        let blob = DevTree::new(buf).unwrap();
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
        // Wait until the end to drop in since we alias the address.
        std::mem::drop(vec);
    }
}

#[test]
fn find_first_compatible() {
    use fdt_rs::DevTreeItem;
    unsafe {
        let (vec, buf) = read_dtb();
        let blob = DevTree::new(buf).unwrap();
        // TODO
        assert!(blob.find(|item| 
            match item {
                DevTreeItem::Prop(p) => p.name().unwrap() == "compatible",
                _ => false,
            }
        ).is_some());
    }
}
