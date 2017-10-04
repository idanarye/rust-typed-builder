use syn;

use quote::Tokens;

use field_info::FieldInfo;

pub struct StructInfo<'a> {
    pub vis: &'a syn::Visibility,
    pub name: &'a syn::Ident,
    pub builder_name: syn::Ident,
    pub generics: &'a syn::Generics,
    pub fields: Vec<FieldInfo<'a>>,
}

impl<'a> StructInfo<'a> {
    pub fn new(ast: &'a syn::DeriveInput, fields: &'a [syn::Field]) -> StructInfo<'a> {
        StructInfo {
            vis: &ast.vis,
            name: &ast.ident,
            builder_name: format!("{}Builder", ast.ident).into(),
            generics: &ast.generics,
            fields: fields.iter().enumerate().map(|(i, f)| FieldInfo::new(i, f)).collect(),
        }
    }

    fn modify_generics<F: Fn(&mut syn::Generics)>(&self, mutator: F) -> syn::Generics {
        let mut generics = self.generics.clone();
        mutator(&mut generics);
        generics
    }

    pub fn builder_creation_impl(&self) -> Tokens {
        let _ = self.modify_generics(|g| g.ty_params.push(self.fields[0].generic_ty_param()));
        let init_empties = {
            let names = self.fields.iter().map(|f| f.name);
            quote!(#( #names: () ),*)
        };
        let builder_generics = {
            let names = self.fields.iter().map(|f| f.name);
            let generic_idents = self.fields.iter().map(|f| &f.generic_ident);
            quote!(#( #names: #generic_idents ),*)
        };
        let StructInfo {
            ref vis,
            ref name,
            ref builder_name,
            ..
        } = *self;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let b_generics = self.modify_generics(|g| {
            for field in self.fields.iter() {
                g.ty_params.push(field.generic_ty_param());
            }
        });
        let generics_with_empty = self.modify_generics(|g| {
            for _ in self.fields.iter() {
                g.ty_params.push(FieldInfo::empty_ty_param());
            }
        });
        let (_, generics_with_empty, _) = generics_with_empty.split_for_impl();
        let phantom_generics = {
            let lifetimes = self.generics.lifetimes.iter().map(|l| &l.lifetime);
            let types = self.generics.ty_params.iter().map(|t| &t.ident);
            quote!{
                #( ::std::marker::PhantomData<&#lifetimes ()>, )*
                #( ::std::marker::PhantomData<#types>, )*
            }
        };
        quote! {
            impl #impl_generics #name #ty_generics #where_clause {
                // #[doc = #doc]
                #[allow(dead_code)]
                #vis fn builder() -> #builder_name #generics_with_empty {
                    #builder_name {
                        _TypedBuilder__phantomGenerics_: ::std::default::Default::default(),
                        #init_empties
                    }
                }
            }

            #[allow(dead_code, non_camel_case_types, non_snake_case)]
            #vis struct #builder_name #b_generics {
                _TypedBuilder__phantomGenerics_: (#phantom_generics),
                #builder_generics
            }
        }
    }

    pub fn field_impl(&self, field: &FieldInfo) -> Tokens {
        let ref builder_name = self.builder_name;
        let other_fields_name = self.fields.iter().filter(|f| f.ordinal != field.ordinal).map(|f| f.name);
        // not really "value", since we just use to self.name - but close enough.
        let other_fields_value = self.fields.iter().filter(|f| f.ordinal != field.ordinal).map(|f| f.name);
        let &FieldInfo {
            name: ref field_name,
            ty: ref field_type,
            ref generic_ident,
            ..
        } = field;
        let generics = self.modify_generics(|g| {
            for f in self.fields.iter() {
                if f.ordinal != field.ordinal {
                    g.ty_params.push(f.generic_ty_param());
                }
            }
        });
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let generics = self.modify_generics(|g| {
            for f in self.fields.iter() {
                if f.ordinal != field.ordinal {
                    g.ty_params.push(f.generic_ty_param());
                } else {
                    g.ty_params.push(FieldInfo::empty_ty_param());
                }
            }
        });
        let (_, ty_generics, _) = generics.split_for_impl();
        let generics = self.modify_generics(|g| {
            for f in self.fields.iter() {
                if f.ordinal != field.ordinal {
                    g.ty_params.push(f.generic_ty_param());
                } else {
                    g.ty_params.push(f.tuplized_type_ty_param());
                }
            }
        });
        let (_, target_generics, _) = generics.split_for_impl();
        quote!{
            #[allow(dead_code, non_camel_case_types)]
            impl #impl_generics #builder_name #ty_generics #where_clause {
                pub fn #field_name<#generic_ident: ::std::convert::Into<#field_type>>(self, value: #generic_ident) -> #builder_name #target_generics {
                    #builder_name {
                        _TypedBuilder__phantomGenerics_: self._TypedBuilder__phantomGenerics_,
                        #field_name: (value.into(),),
                        #( #other_fields_name: self.#other_fields_value ),*
                    }
                }
            }
        }
    }

    pub fn build_method_impl(&self) -> Tokens {
        let StructInfo {
            ref name,
            ref builder_name,
            ..
        } = *self;

        let modified_generics = self.modify_generics(|g| {
            for field in self.fields.iter() {
                g.ty_params.push(field.tuplized_type_ty_param());
            }
        });
        let (_, modified_ty_generics, _) = modified_generics.split_for_impl();
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        let assignments_name = self.fields.iter().map(|f| f.name);
        // not really "value", since for now we just use to self.name.0 - but close enough.
        let assignments_value = self.fields.iter().map(|f| f.name);

        quote! {
            #[allow(dead_code)]
            impl #impl_generics #builder_name #modified_ty_generics #where_clause {
                pub fn build(self) -> #name #ty_generics {
                    #name {
                        #( #assignments_name: self.#assignments_value.0 ),*
                    }
                }
            }
        }
    }
}
