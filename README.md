PLACEHOLDER
[![Build Status](https://api.travis-ci.org/idanarye/rust-typed-builder.svg?branch=master)](https://travis-ci.org/idanarye/rust-typed-builder)
[![Latest Version](https://img.shields.io/crates/v/typed-builder.svg)](https://crates.io/crates/typed-builder)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://idanarye.github.io/rust-typed-builder/)

# Rust TypedBuilder

Creates a compile-time verified builder:

```rust
#[macro_use]
extern crate typed_builder;

#[derive(TypedBuilder)]
struct Foo {
    // Mandatory Field:
    x: i32,

    // #[default] without parameter - use the type's default
    #[default]
    y: Option<i32>,

    // Or you can set the default(encoded as string)
    #[default="20"]
    z: i32,
}
```

Build in any order:
```rust
Foo::builder().x(1).y(2).z(3).build();
Foo::builder().z(1).x(2).y(3).build();
```

Omit optional fields(the one marked with `#[default]`):
```rust
Foo::builder().x(1).build()
```

But you can't omit non-optional arguments - or it won't compile:
```rust
Foo::builder().build(); // missing x
Foo::builder().x(1).y(2).y(3); // y is specified twice
```

# Alternatives - and why typed-builder is better

* [derive-builder](https://crates.io/crates/derive_builder) - does all the checks in runtime, returning a `Result` you need to unwrap.
* [safe-builder-derive](https://crates.io/crates/safe-builder-derive) - this one does compile-time checks - by generating a type for each possible state of the builder. Rust can remove the dead code, but your build time will still be exponential. TypedBuilder is encoding the builder's state in the generics arguments - so Rust will only generate the path you actually use.
