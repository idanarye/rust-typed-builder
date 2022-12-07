use either::Either::*;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use regex::Regex;
use syn::parse::Error;
use syn::punctuated::Punctuated;

use crate::field_info::{FieldBuilderAttr, FieldInfo};
use crate::util::{
    empty_type_tuple, expr_to_single_string, expr_tuple, make_punctuated_single, modify_types_generics_hack,
    path_to_single_string, strip_raw_ident_prefix, type_tuple, GenericDefault,
};

#[derive(Debug)]
pub struct StructInfo<'a> {
    pub vis: &'a syn::Visibility,
    pub name: &'a syn::Ident,
    pub generics: &'a syn::Generics,
    pub fields: Vec<FieldInfo<'a>>,
    // all generics, just with default types removed
    pub generics_without_defaults: syn::Generics,
    /// equivalent of a TyGenerics struct where TypeParams are replaced by their defaults if provided
    pub ty_generics_with_defaults: Punctuated<syn::GenericArgument, syn::token::Comma>,
    /// only generics which had no defaults specified
    pub no_default_generics: syn::Generics,
    /// map of word matching regex patterns for TypeParam idents to their corresponding default type
    pub generic_defaults: Vec<GenericDefault>,

    pub builder_attr: TypeBuilderAttr,
    pub builder_name: syn::Ident,
    pub conversion_helper_trait_name: syn::Ident,
    pub core: syn::Ident,
}

