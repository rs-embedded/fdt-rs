# fdt-rs

A flattened device tree parser for embedded (low memory) no-std environments

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies.fdt-rs]
version = "0.1"
```

and this to your crate root:

```rust
extern crate fdt_rs;
```

## Features

This crate can be used without the standard library (`#![no_std]`) by disabling
the default `std` feature. Use this in `Cargo.toml`:

```toml
[dependencies.fdt-rs]
version = "0.1"
default-features = false
# features = ["ascii"]    # <--- Uncomment if you wish to use the ascii crate for str's
```

The `"ascii"` feature will configure the `Str` type returned by string accessor
methods to be of type `AsciiStr` provided by the [ascii crate](https://docs.rs/ascii/1.0.0/ascii/).
Without this feature enabled, `str` references will be returned.

## Example

```
extern crate fdt_rs;

// Initialize the devtree using raw an &[u8] array
let devtree = unsafe {

    // Get the actual size of the device tree after reading its header.
    let size = DevTree::read_totalsize(buf).unwrap();
    let buf = buf[..size];

    // Create the device tree handle
    DevTree::new(buf).unwrap()
}

// Print the name of the UART compatible node
if let Some((compatible_prop, _)) = devtree.find_prop(
        |prop|
            (prop.name()? == "compatible") && (p.get_str(0)? == "ns16550a")) {

    println!(compatible_prop.parent().name()?);
}
```
