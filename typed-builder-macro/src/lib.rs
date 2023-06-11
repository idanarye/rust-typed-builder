use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse::Error, parse_macro_input, spanned::Spanned, DeriveInput};

mod field_info;
mod struct_info;
mod util;

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
                let struct_info = struct_info::StructInfo::new(ast, fields.named.iter())?;
                let builder_creation = struct_info.builder_creation_impl()?;
                let fields = struct_info
                    .included_fields()
                    .map(|f| struct_info.field_impl(f))
                    .collect::<Result<TokenStream, _>>()?;
                let required_fields = struct_info
                    .included_fields()
                    .filter(|f| f.builder_attr.default.is_none())
                    .map(|f| struct_info.required_field_impl(f));
                let build_method = struct_info.build_method_impl();

                quote! {
                    #builder_creation
                    #fields
                    #(#required_fields)*
                    #build_method
                }
            }
            syn::Fields::Unnamed(_) => return Err(Error::new(ast.span(), "TypedBuilder is not supported for tuple structs")),
            syn::Fields::Unit => return Err(Error::new(ast.span(), "TypedBuilder is not supported for unit structs")),
        },
        syn::Data::Enum(_) => return Err(Error::new(ast.span(), "TypedBuilder is not supported for enums")),
        syn::Data::Union(_) => return Err(Error::new(ast.span(), "TypedBuilder is not supported for unions")),
    };
    Ok(data)
}
