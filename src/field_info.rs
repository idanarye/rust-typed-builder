use syn;
use proc_macro2::TokenStream;
use syn::spanned::Spanned;
use syn::parse::Error;
use quote::quote;

use util::{make_identifier, map_only_one, path_to_single_string, ident_to_type};
use builder_attr::BuilderAttr;

#[derive(Debug)]
pub struct FieldInfo<'a> {
    pub ordinal: usize,
    pub name: &'a syn::Ident,
    pub generic_ident: syn::Ident,
    pub ty: &'a syn::Type,
    pub builder_attr: BuilderAttr,
    // pub default: Option<TokenStream>,
}

impl<'a> FieldInfo<'a> {
    pub fn new(ordinal: usize, field: &syn::Field) -> Result<FieldInfo, Error> {
        if let Some(ref name) = field.ident {
            let builder_attr = Self::find_builder_attr(field)?;
            Ok(FieldInfo {
                ordinal: ordinal,
                name: &name,
                generic_ident: make_identifier("genericType", name),
                ty: &field.ty,
                builder_attr: builder_attr,
            })
        } else {
            Err(Error::new(field.span(), "Nameless field in struct"))
        }
    }

    fn find_builder_attr(field: &syn::Field) -> Result<BuilderAttr, Error> {
        Ok(map_only_one(&field.attrs, |attr| {
            if path_to_single_string(&attr.path).as_ref().map(|s| &**s) == Some("builder") {
                Ok(Some(BuilderAttr::new(&attr.tts)?))
            } else {
                Ok(None)
            }
        })?.unwrap_or_else(|| Default::default()))
    }

    pub fn generic_ty_param(&self) -> syn::GenericParam {
        syn::GenericParam::Type(self.generic_ident.clone().into())
    }

    pub fn type_param(&self) -> syn::TypeParam {
        // syn::TypeParam::Type(self.generic_ident.clone().into())
        self.generic_ident.clone().into()
    }

    pub fn type_ident(&self) -> syn::Type {
        ident_to_type(self.generic_ident.clone())
    }

    pub fn tuplized_type_ty_param(&self) -> syn::Type {
        let mut types = syn::punctuated::Punctuated::default();
        types.push(self.ty.clone());
        types.push_punct(Default::default());
        syn::TypeTuple {
            paren_token: Default::default(),
            elems: types,
        }.into()
    }

    pub fn empty_ty_param() -> syn::TypeParam {
        syn::TypeParam::from(syn::Ident::new("x", proc_macro2::Span::call_site()))
    }
}