impl<'a> StructInfo<'a> {
    pub fn included_fields(&self) -> impl Iterator<Item = &FieldInfo<'a>> {
        self.fields.iter().filter(|f| f.builder_attr.setter.skip.is_none())
    }

    pub fn new(ast: &'a syn::DeriveInput, fields: impl Iterator<Item = &'a syn::Field>) -> Result<StructInfo<'a>, Error> {
        let builder_attr = TypeBuilderAttr::new(&ast.attrs)?;
        let builder_name = strip_raw_ident_prefix(format!("{}Builder", ast.ident));

        let mut generics_without_defaults = ast.generics.clone();
        generics_without_defaults.params = generics_without_defaults
            .params
            .into_iter()
            .map(|param| match param {
                syn::GenericParam::Type(type_param) => syn::GenericParam::Type(syn::TypeParam {
                    attrs: type_param.attrs,
                    ident: type_param.ident,
                    colon_token: type_param.colon_token,
                    bounds: type_param.bounds,
                    eq_token: None,
                    default: None,
                }),
                syn::GenericParam::Const(const_param) => syn::GenericParam::Const(syn::ConstParam {
                    attrs: const_param.attrs,
                    const_token: const_param.const_token,
                    ident: const_param.ident,
                    colon_token: const_param.colon_token,
                    ty: const_param.ty,
                    eq_token: None,
                    default: None,
                }),
                param => param,
            })
            .collect();

        let ty_generics_with_defaults: Punctuated<_, syn::token::Comma> = ast
            .generics
            .params
            .clone()
            .into_iter()
            .map::<Result<syn::GenericArgument, _>, _>(|param| match param {
                syn::GenericParam::Type(type_param) => match type_param.default {
                    Some(default) => syn::parse(proc_macro::TokenStream::from(quote!(#default))),
                    None => {
                        let ident = type_param.ident;
                        syn::parse(proc_macro::TokenStream::from(quote!(#ident)))
                    }
                },
                syn::GenericParam::Lifetime(syn::LifetimeDef { lifetime, .. }) => {
                    syn::parse(proc_macro::TokenStream::from(quote!(#lifetime)))
                }
                syn::GenericParam::Const(const_param) => match const_param.default {
                    Some(default) => syn::parse(proc_macro::TokenStream::from(quote!(#default))),
                    None => {
                        let ident = const_param.ident;
                        syn::parse(proc_macro::TokenStream::from(quote!(#ident)))
                    }
                },
            })
            .collect::<Result<_, _>>()?;

        let mut no_default_generics = ast.generics.clone();
        let mut generic_defaults = Vec::<GenericDefault>::default();
        no_default_generics.params = no_default_generics
            .params
            .into_iter()
            .filter_map(|param| match param {
                syn::GenericParam::Type(type_param) => match type_param.default.clone() {
                    Some(default) => {
                        let ident = &type_param.ident;
                        let regular_expression = Regex::new(format!(r#"\b{}\b"#, quote!(#ident)).trim()).expect(&format!(
                            "unable to replace generic parameter `{}`, not a matchable regex pattern",
                            format!("{}", quote!(#type_param))
                        ));
                        generic_defaults.push((
                            Left(type_param),
                            regular_expression,
                            Some(format!("{}", quote!(#default)).trim().to_string()),
                        ));
                        None
                    }
                    None => {
                        generic_defaults.push((
                            Left(type_param.clone()),
                            Regex::new(format!(r#"\b{}\b"#, quote!(#type_param)).trim()).expect(&format!(
                                "unable to replace generic parameter `{}`, not a matchable regex pattern",
                                format!("{}", quote!(#type_param))
                            )),
                            None,
                        ));
                        Some(syn::GenericParam::Type(type_param))
                    }
                },
                syn::GenericParam::Const(const_param) => match const_param.default.clone() {
                    Some(default) => {
                        let ident = &const_param.ident;
                        let regular_expression = Regex::new(format!(r#"\b{}\b"#, quote!(#ident)).trim()).expect(&format!(
                            "unable to replace generic parameter `{}`, not a matchable regex pattern",
                            format!("{}", quote!(#const_param))
                        ));
                        generic_defaults.push((
                            Right(const_param),
                            regular_expression,
                            Some(format!("{}", quote!(#default)).trim().to_string()),
                        ));
                        None
                    }
                    None => {
                        generic_defaults.push((
                            Right(const_param.clone()),
                            Regex::new(format!(r#"\b{}\b"#, quote!(#const_param)).trim()).expect(&format!(
                                "unable to replace generic parameter `{}`, not a matchable regex pattern",
                                format!("{}", quote!(#const_param))
                            )),
                            None,
                        ));
                        Some(syn::GenericParam::Const(const_param))
                    }
                },
                param => Some(param),
            })
            .collect();

        Ok(StructInfo {
            vis: &ast.vis,
            name: &ast.ident,
            generics: &ast.generics,
            fields: fields
                .enumerate()
                .map(|(i, f)| FieldInfo::new(i, f, builder_attr.field_defaults.clone(), &generic_defaults))
                .collect::<Result<_, _>>()?,
            builder_name: syn::Ident::new(&builder_name, proc_macro2::Span::call_site()),
            conversion_helper_trait_name: syn::Ident::new(&format!("{}_Optional", builder_name), proc_macro2::Span::call_site()),
            core: syn::Ident::new(&format!("{}_core", builder_name), proc_macro2::Span::call_site()),
            builder_attr,
            generics_without_defaults,
            ty_generics_with_defaults,
            no_default_generics,
            generic_defaults,
        })
    }

    fn modify_generics<F: FnMut(&mut syn::Generics)>(&self, mut mutator: F) -> syn::Generics {
        let mut generics = self.generics.clone();
        mutator(&mut generics);
        generics
    }

    fn modify_generics_alter_if_used_default_generic<F: FnMut(&mut syn::Generics)>(
        &self,
        mut mutator: F,
        field: &FieldInfo,
    ) -> syn::Generics {
        let mut generics = self.generics.clone();
        generics.params.iter_mut().for_each(|param| match param {
            syn::GenericParam::Type(ref mut type_param) => {
                if type_param.default.is_some() && field.used_default_generic_idents.contains(&type_param.ident) {
                    type_param.ident = format_ident_target_generic_default(&type_param.ident);
                }
            }
            syn::GenericParam::Const(ref mut const_param) => {
                if const_param.default.is_some() && field.used_default_generic_idents.contains(&const_param.ident) {
                    const_param.ident = format_ident_target_generic_default(&const_param.ident);
                }
            }
            _ => {}
        });
        mutator(&mut generics);
        generics
    }

    fn modify_generics_with_no_defaults<F: FnMut(&mut syn::Generics)>(&self, mut mutator: F) -> syn::Generics {
        let mut generics = self.generics_without_defaults.clone();
        mutator(&mut generics);
        generics
    }

    fn ty_generics_with_defaults_except_field(&self, field: &FieldInfo) -> Result<Vec<syn::GenericArgument>, Error> {
        self.generics
            .params
            .clone()
            .into_iter()
            .map::<Result<syn::GenericArgument, _>, _>(|param| match param {
                syn::GenericParam::Type(type_param) => match field.used_default_generic_idents.contains(&type_param.ident) {
                    true => {
                        let ident = format_ident_target_generic_default(&type_param.ident);
                        syn::parse(proc_macro::TokenStream::from(quote!(#ident)))
                    }
                    false => {
                        let ident = &type_param.ident;
                        syn::parse(proc_macro::TokenStream::from(quote!(#ident)))
                    }
                },
                syn::GenericParam::Lifetime(syn::LifetimeDef { lifetime, .. }) => {
                    syn::parse(proc_macro::TokenStream::from(quote!(#lifetime)))
                }
                syn::GenericParam::Const(const_param) => match field.used_default_generic_idents.contains(&const_param.ident) {
                    true => {
                        let ident = format_ident_target_generic_default(&const_param.ident);
                        syn::parse(proc_macro::TokenStream::from(quote!(#ident)))
                    }
                    false => {
                        let ident = &const_param.ident;
                        syn::parse(proc_macro::TokenStream::from(quote!(#ident)))
                    }
                },
            })
            .collect::<Result<_, _>>()
    }

    pub fn builder_creation_impl(&self) -> Result<TokenStream, Error> {
        let StructInfo {
            ref vis,
            ref name,
            ref builder_name,
            ..
        } = *self;
        let (impl_generics, ty_generics, where_clause) = self.no_default_generics.split_for_impl();
        let all_fields_param =
            syn::GenericParam::Type(syn::Ident::new("TypedBuilderFields", proc_macro2::Span::call_site()).into());
        let b_generics = self.modify_generics_with_no_defaults(|g| {
            g.params.insert(0, all_fields_param.clone());
        });
        let expr_empties_tuple = expr_tuple(self.included_fields().map(|f| f.default_expr()));
        let ty_empties_tuple = type_tuple(self.included_fields().map(|f| f.default_type()));

        let mut ty_generics_with_defaults = self.ty_generics_with_defaults.clone();
        ty_generics_with_defaults.insert(0, syn::GenericArgument::Type(ty_empties_tuple.clone().into()));

        let generics_with_empty = syn::AngleBracketedGenericArguments {
            colon2_token: None,
            lt_token: Default::default(),
            args: ty_generics_with_defaults,
            gt_token: Default::default(),
        };
        let phantom_generics = self.generics.params.iter().map(|param| match param {
            syn::GenericParam::Lifetime(lifetime) => {
                let lifetime = &lifetime.lifetime;
                quote!(::core::marker::PhantomData<&#lifetime ()>)
            }
            syn::GenericParam::Type(ty) => {
                let ty = &ty.ident;
                quote!(::core::marker::PhantomData<#ty>)
            }
            syn::GenericParam::Const(_cnst) => {
                quote!()
            }
        });
        let builder_method_doc = if let Some(ref doc) = self.builder_attr.builder_method_doc {
            quote!(#doc)
        } else {
            let doc = format!(
                "
                    Create a builder for building `{name}`.
                    On the builder, call {setters} to set the values of the fields.
                    Finally, call `.build()` to create the instance of `{name}`.
                    ",
                name = self.name,
                setters = {
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
                }
            );
            quote!(#doc)
        };
        let builder_type_doc = if self.builder_attr.doc {
            if let Some(ref doc) = self.builder_attr.builder_type_doc {
                quote!(#[doc = #doc])
            } else {
                let doc = format!(
                    "Builder for [`{name}`] instances.\n\nSee [`{name}::builder()`] for more info.",
                    name = name
                );
                quote!(#[doc = #doc])
            }
        } else {
            quote!(#[doc(hidden)])
        };

        let (b_generics_impl, b_generics_ty, b_generics_where_extras_predicates) = b_generics.split_for_impl();
        let mut b_generics_where: syn::WhereClause = syn::parse2(quote! {
            where TypedBuilderFields: Clone
        })?;
        if let Some(predicates) = b_generics_where_extras_predicates {
            b_generics_where.predicates.extend(predicates.predicates.clone());
        }

        Ok(quote! {
            impl #impl_generics #name #ty_generics #where_clause {
                #[doc = #builder_method_doc]
                #[allow(dead_code, clippy::default_trait_access)]
                #vis fn builder() -> #builder_name #generics_with_empty {
                    #builder_name {
                        fields: #expr_empties_tuple,
                        phantom: ::core::default::Default::default(),
                    }
                }
            }

            #[must_use]
            #builder_type_doc
            #[allow(dead_code, non_camel_case_types, non_snake_case)]
            #vis struct #builder_name #b_generics {
                fields: #all_fields_param,
                phantom: (#( #phantom_generics ),*),
            }

            impl #b_generics_impl Clone for #builder_name #b_generics_ty #b_generics_where {
                #[allow(clippy::default_trait_access)]
                fn clone(&self) -> Self {
                    Self {
                        fields: self.fields.clone(),
                        phantom: ::core::default::Default::default(),
                    }
                }
            }
        })
    }

    // TODO: once the proc-macro crate limitation is lifted, make this an util trait of this
    // crate.
    pub fn conversion_helper_impl(&self) -> TokenStream {
        let trait_name = &self.conversion_helper_trait_name;
        quote! {
            #[doc(hidden)]
            #[allow(dead_code, non_camel_case_types, non_snake_case)]
            pub trait #trait_name<T> {
                fn into_value<F: FnOnce() -> Option<T>>(self, default: F) -> T;
            }

            impl<T> #trait_name<T> for () {
                fn into_value<F: FnOnce() -> Option<T>>(self, default: F) -> T {
                    default().unwrap()
                }
            }

            impl<T> #trait_name<T> for (T,) {
                fn into_value<F: FnOnce() -> Option<T>>(self, _: F) -> T {
                    self.0
                }
            }
        }
    }

    pub fn field_impl(&self, field: &FieldInfo) -> Result<TokenStream, Error> {
        let StructInfo { ref builder_name, .. } = *self;

        let descructuring = self.included_fields().map(|f| {
            if f.ordinal == field.ordinal {
                quote!(_)
            } else {
                let name = f.name;
                quote!(#name)
            }
        });
        let reconstructing = self.included_fields().map(|f| f.name);

        let &FieldInfo {
            name: ref field_name,
            ty: field_type,
            ..
        } = field;

        let mut target_generics: Vec<syn::GenericArgument> = self
            .generics
            .params
            .iter()
            .map(|generic_param| match generic_param {
                syn::GenericParam::Type(type_param) => {
                    let ident = type_param.ident.clone();
                    syn::parse(quote!(#ident).into()).unwrap()
                }
                syn::GenericParam::Lifetime(lifetime_def) => syn::GenericArgument::Lifetime(lifetime_def.lifetime.clone()),
                syn::GenericParam::Const(const_param) => {
                    let ident = const_param.ident.clone();
                    syn::parse(quote!(#ident).into()).unwrap()
                }
            })
            .collect();

        let mut ty_generics: Vec<syn::GenericArgument> = if !field.used_default_generic_idents.is_empty() {
            self.ty_generics_with_defaults_except_field(field)?
        } else {
            target_generics.clone()
        };

        let mut target_generics_tuple = empty_type_tuple();
        let mut ty_generics_tuple = empty_type_tuple();

        let modify_generics_callback = |g: &mut syn::Generics| {
            let index_after_lifetime_in_generics = g
                .params
                .iter()
                .filter(|arg| matches!(arg, syn::GenericParam::Lifetime(_)))
                .count();
            for f in self.included_fields() {
                if f.ordinal == field.ordinal {
                    ty_generics_tuple.elems.push_value(f.default_type());
                    target_generics_tuple.elems.push_value(f.tuplized_type_ty_param());
                } else {
                    g.params.insert(index_after_lifetime_in_generics, f.generic_ty_param());
                    let generic_argument: syn::Type = f.type_ident();
                    ty_generics_tuple.elems.push_value(generic_argument.clone());
                    target_generics_tuple.elems.push_value(generic_argument);
                }
                ty_generics_tuple.elems.push_punct(Default::default());
                target_generics_tuple.elems.push_punct(Default::default());
            }
        };
        let generics = if !field.used_default_generic_idents.is_empty() {
            self.modify_generics_alter_if_used_default_generic(modify_generics_callback, field)
        } else {
            self.modify_generics(modify_generics_callback)
        };

        let index_after_lifetime_in_generics = target_generics
            .iter()
            .filter(|arg| matches!(arg, syn::GenericArgument::Lifetime(_)))
            .count();
        target_generics.insert(
            index_after_lifetime_in_generics,
            syn::GenericArgument::Type(target_generics_tuple.into()),
        );
        ty_generics.insert(
            index_after_lifetime_in_generics,
            syn::GenericArgument::Type(ty_generics_tuple.into()),
        );
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let doc = match field.builder_attr.setter.doc {
            Some(ref doc) => quote!(#[doc = #doc]),
            None => quote!(),
        };

        // NOTE: both auto_into and strip_option affect `arg_type` and `arg_expr`, but the order of
        // nesting is different so we have to do this little dance.
        let arg_type = if field.builder_attr.setter.strip_option.is_some() && field.builder_attr.setter.transform.is_none() {
            let internal_type = field
                .type_from_inside_option()
                .ok_or_else(|| Error::new_spanned(&field_type, "can't `strip_option` - field is not `Option<...>`"))?;
            internal_type
        } else {
            field_type
        };
        let (arg_type, arg_expr) = if field.builder_attr.setter.auto_into.is_some() {
            (quote!(impl ::core::convert::Into<#arg_type>), quote!(#field_name.into()))
        } else {
            (quote!(#arg_type), quote!(#field_name))
        };

        let fn_generics = {
            let ty_str = format!("{arg_type}");
            let mut generic_params = vec![];
            for (generic_param, regular_expression, default_type) in self.generic_defaults.iter() {
                if default_type.is_some() && regular_expression.is_match(&ty_str) {
                    match generic_param {
                        Left(type_param) => {
                            let mut type_param = type_param.clone();
                            type_param.eq_token = None;
                            type_param.default = None;
                            generic_params.push(quote!(#type_param));
                        }
                        Right(const_param) => {
                            let mut const_param = const_param.clone();
                            const_param.eq_token = None;
                            const_param.default = None;
                            generic_params.push(quote!(#const_param));
                        }
                    }
                }
            }
            if generic_params.is_empty() {
                quote!()
            } else {
                quote!(<#(#generic_params),*>)
            }
        };

        let (param_list, arg_expr) = if field.builder_attr.setter.strip_bool.is_some() {
            (quote!(), quote!(true))
        } else if let Some(transform) = &field.builder_attr.setter.transform {
            let params = transform.params.iter().map(|(pat, ty)| quote!(#pat: #ty));
            let body = &transform.body;
            (quote!(#(#params),*), quote!({ #body }))
        } else if field.builder_attr.setter.strip_option.is_some() {
            (quote!(#field_name: #arg_type), quote!(Some(#arg_expr)))
        } else {
            (quote!(#field_name: #arg_type), arg_expr)
        };

        // repeated field impl cannot exist if field includes use of a default type because of overlapping impls
        let repeated_field_impl = if field.used_default_generic_idents.is_empty() {
            let repeated_fields_error_type_name = syn::Ident::new(
                &format!(
                    "{}_Error_Repeated_field_{}",
                    builder_name,
                    strip_raw_ident_prefix(field_name.to_string())
                ),
                proc_macro2::Span::call_site(),
            );
            let repeated_fields_error_message = format!("Repeated field {}", field_name);
            quote! {
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
            }
        } else {
            quote! {}
        };

        Ok(quote! {
            #[allow(dead_code, non_camel_case_types, missing_docs)]
            impl #impl_generics #builder_name < #( #ty_generics ),* > #where_clause {
                #doc
                pub fn #field_name #fn_generics (self, #param_list) -> #builder_name < #( #target_generics ),* > {
                    let #field_name = (#arg_expr,);
                    let ( #(#descructuring,)* ) = self.fields;
                    #builder_name {
                        fields: ( #(#reconstructing,)* ),
                        phantom: ::core::default::Default::default(),
                    }
                }
            }

            #repeated_field_impl
        })
    }

    pub fn required_field_impl(&self, field: &FieldInfo) -> TokenStream {
        let StructInfo {
            ref name,
            ref builder_name,
            ..
        } = self;

        let FieldInfo {
            name: ref field_name, ..
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
                syn::GenericParam::Lifetime(lifetime_def) => syn::GenericArgument::Lifetime(lifetime_def.lifetime.clone()),
                syn::GenericParam::Const(const_param) => {
                    let ident = &const_param.ident;
                    syn::parse(quote!(#ident).into()).unwrap()
                }
            })
            .collect();
        let mut builder_generics_tuple = empty_type_tuple();
        let generics = self.modify_generics(|g| {
            let index_after_lifetime_in_generics = g
                .params
                .iter()
                .filter(|arg| matches!(arg, syn::GenericParam::Lifetime(_)))
                .count();
            for f in self.included_fields() {
                if f.builder_attr.default.is_some() {
                    // `f` is not mandatory - it does not have it's own fake `build` method, so `field` will need
                    // to warn about missing `field` whether or not `f` is set.
                    assert!(
                        f.ordinal != field.ordinal,
                        "`required_field_impl` called for optional field {}",
                        field.name
                    );
                    g.params.insert(index_after_lifetime_in_generics, f.generic_ty_param());
                    builder_generics_tuple.elems.push_value(f.type_ident());
                } else if f.ordinal < field.ordinal {
                    // Only add a `build` method that warns about missing `field` if `f` is set. If `f` is not set,
                    // `f`'s `build` method will warn, since it appears earlier in the argument list.
                    builder_generics_tuple.elems.push_value(f.tuplized_type_ty_param());
                } else if f.ordinal == field.ordinal {
                    builder_generics_tuple.elems.push_value(f.default_type());
                } else {
                    // `f` appears later in the argument list after `field`, so if they are both missing we will
                    // show a warning for `field` and not for `f` - which means this warning should appear whether
                    // or not `f` is set.
                    g.params.insert(index_after_lifetime_in_generics, f.generic_ty_param());
                    builder_generics_tuple.elems.push_value(f.type_ident());
                }

                builder_generics_tuple.elems.push_punct(Default::default());
            }
        });

        let index_after_lifetime_in_generics = builder_generics
            .iter()
            .filter(|arg| matches!(arg, syn::GenericArgument::Lifetime(_)))
            .count();
        builder_generics.insert(
            index_after_lifetime_in_generics,
            syn::GenericArgument::Type(builder_generics_tuple.into()),
        );
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        let early_build_error_type_name = syn::Ident::new(
            &format!(
                "{}_Error_Missing_required_field_{}",
                builder_name,
                strip_raw_ident_prefix(field_name.to_string())
            ),
            proc_macro2::Span::call_site(),
        );
        let early_build_error_message = format!("Missing required field {}", field_name);

        quote! {
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
        }
    }

    pub fn build_method_impl(&self) -> TokenStream {
        let StructInfo {
            ref name,
            ref builder_name,
            ..
        } = *self;

        let generics = self.modify_generics(|g| {
            let index_after_lifetime_in_generics = g
                .params
                .iter()
                .filter(|arg| matches!(arg, syn::GenericParam::Lifetime(_)))
                .count();
            for field in self.included_fields() {
                if field.builder_attr.default.is_some() {
                    let trait_ref = syn::TraitBound {
                        paren_token: None,
                        lifetimes: None,
                        modifier: syn::TraitBoundModifier::None,
                        path: syn::PathSegment {
                            ident: self.conversion_helper_trait_name.clone(),
                            arguments: syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                                colon2_token: None,
                                lt_token: Default::default(),
                                args: make_punctuated_single(syn::GenericArgument::Type(field.ty.clone())),
                                gt_token: Default::default(),
                            }),
                        }
                        .into(),
                    };
                    let mut generic_param: syn::TypeParam = field.generic_ident.clone().into();
                    generic_param.bounds.push(trait_ref.into());
                    g.params.insert(index_after_lifetime_in_generics, generic_param.into());
                }
            }
        });
        let (impl_generics, _, _) = generics.split_for_impl();

        let (_, ty_generics, where_clause) = self.generics.split_for_impl();

        let modified_ty_generics = modify_types_generics_hack(&ty_generics, |args| {
            args.insert(
                0,
                syn::GenericArgument::Type(
                    type_tuple(self.included_fields().map(|field| {
                        if field.builder_attr.default.is_some() {
                            field.type_ident()
                        } else {
                            field.tuplized_type_ty_param()
                        }
                    }))
                    .into(),
                ),
            );
        });

        let descructuring = self.included_fields().map(|f| f.name);

        let helper_trait_name = &self.conversion_helper_trait_name;
        // The default of a field can refer to earlier-defined fields, which we handle by
        // writing out a bunch of `let` statements first, which can each refer to earlier ones.
        // This means that field ordering may actually be significant, which isn't ideal. We could
        // relax that restriction by calculating a DAG of field default dependencies and
        // reordering based on that, but for now this much simpler thing is a reasonable approach.
        let assignments = self.fields.iter().map(|field| {
            let name = &field.name;
            if let Some(ref default) = field.builder_attr.default {
                if field.builder_attr.setter.skip.is_some() {
                    quote!(let #name = #default;)
                } else if !field.used_default_generic_idents.is_empty() {
                    quote!(let #name = #helper_trait_name::into_value(#name, || None);)
                } else {
                    quote!(let #name = #helper_trait_name::into_value(#name, || Some(#default));)
                }
            } else {
                quote!(let #name = #name.0;)
            }
        });
        let field_names = self.fields.iter().map(|field| field.name);
        let doc = if self.builder_attr.doc {
            if let Some(ref doc) = self.builder_attr.build_method_doc {
                quote!(#[doc = #doc])
            } else {
                // I'd prefer “a” or “an” to “its”, but determining which is grammatically
                // correct is roughly impossible.
                let doc = format!("Finalise the builder and create its [`{}`] instance", name);
                quote!(#[doc = #doc])
            }
        } else {
            quote!()
        };

        let build_method_name = self
            .builder_attr
            .build_method
            .name
            .as_ref()
            .map(|name| quote!(#name))
            .unwrap_or(quote!(build));
        let visibility = self
            .builder_attr
            .build_method
            .vis
            .as_ref()
            .map(|v| quote!(#v))
            .unwrap_or(quote!(pub));
        quote!(
            #[allow(dead_code, non_camel_case_types, missing_docs)]
            impl #impl_generics #builder_name #modified_ty_generics #where_clause {
                #doc
                #[allow(clippy::default_trait_access)]
                #visibility fn #build_method_name(self) -> #name #ty_generics {
                    let ( #(#descructuring,)* ) = self.fields;
                    #( #assignments )*
                    #name {
                        #( #field_names ),*
                    }
                }
            }
        )
    }
}

#[derive(Debug, Default, Clone)]
pub struct BuildMethodSettings {
    pub vis: Option<syn::Visibility>,
    pub name: Option<syn::Expr>,
}
impl BuildMethodSettings {
    fn apply_meta(&mut self, expr: syn::Expr) -> Result<(), Error> {
        match expr {
            syn::Expr::Assign(assign) => {
                let name =
                    expr_to_single_string(&assign.left).ok_or_else(|| Error::new_spanned(&assign.left, "Expected identifier"))?;
                match name.as_str() {
                    "vis" => {
                        if let syn::Expr::Lit(expr_lit) = &*assign.right {
                            if let syn::Lit::Str(ref s) = expr_lit.lit {
                                self.vis = Some(syn::parse_str(&s.value()).expect("invalid visibility found"));
                            }
                        }
                        if self.vis.is_none() {
                            panic!("invalid visibility found")
                        }
                        Ok(())
                    }
                    "name" => {
                        self.name = Some(*assign.right);
                        Ok(())
                    }
                    _ => Err(Error::new_spanned(&assign, format!("Unknown parameter {:?}", name))),
                }
            }
            _ => Err(Error::new_spanned(expr, "Expected (<...>=<...>)")),
        }
    }
}

#[derive(Debug, Default)]
pub struct TypeBuilderAttr {
    /// Whether to show docs for the `TypeBuilder` type (rather than hiding them).
    pub doc: bool,

    /// Customize build method, ex. visibility, name
    pub build_method: BuildMethodSettings,

    /// Docs on the `Type::builder()` method.
    pub builder_method_doc: Option<syn::Expr>,

    /// Docs on the `TypeBuilder` type. Specifying this implies `doc`, but you can just specify
    /// `doc` instead and a default value will be filled in here.
    pub builder_type_doc: Option<syn::Expr>,

    /// Docs on the `TypeBuilder.build()` method. Specifying this implies `doc`, but you can just
    /// specify `doc` instead and a default value will be filled in here.
    pub build_method_doc: Option<syn::Expr>,

    pub field_defaults: FieldBuilderAttr,
}

impl TypeBuilderAttr {
    pub fn new(attrs: &[syn::Attribute]) -> Result<Self, Error> {
        let mut result = Self::default();
        for attr in attrs {
            if path_to_single_string(&attr.path).as_deref() != Some("builder") {
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
                    for expr in body.elems {
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
                let name =
                    expr_to_single_string(&assign.left).ok_or_else(|| Error::new_spanned(&assign.left, "Expected identifier"))?;
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
                    _ => Err(Error::new_spanned(&assign, format!("Unknown parameter {:?}", name))),
                }
            }
            syn::Expr::Path(path) => {
                let name = path_to_single_string(&path.path).ok_or_else(|| Error::new_spanned(&path, "Expected identifier"))?;
                match name.as_str() {
                    "doc" => {
                        self.doc = true;
                        Ok(())
                    }
                    _ => Err(Error::new_spanned(&path, format!("Unknown parameter {:?}", name))),
                }
            }
            syn::Expr::Call(call) => {
                let subsetting_name = if let syn::Expr::Path(path) = &*call.func {
                    path_to_single_string(&path.path)
                } else {
                    None
                }
                .ok_or_else(|| {
                    let call_func = &call.func;
                    let call_func = quote!(#call_func);
                    Error::new_spanned(&call.func, format!("Illegal builder setting group {}", call_func))
                })?;
                match subsetting_name.as_str() {
                    "field_defaults" => {
                        for arg in call.args {
                            self.field_defaults.apply_meta(arg)?;
                        }
                        Ok(())
                    }
                    "build_method" => {
                        for arg in call.args {
                            self.build_method.apply_meta(arg)?;
                        }
                        Ok(())
                    }
                    _ => Err(Error::new_spanned(
                        &call.func,
                        format!("Illegal builder setting group name {}", subsetting_name),
                    )),
                }
            }
            _ => Err(Error::new_spanned(expr, "Expected (<...>=<...>)")),
        }
    }
}

fn format_ident_target_generic_default(ident: &syn::Ident) -> syn::Ident {
    format_ident!("{ident}__")
}
