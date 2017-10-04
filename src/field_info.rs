use syn;

#[derive(Debug)]
pub struct FieldInfo<'a> {
    pub ordinal: usize,
    pub name: &'a syn::Ident,
    pub generic_ident: syn::Ident,
    pub ty: &'a syn::Ty,
}

impl<'a> FieldInfo<'a> {
    pub fn new(ordinal: usize, field: &syn::Field) -> FieldInfo {
        if let Some(ref name) = field.ident {
            FieldInfo {
                ordinal: ordinal,
                name: &name,
                generic_ident: format!("_TypedBuilder__{}_", name).into(),
                ty: &field.ty,
            }
        } else {
            panic!("Nameless field in struct");
        }
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
