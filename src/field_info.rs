use syn;
use syn::spanned::Spanned;
use syn::parse::Error;

use crate::util::ident_to_type;
use crate::builder_attr::FieldBuilderAttr;

#[derive(Debug)]
pub struct FieldInfo<'a> {
    pub ordinal: usize,
    pub name: &'a syn::Ident,
    pub generic_ident: syn::Ident,
    pub ty: &'a syn::Type,
    pub builder_attr: FieldBuilderAttr,
}

impl<'a> FieldInfo<'a> {
    pub fn new(ordinal: usize, field: &syn::Field) -> Result<FieldInfo, Error> {
        if let Some(ref name) = field.ident {
            Ok(FieldInfo {
                ordinal: ordinal,
                name: &name,
                generic_ident: syn::Ident::new(
                    &format!("__{}", name),
                    proc_macro2::Span::call_site(),
                ),
                ty: &field.ty,
                builder_attr: FieldBuilderAttr::new(&field.attrs)?,
            })
        } else {
            Err(Error::new(field.span(), "Nameless field in struct"))
        }
    }

    pub fn generic_ty_param(&self) -> syn::GenericParam {
        syn::GenericParam::Type(self.generic_ident.clone().into())
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
}
