extern crate proc_macro;
extern crate proc_macro2;
extern crate syn;

extern crate quote;

use proc_macro2::TokenStream;

use syn::parse::Error;
use syn::spanned::Spanned;
use syn::{parse_macro_input, DeriveInput};

use quote::quote;

mod field_info;
mod struct_info;
mod util;

/// `TypedBuilder` is not a real type - deriving it will generate a `::builder()` method on your
/// struct that will return a compile-time checked builder. Set the fields using setters with the
/// same name as the struct's fields that accept `Into` types for the type of the field, and call
/// `.build()` when you are done to create your object.
///
/// Trying to set the same fields twice will generate a compile-time error. Trying to build without
/// setting one of the fields will also generate a compile-time error - unless that field is marked
/// as `#[builder(default)]`, in which case the `::default()` value of it's type will be picked. If
/// you want to set a different default, use `#[builder(default=...)]`.
///
/// # Examples
///
/// ```
/// #[macro_use]
/// extern crate typed_builder;
///
/// #[derive(PartialEq, TypedBuilder)]
/// struct Foo {
///     // Mandatory Field:
///     x: i32,
///
///     // #[default] without parameter - use the type's default
///     #[builder(default)]
///     y: Option<i32>,
///
///     // Or you can set the default
///     #[builder(default=20)]
///     z: i32,
///
///     // If the default cannot be parsed, you must encode it as a string.
///     // This also allows you to refer to the values of earlier-declared fields.
///     #[builder(default_code="vec![z as u32, 40]")]
///     w: Vec<u32>,
/// }
///
/// fn main() {
///     assert!(
///         Foo::builder().x(1).y(2).z(3).w(vec![4, 5]).build()
///         == Foo { x: 1, y: Some(2), z: 3, w: vec![4, 5] });
///
///     // Change the order of construction:
///     assert!(
///         Foo::builder().z(1).x(2).w(vec![4, 5]).y(3).build()
///         == Foo { x: 2, y: Some(3), z: 1, w: vec![4, 5] });
///
///     // Optional fields are optional:
///     assert!(
///         Foo::builder().x(1).build()
///         == Foo { x: 1, y: None, z: 20, w: vec![20, 40] });
///
///     // This will not compile - because we did not set x:
///     // Foo::builder().build();
///
///     // This will not compile - because we set y twice:
///     // Foo::builder().x(1).y(2).y(3);
/// }
/// ```
///
/// # Customisation with attributes
///
/// In addition to putting `#[derive(TypedBuilder)]` on a type, you can specify a `#[builder(…)]`
/// attribute on the type, and on any fields in it.
///
/// On the **type**, the following values are permitted:
///
/// - `name = FooBuilder`: customise the name of the builder type. By default, the builder type
///   will use the type’s name plus “Builder”, e.g. `FooBuilder` for type `Foo`. (Note this is
///   `name = FooBuilder` and not `name = "FooBuilder"`.)
///
/// - `doc`: enable documentation of the builder type. By default, the builder type is given
///   `#[doc(hidden)]`, so that the `builder()` method will show `FooBuilder` as its return type,
///   but it won’t be a link. If you turn this on, the builder type and its `build` method will get
///   sane defaults. The field methods on the builder will be undocumented by default.
///
/// - `builder_method_doc = "…"` replaces the default documentation that will be generated for the
///   `builder()` method of the type for which the builder is being generated.
///
/// - `builder_type_doc = "…"` replaces the default documentation that will be generated for the
///   builder type. Setting this implies `doc`.
///
/// - `build_method_doc = "…"` replaces the default documentation that will be generated for the
///   `build()` method of the builder type. Setting this implies `doc`.
///
/// On each **field**, the following values are permitted:
///
/// - `default`: make the field optional, defaulting to `Default::default()`. This requires that
///   the field type implement `Default`. Mutually exclusive with any other form of default.
///
/// - `default = …`: make the field optional, defaulting to the expression `…`. This can be
///   anything that will parse in an attribute, e.g. a string or a number. Although some
///   non-literal expressions will successfully parse (e.g. `Some(foo)`), it is recommended for
///   stylistic consistency across the Rust ecosystem that anything that is not a literal use
///   `default_code` instead. Mutually exclusive with any other form of default.
///
/// - `default_code = "…"`: make the field optional, defaulting to the expression `…`.
///   This must be used when the expression will not parse in an attribute with `default = …`.
///   Mutually exclusive with any other form of default. You can refer by name to the values
///   determined for fields that are defined earlier in the type.
///
/// - `doc = "…"`: sets the documentation for the field’s method on the builder type. This will be
///   of no value unless you enable docs for the builder type with `#[builder(doc)]` or similar on
///   the type.
///
/// - `skip`: do not define a method on the builder for this field. This requires that a default
///   be set.
#[proc_macro_derive(TypedBuilder, attributes(builder))]
pub fn derive_typed_builder(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_my_derive(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn impl_my_derive(ast: &syn::DeriveInput) -> Result<TokenStream, Error> {
    let data = match &ast.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => {
                let struct_info = struct_info::StructInfo::new(&ast, fields.named.iter())?;
                let builder_creation = struct_info.builder_creation_impl()?;
                let conversion_helper = struct_info.conversion_helper_impl()?;
                let fields = struct_info
                    .included_fields()
                    .map(|f| struct_info.field_impl(f).unwrap());
                let fields = quote!(#(#fields)*).into_iter();
                let build_method = struct_info.build_method_impl();

                quote! {
                    #builder_creation
                    #conversion_helper
                    #( #fields )*
                    #build_method
                }
            }
            syn::Fields::Unnamed(_) => {
                return Err(Error::new(
                    ast.span(),
                    "TypedBuilder is not supported for tuple structs",
                ))
            }
            syn::Fields::Unit => {
                return Err(Error::new(
                    ast.span(),
                    "TypedBuilder is not supported for unit structs",
                ))
            }
        },
        syn::Data::Enum(_) => {
            return Err(Error::new(
                ast.span(),
                "TypedBuilder is not supported for enums",
            ))
        }
        syn::Data::Union(_) => {
            return Err(Error::new(
                ast.span(),
                "TypedBuilder is not supported for unions",
            ))
        }
    };
    Ok(data)
}

// It’d be nice for the compilation tests to live in tests/ with the rest, but short of pulling in
// some other test runner for that purpose (e.g. compiletest_rs), rustdoc compile_fail in this
// crate is all we can use.

#[doc(hidden)]
/// When a property is skipped, you can’t set it:
/// (“method `y` not found for this”)
///
/// ```compile_fail
/// #[macro_use] extern crate typed_builder;
/// #[derive(PartialEq, TypedBuilder)]
/// struct Foo {
///     #[builder(skip, default)]
///     y: i8,
/// }
///
/// let _ = Foo::builder().y(1i8).build();
/// ```
///
/// But you can build a record:
///
/// ```
/// #[macro_use] extern crate typed_builder;
/// #[derive(PartialEq, TypedBuilder)]
/// struct Foo {
///     #[builder(skip, default)]
///     y: i8,
/// }
///
/// let _ = Foo::builder().build();
/// ```
///
/// `skip` without `default` or `default_code` is disallowed:
/// (“error: #[builder(skip)] must be accompanied by default or default_code”)
///
/// ```compile_fail
/// #[macro_use] extern crate typed_builder;
/// #[derive(PartialEq, TypedBuilder)]
/// struct Foo {
///     #[builder(skip)]
///     y: i8,
/// }
/// ```
fn _compile_fail_tests() {}
