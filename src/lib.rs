extern crate proc_macro;
extern crate syn;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

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
