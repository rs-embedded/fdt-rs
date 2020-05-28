# fdt-rs

A flattened device tree parser for embedded no-std environments

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
