//! # Typed Builder
//!
//! This crate provides a custom derive for `TypedBuilder`. `TypedBuilder` is not a real type -
//! deriving it will generate a `::builder()` method on your struct that will return a compile-time
//! checked builder. Set the fields using setters with the same name as the struct's fields that
//! accept `Into` types for the type of the field, and call `.build()` when you are done to create
//! your object.
//!
//! Trying to set the same fields twice will generate a compile-time error. Trying to build without
//! setting one of the fields will also generate a compile-time error - unless that field is marked
//! as `#[default]`, in which case the `::default()` value of it's type will be picked. If you want
//! to set a different default, use `#[default="..."]` - note that it has to be encoded in a
//! string, so `1` is `#[default="1"]` and `"hello"` is `#[default="\"hello\""]`.
//!
//! # Examples
//!
//! ```
//! #[macro_use]
//! extern crate typed_builder;
//!
//! #[derive(PartialEq, TypedBuilder)]
//! struct Foo {
//!     // Mandatory Field:
//!     x: i32,
//!
//!     // #[default] without parameter - use the type's default
//!     #[default]
//!     y: Option<i32>,
//!
//!     // Or you can set the default(encoded as string)
//!     #[default="20"]
//!     z: i32,
//! }
//!
//! fn main() {
//!     assert!(
//!         Foo::builder().x(1).y(2).z(3).build()
//!         == Foo { x: 1, y: Some(2), z: 3 });
//!
//!     // Change the order of construction:
//!     assert!(
//!         Foo::builder().z(1).x(2).y(3).build()
//!         == Foo { x: 2, y: Some(3), z: 1 });
//!
//!     // Optional fields are optional:
//!     assert!(
//!         Foo::builder().x(1).build()
//!         == Foo { x: 1, y: None, z: 20 });
//!
//!     // This will not compile - because we did not set x:
//!     // Foo::builder().build();
//!
//!     // This will not compile - because we set y twice:
//!     // Foo::builder().x(1).y(2).y(3);
//! }
//! ```
extern crate proc_macro;
extern crate syn;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

mod util;
mod field_info;
mod struct_info;


#[doc(hidden)]
#[proc_macro_derive(TypedBuilder, attributes(default))]
pub fn derive_typed_builder(input: TokenStream) -> TokenStream {
    let ast = syn::parse_derive_input(&input.to_string()).unwrap();
    impl_my_derive(&ast).parse().unwrap()
}

fn impl_my_derive(ast: &syn::DeriveInput) -> quote::Tokens {

    match ast.body {
        syn::Body::Struct(syn::VariantData::Struct(ref body)) => {
            let struct_info = struct_info::StructInfo::new(&ast, body);
            let builder_creation = struct_info.builder_creation_impl();
            let conversion_helper = struct_info.conversion_helper_impl();
            let fields = struct_info.fields.iter().map(|f| struct_info.field_impl(f));
            let build_method = struct_info.build_method_impl();
            quote!{
                #builder_creation
                #conversion_helper
                #( #fields )*
                #build_method
            }
        },
        syn::Body::Struct(syn::VariantData::Unit) => panic!("SmartBuilder is not supported for unit types"),
        syn::Body::Struct(syn::VariantData::Tuple(_)) => panic!("SmartBuilder is not supported for tuples"),
        syn::Body::Enum(_) => panic!("SmartBuilder is not supported for enums"),
    }
}

