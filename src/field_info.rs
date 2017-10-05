use syn;
use quote::Tokens;

use util::{make_identifier, map_only_one};

pub struct FieldInfo<'a> {
    pub ordinal: usize,
    pub name: &'a syn::Ident,
    pub generic_ident: syn::Ident,
    pub ty: &'a syn::Ty,
    pub default: Option<Tokens>,
}

impl<'a> FieldInfo<'a> {
    pub fn new(ordinal: usize, field: &syn::Field) -> FieldInfo {
        if let Some(ref name) = field.ident {
            FieldInfo {
                ordinal: ordinal,
                name: &name,
                generic_ident: make_identifier("genericType", name),
                ty: &field.ty,
                default: Self::find_field_default(field).unwrap_or_else(|f| panic!("Field {}: {}", name, f)),
            }
        } else {
            panic!("Nameless field in struct");
        }
    }

    fn find_field_default(field: &syn::Field) -> Result<Option<Tokens>, String> {
        map_only_one(&field.attrs, |attr| {
            match attr.value {
                syn::MetaItem::Word(ref name) if name == "default" => {
                    Ok(Some(quote!(::std::default::Default::default())))
                },
                syn::MetaItem::List(ref name, _) if name == "default" => {
                    Err("default can not be a list style attribute".into())
                }
                syn::MetaItem::NameValue(ref name, syn::Lit::Str(ref lit, _)) if name == "default" => {
                    let field_value = syn::parse_token_trees(lit)?;
                    Ok(Some(quote!(#( #field_value )*)))
                },
                _ => Ok(None)
            }
        })
    }

    pub fn generic_ty_param(&self) -> syn::TyParam {
        syn::TyParam::from(self.generic_ident.clone())
    }

    pub fn tuplized_type_ty_param(&self) -> syn :: TyParam {
        let ref ty = self.ty;
        let quoted = quote!((#ty,));
        syn::TyParam::from(syn::Ident::from(quoted.into_string()))
    }

    pub fn empty_ty_param() -> syn::TyParam {
        syn::TyParam::from(syn::Ident::from("()"))
    }
}
