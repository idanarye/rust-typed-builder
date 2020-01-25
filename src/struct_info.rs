use syn;

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Error;

use crate::field_info::{
    FieldInfo,
    SetterArgSugar
};
use crate::util::{
    empty_type,
    type_tuple,
    empty_type_tuple,
    make_punctuated_single,
    modify_types_generics_hack,
    expr_to_single_string,
    path_to_single_string,
};

#[derive(Debug)]
pub struct StructInfo<'a> {
    pub vis: &'a syn::Visibility,
    pub name: &'a syn::Ident,
    pub generics: &'a syn::Generics,
    pub fields: Vec<FieldInfo<'a>>,

    pub builder_attr: TypeBuilderAttr,
    pub builder_name: syn::Ident,
    pub conversion_helper_trait_name: syn::Ident,
    pub core: syn::Ident,
}

impl<'a> StructInfo<'a> {
    pub fn included_fields(&self) -> impl Iterator<Item = &FieldInfo<'a>> {
        self.fields.iter().filter(|f| !f.builder_attr.setter.skip)
    }

    pub fn new(
        ast: &'a syn::DeriveInput,
        fields: impl Iterator<Item = &'a syn::Field>,
    ) -> Result<StructInfo<'a>, Error> {
        let builder_attr = TypeBuilderAttr::new(&ast.attrs)?;
        let builder_name = format!("{}Builder", ast.ident);
        Ok(StructInfo {
            vis: &ast.vis,
            name: &ast.ident,
            generics: &ast.generics,
            fields: fields
                .enumerate()
                .map(|(i, f)| FieldInfo::new(i, f))
                .collect::<Result<_, _>>()?,
            builder_attr: builder_attr,
            builder_name: syn::Ident::new(&builder_name, proc_macro2::Span::call_site()),
            conversion_helper_trait_name: syn::Ident::new(
                &format!("{}_Optional", builder_name),
                proc_macro2::Span::call_site(),
            ),
            core: syn::Ident::new(
                &format!("{}_core", builder_name),
                proc_macro2::Span::call_site(),
            ),
        })
    }

    fn modify_generics<F: FnMut(&mut syn::Generics)>(&self, mut mutator: F) -> syn::Generics {
        let mut generics = self.generics.clone();
        mutator(&mut generics);
        generics
    }

    pub fn builder_creation_impl(&self) -> Result<TokenStream, Error> {
        let StructInfo {
            ref vis,
            ref name,
            ref builder_name,
            ref core,
            ..
        } = *self;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let all_fields_param = syn::GenericParam::Type(syn::Ident::new("TypedBuilderFields", proc_macro2::Span::call_site()).into());
        let b_generics = self.modify_generics(|g| {
            g.params.insert(0, all_fields_param.clone());
        });
        let empties_tuple = type_tuple(self.included_fields().map(|_| empty_type()));
        let generics_with_empty = modify_types_generics_hack(&ty_generics, |args| {
            args.insert(0, syn::GenericArgument::Type(empties_tuple.clone().into()));
        });
        let phantom_generics = self.generics.params.iter().map(|param| {
            let t = match param {
                syn::GenericParam::Lifetime(lifetime) => quote!(&#lifetime ()),
                syn::GenericParam::Type(ty) => {
                    let ty = &ty.ident;
                    quote!(#ty)
                }
                syn::GenericParam::Const(cnst) => {
                    let cnst = &cnst.ident;
                    quote!(#cnst)
                }
            };
            quote!(#core::marker::PhantomData<#t>)
        });
        let builder_method_doc = match self.builder_attr.builder_method_doc {
            Some(ref doc) => quote!(#doc),
            None => {
                let doc = format!("
                    Create a builder for building `{name}`.
                    On the builder, call {setters} to set the values of the fields (they accept `Into` values).
                    Finally, call `.build()` to create the instance of `{name}`.
                    ",
                    name=self.name,
                    setters={
                        let mut result = String::new();
                        let mut is_first = true;
                        for field in self.included_fields() {
                            use std::fmt::Write;
                            if is_first {
                                is_first = false;
                            } else {
                                write!(&mut result, ", ").unwrap();
                            }
                            write!(&mut result, "`.{}(...)`", field.name).unwrap();
                            if field.builder_attr.default.is_some() {
                                write!(&mut result, "(optional)").unwrap();
                            }
                        }
                        result
                    });
                quote!(#doc)
            }
        };
        let builder_type_doc = if self.builder_attr.doc {
            match self.builder_attr.builder_type_doc {
                Some(ref doc) => quote!(#[doc = #doc]),
                None => {
                    let doc = format!("Builder for [`{name}`] instances.\n\nSee [`{name}::builder()`] for more info.", name = name);
                    quote!(#[doc = #doc])
                }
            }
        } else {
            quote!(#[doc(hidden)])
        };
        Ok(quote! {
            extern crate core as #core;
            impl #impl_generics #name #ty_generics #where_clause {
                #[doc = #builder_method_doc]
                #[allow(dead_code)]
                #vis fn builder() -> #builder_name #generics_with_empty {
                    #builder_name {
                        fields: #empties_tuple,
                        _phantom: #core::default::Default::default(),
                    }
                }
            }

            #[must_use]
            #builder_type_doc
            #[allow(dead_code, non_camel_case_types, non_snake_case)]
            #vis struct #builder_name #b_generics {
                fields: #all_fields_param,
                _phantom: (#( #phantom_generics ),*),
            }
        })
    }

    // TODO: once the proc-macro crate limitation is lifted, make this an util trait of this
    // crate.
    pub fn conversion_helper_impl(&self) -> Result<TokenStream, Error> {
        let trait_name = &self.conversion_helper_trait_name;
        Ok(quote! {
            #[doc(hidden)]
            #[allow(dead_code, non_camel_case_types, non_snake_case)]
            pub trait #trait_name<T> {
                fn into_value<F: FnOnce() -> T>(self, default: F) -> T;
            }

            impl<T> #trait_name<T> for () {
                fn into_value<F: FnOnce() -> T>(self, default: F) -> T {
                    default()
                }
            }

            impl<T> #trait_name<T> for (T,) {
                fn into_value<F: FnOnce() -> T>(self, _: F) -> T {
                    self.0
                }
            }
        })
    }

    pub fn field_impl(&self, field: &FieldInfo) -> Result<TokenStream, Error> {
        let StructInfo {
            ref builder_name,
            ref core,
            ..
        } = *self;

        let descructuring = self
            .included_fields()
            .map(|f| {
                if f.ordinal == field.ordinal {
                    quote!(_)
                } else {
                    let name = f.name;
                    quote!(#name)
                }
            });
        let reconstructing = self
            .included_fields()
            .map(|f| f.name);

        let &FieldInfo {
            name: ref field_name,
            ty: ref field_type,
            ..
        } = field;
        let mut ty_generics: Vec<syn::GenericArgument> = self
            .generics
            .params
            .iter()
            .map(|generic_param| match generic_param {
                syn::GenericParam::Type(type_param) => {
                    let ident = type_param.ident.clone();
                    syn::parse(quote!(#ident).into()).unwrap()
                }
                syn::GenericParam::Lifetime(lifetime_def) => {
                    syn::GenericArgument::Lifetime(lifetime_def.lifetime.clone())
                }
                syn::GenericParam::Const(const_param) => {
                    let ident = const_param.ident.clone();
                    syn::parse(quote!(#ident).into()).unwrap()
                }
            })
            .collect();
        let mut target_generics_tuple = empty_type_tuple();
        let mut ty_generics_tuple = empty_type_tuple();
        let generics = self.modify_generics(|g| {
            for f in self.included_fields() {
                if f.ordinal == field.ordinal {
                    ty_generics_tuple.elems.push_value(empty_type());
                    target_generics_tuple.elems.push_value(f.tuplized_type_ty_param());
                } else {
                    g.params.push(f.generic_ty_param());
                    let generic_argument: syn::Type = f.type_ident().into();
                    ty_generics_tuple.elems.push_value(generic_argument.clone());
                    target_generics_tuple.elems.push_value(generic_argument);
                }
                ty_generics_tuple.elems.push_punct(Default::default());
                target_generics_tuple.elems.push_punct(Default::default());
            }
        });
        let mut target_generics = ty_generics.clone();

        let index_after_lifetime_in_generics = target_generics.iter().filter(|arg| {
            if let syn::GenericArgument::Lifetime(_) = arg { true } else { false }
        }).count();
        target_generics.insert(index_after_lifetime_in_generics, syn::GenericArgument::Type(target_generics_tuple.into()));
        ty_generics.insert(index_after_lifetime_in_generics, syn::GenericArgument::Type(ty_generics_tuple.into()));
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let doc = match field.builder_attr.setter.doc {
            Some(ref doc) => quote!(#[doc = #doc]),
            None => quote!(),
        };

        let (arg_type, arg_expr) = match field.builder_attr.setter.arg_sugar {
            SetterArgSugar::NoSugar => (
                quote!(#field_type),
                quote!(#field_name),
            ),
            SetterArgSugar::AutoInto => (
                quote!(impl #core::convert::Into<#field_type>),
                quote!(#field_name.into()),
            ),
            SetterArgSugar::StripOption => {
                let internal_type = field.type_from_inside_option()
                    .ok_or_else(|| Error::new_spanned(&field_type, "can't `strip_option` - field is not `Option<...>`"))?;
                (
                    quote!(#internal_type),
                    quote!(Some(#field_name)),
                )
            },
        };

        let repeated_fields_error_type_name = syn::Ident::new(
            &format!("{}_Error_Repeated_field_{}", builder_name, field_name),
            proc_macro2::Span::call_site(),
        );
        let repeated_fields_error_message = format!("Repeated field {}", field_name);

        Ok(quote! {
            #[allow(dead_code, non_camel_case_types, missing_docs)]
            impl #impl_generics #builder_name < #( #ty_generics ),* > #where_clause {
                #doc
                pub fn #field_name (self, #field_name: #arg_type) -> #builder_name < #( #target_generics ),* > {
                    let #field_name = (#arg_expr,);
                    let ( #(#descructuring,)* ) = self.fields;
                    #builder_name {
                        fields: ( #(#reconstructing,)* ),
                        _phantom: self._phantom,
                    }
                }
            }
            #[doc(hidden)]
            #[allow(dead_code, non_camel_case_types, non_snake_case)]
            pub enum #repeated_fields_error_type_name {}
            #[doc(hidden)]
            #[allow(dead_code, non_camel_case_types, missing_docs)]
            impl #impl_generics #builder_name < #( #target_generics ),* > #where_clause {
                #[deprecated(
                    note = #repeated_fields_error_message
                )]
                pub fn #field_name (self, _: #repeated_fields_error_type_name) -> #builder_name < #( #target_generics ),* > {
                    self
                }
            }
        })
    }

    pub fn required_field_impl(&self, field: &FieldInfo) -> Result<TokenStream, Error> {
        let StructInfo {
            ref name,
            ref builder_name,
            ..
        } = self;

        let FieldInfo {
            name: ref field_name,
            ..
        } = field;
        let mut builder_generics: Vec<syn::GenericArgument> = self
            .generics
            .params
            .iter()
            .map(|generic_param| match generic_param {
                syn::GenericParam::Type(type_param) => {
                    let ident = &type_param.ident;
                    syn::parse(quote!(#ident).into()).unwrap()
                }
                syn::GenericParam::Lifetime(lifetime_def) => {
                    syn::GenericArgument::Lifetime(lifetime_def.lifetime.clone())
                }
                syn::GenericParam::Const(const_param) => {
                    let ident = &const_param.ident;
                    syn::parse(quote!(#ident).into()).unwrap()
                }
            })
            .collect();
        let mut builder_generics_tuple = empty_type_tuple();
        let generics = self.modify_generics(|g| {
            for f in self.included_fields() {
                if f.builder_attr.default.is_some() {
                    // `f` is not mandatory - it does not have it's own fake `build` method, so `field` will need
                    // to warn about missing `field` whether or not `f` is set.
                    assert!(f.ordinal != field.ordinal, "`required_field_impl` called for optional field {}", field.name);
                    g.params.push(f.generic_ty_param());
                    builder_generics_tuple.elems.push_value(f.type_ident().into());
                } else if f.ordinal < field.ordinal {
                    // Only add a `build` method that warns about missing `field` if `f` is set. If `f` is not set,
                    // `f`'s `build` method will warn, since it appears earlier in the argument list.
                    builder_generics_tuple.elems.push_value(f.tuplized_type_ty_param());
                } else if f.ordinal == field.ordinal {
                    builder_generics_tuple.elems.push_value(empty_type());
                } else {
                    // `f` appears later in the argument list after `field`, so if they are both missing we will
                    // show a warning for `field` and not for `f` - which means this warning should appear whether
                    // or not `f` is set.
                    g.params.push(f.generic_ty_param());
                    builder_generics_tuple.elems.push_value(f.type_ident().into());
                }

                builder_generics_tuple.elems.push_punct(Default::default());
            }
        });

        let index_after_lifetime_in_generics = builder_generics.iter().filter(|arg| {
            if let syn::GenericArgument::Lifetime(_) = arg { true } else { false }
        }).count();
        builder_generics.insert(index_after_lifetime_in_generics, syn::GenericArgument::Type(builder_generics_tuple.into()));
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        let early_build_error_type_name = syn::Ident::new(
            &format!(
                "{}_Error_Missing_required_field_{}",
                builder_name, field_name
            ),
            proc_macro2::Span::call_site(),
        );
        let early_build_error_message = format!("Missing required field {}", field_name);

        Ok(quote! {
            #[doc(hidden)]
            #[allow(dead_code, non_camel_case_types, non_snake_case)]
            pub enum #early_build_error_type_name {}
            #[doc(hidden)]
            #[allow(dead_code, non_camel_case_types, missing_docs, clippy::panic)]
            impl #impl_generics #builder_name < #( #builder_generics ),* > #where_clause {
                #[deprecated(
                    note = #early_build_error_message
                )]
                pub fn build(self, _: #early_build_error_type_name) -> #name #ty_generics {
                    panic!();
                }
            }
        })
    }

    pub fn build_method_impl(&self) -> TokenStream {
        let StructInfo {
            ref name,
            ref builder_name,
            ..
        } = *self;

        let generics = self.modify_generics(|g| {
            for field in self.included_fields() {
                if field.builder_attr.default.is_some() {
                    let trait_ref = syn::TraitBound {
                        paren_token: None,
                        lifetimes: None,
                        modifier: syn::TraitBoundModifier::None,
                        path: syn::PathSegment {
                            ident: self.conversion_helper_trait_name.clone(),
                            arguments: syn::PathArguments::AngleBracketed(
                                syn::AngleBracketedGenericArguments {
                                    colon2_token: None,
                                    lt_token: Default::default(),
                                    args: make_punctuated_single(syn::GenericArgument::Type(
                                        field.ty.clone(),
                                    )),
                                    gt_token: Default::default(),
                                },
                            ),
                        }
                        .into(),
                    };
                    let mut generic_param: syn::TypeParam = field.generic_ident.clone().into();
                    generic_param.bounds.push(trait_ref.into());
                    g.params.push(generic_param.into());
                }
            }
        });
        let (impl_generics, _, _) = generics.split_for_impl();

        let (_, ty_generics, where_clause) = self.generics.split_for_impl();

        let modified_ty_generics = modify_types_generics_hack(&ty_generics, |args| {
            args.insert(0, syn::GenericArgument::Type(type_tuple(self.included_fields().map(|field| {
                if field.builder_attr.default.is_some() {
                    field.type_ident()
                } else {
                    field.tuplized_type_ty_param()
                }
            })).into()));
        });

        let descructuring = self
            .included_fields()
            .map(|f| f.name);

        let ref helper_trait_name = self.conversion_helper_trait_name;
        // The default of a field can refer to earlier-defined fields, which we handle by
        // writing out a bunch of `let` statements first, which can each refer to earlier ones.
        // This means that field ordering may actually be significant, which isn’t ideal. We could
        // relax that restriction by calculating a DAG of field default dependencies and
        // reordering based on that, but for now this much simpler thing is a reasonable approach.
        let assignments = self.fields.iter().map(|field| {
            let ref name = field.name;
            if let Some(ref default) = field.builder_attr.default {
                if field.builder_attr.setter.skip {
                    quote!(let #name = #default;)
                } else {
                    quote!(let #name = #helper_trait_name::into_value(#name, || #default);)
                }
            } else {
                quote!(let #name = #name.0;)
            }
        });
        let field_names = self.fields.iter().map(|field| field.name);
        let doc = if self.builder_attr.doc {
            match self.builder_attr.build_method_doc {
                Some(ref doc) => quote!(#[doc = #doc]),
                None => {
                    // I’d prefer “a” or “an” to “its”, but determining which is grammatically
                    // correct is roughly impossible.
                    let doc = format!("Finalise the builder and create its [`{}`] instance", name);
                    quote!(#[doc = #doc])
                }
            }
        } else {
            quote!()
        };
        quote!(
            #[allow(dead_code, non_camel_case_types, missing_docs)]
            impl #impl_generics #builder_name #modified_ty_generics #where_clause {
                #doc
                pub fn build(self) -> #name #ty_generics {
                    let ( #(#descructuring,)* ) = self.fields;
                    #( #assignments )*
                    #name {
                        #( #field_names ),*
                    }
                }
            }
        )
        .into()
    }
}

#[derive(Debug, Default)]
pub struct TypeBuilderAttr {
    /// Whether to show docs for the `TypeBuilder` type (rather than hiding them).
    pub doc: bool,

    /// Docs on the `Type::builder()` method.
    pub builder_method_doc: Option<syn::Expr>,

    /// Docs on the `TypeBuilder` type. Specifying this implies `doc`, but you can just specify
    /// `doc` instead and a default value will be filled in here.
    pub builder_type_doc: Option<syn::Expr>,

    /// Docs on the `TypeBuilder.build()` method. Specifying this implies `doc`, but you can just
    /// specify `doc` instead and a default value will be filled in here.
    pub build_method_doc: Option<syn::Expr>,
}

impl TypeBuilderAttr {
    pub fn new(attrs: &[syn::Attribute]) -> Result<TypeBuilderAttr, Error> {
        let mut result = TypeBuilderAttr::default();
        for attr in attrs {
            if path_to_single_string(&attr.path).as_ref().map(|s| &**s) != Some("builder") {
                continue;
            }

            if attr.tokens.is_empty() {
                continue;
            }
            let as_expr: syn::Expr = syn::parse2(attr.tokens.clone())?;

            match as_expr {
                syn::Expr::Paren(body) => {
                    result.apply_meta(*body.expr)?;
                }
                syn::Expr::Tuple(body) => {
                    for expr in body.elems.into_iter() {
                        result.apply_meta(expr)?;
                    }
                }
                _ => {
                    return Err(Error::new_spanned(attr.tokens.clone(), "Expected (<...>)"));
                }
            }
        }

        Ok(result)
    }

    fn apply_meta(&mut self, expr: syn::Expr) -> Result<(), Error> {
        match expr {
            syn::Expr::Assign(assign) => {
                let name = expr_to_single_string(&assign.left)
                    .ok_or_else(|| Error::new_spanned(&assign.left, "Expected identifier"))?;
                match name.as_str() {
                    "builder_method_doc" => {
                        self.builder_method_doc = Some(*assign.right);
                        Ok(())
                    }
                    "builder_type_doc" => {
                        self.builder_type_doc = Some(*assign.right);
                        self.doc = true;
                        Ok(())
                    }
                    "build_method_doc" => {
                        self.build_method_doc = Some(*assign.right);
                        self.doc = true;
                        Ok(())
                    }
                    _ => Err(Error::new_spanned(
                        &assign,
                        format!("Unknown parameter {:?}", name),
                    )),
                }
            }
            syn::Expr::Path(path) => {
                let name = path_to_single_string(&path.path)
                    .ok_or_else(|| Error::new_spanned(&path, "Expected identifier"))?;
                match name.as_str() {
                    "doc" => {
                        self.doc = true;
                        Ok(())
                    }
                    _ => Err(Error::new_spanned(
                        &path,
                        format!("Unknown parameter {:?}", name),
                    )),
                }
            }
            _ => Err(Error::new_spanned(expr, "Expected (<...>=<...>)")),
        }
    }
}
