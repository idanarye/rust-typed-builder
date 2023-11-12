use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{parse::Error, parse_quote, punctuated::Punctuated, Token};

use crate::builder_attr::{IntoSetting, TypeBuilderAttr};
use crate::struct_info::StructInfo;

pub struct EnumInfo<'a> {
    ast: &'a syn::DeriveInput,
    variants: Vec<&'a syn::Variant>,
}

impl<'a> EnumInfo<'a> {
    pub fn new(ast: &'a syn::DeriveInput, variants: impl Iterator<Item = &'a syn::Variant>) -> syn::Result<EnumInfo<'a>> {
        if !ast.generics.params.is_empty() {
            return Err(Error::new_spanned(
                &ast.generics,
                "TypedBuilder is not supported for enum with generics or lifetime",
            ));
        }
        let builder_attr = TypeBuilderAttr::new(&ast.attrs)?;
        if builder_attr.builder_method.name.is_some() {
            return Err(Error::new_spanned(
                ast,
                "TypedBuilder is not supported for enum with builder_method(name=...)",
            ));
        }
        if builder_attr.builder_type.name.is_some() {
            return Err(Error::new_spanned(
                ast,
                "TypedBuilder is not supported for enum with builder_type(name=...)",
            ));
        }
        if !matches!(builder_attr.build_method.into, IntoSetting::NoConversion) {
            return Err(Error::new_spanned(
                ast,
                "TypedBuilder is not supported for enum with build_method(into=...)",
            ));
        }
        Ok(EnumInfo {
            ast,
            variants: variants.collect(),
        })
    }

    fn derive_variant_impl(
        &self,
        variant_name: &syn::Ident,
        variant_attrs: &[syn::Attribute],
        variant_fields: &syn::FieldsNamed,
    ) -> syn::Result<TokenStream> {
        let enum_name = &self.ast.ident;
        let internal_struct_name = format_ident!("{}{}", enum_name, variant_name);
        let internal_struct_ast = &syn::DeriveInput {
            attrs: {
                let mut attrs = self.ast.attrs.clone();
                attrs.extend_from_slice(variant_attrs);
                attrs.push(parse_quote! { #[builder(build_method(into=#enum_name))] });
                attrs
            },
            vis: self.ast.vis.clone(),
            ident: internal_struct_name.clone(),
            generics: syn::Generics::default(),
            ..self.ast.clone() // do not care what data is
        };
        let internal_struct_info = StructInfo::new(internal_struct_ast, variant_fields.named.iter())?;
        let build_method_name = internal_struct_info.build_method_name();
        let builder_method_visibility = internal_struct_info.builder_method_visibility();
        let builder_method_name = internal_struct_info
            .builder_attr
            .builder_method
            .get_name()
            .unwrap_or(syn::Ident::new(&variant_name.to_string().to_case(Case::Snake), Span::call_site()).to_token_stream());
        let internal_struct_doc_and_visibility = if internal_struct_info.builder_attr.doc {
            let doc = format!(
                "
                Internal struct for building [`{enum_name}::{variant_name}`] instances.

                See [`{enum_name}::{builder_method_name}()`] for more info.
                ",
                enum_name = enum_name,
                variant_name = variant_name,
                builder_method_name = builder_method_name,
            );
            let vis = &self.ast.vis;
            quote!(#[doc = #doc] #vis)
        } else {
            quote!(#[doc(hidden)])
        };
        let builder_method_doc = format!(
            "
            Create a builder for building [`{enum_name}::{variant_name}`].
            On the builder, call {setters} to set the values of the fields.
            Finally, call `.{build_method_name}()` to create the instance of `{enum_name}`.
            ",
            enum_name = enum_name,
            variant_name = variant_name,
            setters = internal_struct_info.builder_method_setters_doc(),
            build_method_name = build_method_name,
        );
        let internal_struct_derived_tokenstream = internal_struct_info.derive()?;
        let variant_field_names = variant_fields
            .named
            .iter()
            .map(|f| f.ident.to_token_stream())
            .collect::<Punctuated<_, Token![,]>>();
        let variant_field_name_and_types = variant_fields
            .named
            .iter()
            .map(|f| {
                let (field_name, field_type) = (&f.ident, &f.ty);
                quote! { #field_name: #field_type, }
            })
            .collect::<TokenStream>();
        let internal_builder_name = &internal_struct_info.builder_name;
        let internal_builder_method_name = internal_struct_info.builder_method_name();
        Ok(quote! {
            #[allow(dead_code, non_camel_case_types, missing_docs)]
            #internal_struct_doc_and_visibility
            struct #internal_struct_name { #variant_field_name_and_types }

            #internal_struct_derived_tokenstream

            impl #enum_name {
                #[doc = #builder_method_doc]
                #[allow(dead_code)]
                #builder_method_visibility fn #builder_method_name() -> #internal_builder_name {
                    #internal_struct_name::#internal_builder_method_name()
                }
            }

            #[automatically_derived]
            impl From<#internal_struct_name> for #enum_name {
                fn from(#internal_struct_name { #variant_field_names }: #internal_struct_name) -> Self {
                    Self::#variant_name { #variant_field_names }
                }
            }
        })
    }

    pub fn derive(&self) -> syn::Result<TokenStream> {
        self.variants
            .iter()
            .map(|variant| match &variant.fields {
                syn::Fields::Named(fields) => self.derive_variant_impl(&variant.ident, &variant.attrs, fields),
                syn::Fields::Unnamed(_) => Err(Error::new_spanned(
                    variant,
                    "TypedBuilder is not supported for enum with tuple enum variants",
                )),
                syn::Fields::Unit => Err(Error::new_spanned(
                    variant,
                    "TypedBuilder is not supported for enum with unit enum variants",
                )),
            })
            .collect::<syn::Result<TokenStream>>()
    }
}
