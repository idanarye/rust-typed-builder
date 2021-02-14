use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use std::fmt::Write;
use syn::{parse::Error, spanned::Spanned};

use crate::field_info::{Configured, ExtendField, FieldBuilderAttr, FieldInfo, SetterSettings};
use crate::util::{
    empty_type, empty_type_tuple, expr_to_single_string, make_punctuated_single, path_to_single_string, strip_raw_ident_prefix,
    try_modify_types_generics_hack, type_tuple,
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

    pub fn new(ast: &'a syn::DeriveInput, fields: impl Iterator<Item = &'a syn::Field>) -> Result<StructInfo<'a>, Error> {
        let builder_attr = TypeBuilderAttr::new(&ast.attrs)?;
        let builder_name = strip_raw_ident_prefix(format!("{}Builder", ast.ident));
        Ok(StructInfo {
            vis: &ast.vis,
            name: &ast.ident,
            generics: &ast.generics,
            fields: fields
                .enumerate()
                .map(|(i, f)| FieldInfo::new(i, f, builder_attr.field_defaults.clone()))
                .collect::<Result<_, _>>()?,
            builder_attr,
            builder_name: syn::Ident::new(&builder_name, proc_macro2::Span::call_site()),
            conversion_helper_trait_name: syn::Ident::new(&format!("{}_Optional", builder_name), proc_macro2::Span::call_site()),
            core: syn::Ident::new(&format!("{}_core", builder_name), proc_macro2::Span::call_site()),
        })
    }

    fn modify_generics<F: FnMut(&mut syn::Generics)>(&self, mut mutator: F) -> syn::Generics {
        let mut generics = self.generics.clone();
        mutator(&mut generics);
        generics
    }

    fn try_modify_generics<F: FnMut(&mut syn::Generics) -> Result<(), Error>>(
        &self,
        mut mutator: F,
    ) -> Result<syn::Generics, Error> {
        let mut generics = self.generics.clone();
        mutator(&mut generics)?;
        Ok(generics)
    }

    pub fn builder_creation_impl(&self) -> Result<TokenStream, Error> {
        let StructInfo {
            ref vis,
            ref name,
            ref builder_name,
            ..
        } = *self;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let all_fields_param =
            syn::GenericParam::Type(syn::Ident::new("TypedBuilderFields", proc_macro2::Span::call_site()).into());
        let b_generics = self.modify_generics(|g| {
            g.params.insert(0, all_fields_param.clone());
        });
        let empties_tuple = type_tuple(self.included_fields().map(|_| empty_type()));
        let generics_with_empty = try_modify_types_generics_hack(&ty_generics, |args| {
            args.insert(0, syn::GenericArgument::Type(empties_tuple.clone().into()));
            Ok(())
        })?;
        let phantom_generics = self.generics.params.iter().map(|param| {
            let t = match param {
                syn::GenericParam::Lifetime(lifetime) => {
                    let lifetime = &lifetime.lifetime;
                    quote!(&#lifetime ())
                }
                syn::GenericParam::Type(ty) => {
                    let ty = &ty.ident;
                    quote!(#ty)
                }
                syn::GenericParam::Const(cnst) => {
                    let cnst = &cnst.ident;
                    quote!(#cnst)
                }
            };
            quote!(core::marker::PhantomData<#t>)
        });
        let builder_method_doc = match self.builder_attr.builder_method_doc {
            Some(ref doc) => quote!(#doc),
            None => {
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
            }
        };
        let builder_type_doc = if self.builder_attr.doc {
            match self.builder_attr.builder_type_doc {
                Some(ref doc) => quote!(#[doc = #doc]),
                None => {
                    let doc = format!(
                        "Builder for [`{name}`] instances.\n\nSee [`{name}::builder()`] for more info.",
                        name = name
                    );
                    quote!(#[doc = #doc])
                }
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
                #[allow(dead_code)]
                #vis fn builder() -> #builder_name #generics_with_empty {
                    #builder_name {
                        fields: #empties_tuple,
                        _phantom: core::default::Default::default(),
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

            impl #b_generics_impl Clone for #builder_name #b_generics_ty #b_generics_where {
                fn clone(&self) -> Self {
                    Self {
                        fields: self.fields.clone(),
                        _phantom: Default::default(),
                    }
                }
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
                fn into_some_or_else<F: FnOnce() -> ::core::option::Option<T>>(self, default: F) -> ::core::option::Option<T>;
            }

            impl<T> #trait_name<T> for () {
                fn into_value<F: FnOnce() -> T>(self, default: F) -> T {
                    default()
                }
                fn into_some_or_else<F: FnOnce() -> ::core::option::Option<T>>(self, default: F) -> ::core::option::Option<T> {
                    default()
                }
            }

            impl<T> #trait_name<T> for (T,) {
                fn into_value<F: FnOnce() -> T>(self, _: F) -> T {
                    self.0
                }
                fn into_some_or_else<F: FnOnce() -> ::core::option::Option<T>>(self, _: F) -> ::core::option::Option<T> {
                    ::core::option::Option::Some(self.0)
                }
            }
        })
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
                syn::GenericParam::Lifetime(lifetime_def) => syn::GenericArgument::Lifetime(lifetime_def.lifetime.clone()),
                syn::GenericParam::Const(const_param) => {
                    let ident = const_param.ident.clone();
                    syn::parse(quote!(#ident).into()).unwrap()
                }
            })
            .collect();
        let mut target_generics_tuple = empty_type_tuple();
        let mut ty_generics_tuple = empty_type_tuple();
        let generics = self.try_modify_generics(|g| {
            for f in self.included_fields() {
                if f.ordinal == field.ordinal {
                    ty_generics_tuple.elems.push_value(empty_type());
                    target_generics_tuple.elems.push_value(f.tuplized_type_ty_param()?);
                } else {
                    g.params.push(f.generic_ty_param());
                    let generic_argument: syn::Type = f.type_ident();
                    ty_generics_tuple.elems.push_value(generic_argument.clone());
                    target_generics_tuple.elems.push_value(generic_argument);
                }
                ty_generics_tuple.elems.push_punct(Default::default());
                target_generics_tuple.elems.push_punct(Default::default());
            }
            Ok(())
        })?;
        let mut target_generics = ty_generics.clone();

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

        if let Some(ExtendField {
            keyword_span,
            from_first,
            from_iter,
            item_name: _,
        }) = &field.builder_attr.setter.extend
        {
            // Changing the builder field type entirely here (instead of just the argument) means there
            // won't be a need to unwrap an `Option` in every repeat field setter call.
            let field_type = if field.builder_attr.setter.strip_option {
                let internal_type = field.type_from_inside_option()?;
                internal_type
            } else {
                field_type
            };

            let item_name = syn::Ident::new(&field.item_name(), field.name.span());

            let descructuring = descructuring.collect::<Vec<_>>();
            let reconstructing = reconstructing.collect::<Vec<_>>();

            let from_first = from_first
                .clone()
                .into_configured(|span| {
                    syn::parse2(quote_spanned! {span.resolved_at(Span::mixed_site())=>
                        |first| ::core::iter::once(first).collect()
                    })
                    .expect("extend from_first collect")
                })
                .map({
                    |Configured { keyword_span, value }| {
                    quote_spanned! {keyword_span.resolved_at(Span::mixed_site())=>
                        #doc
                        pub fn #item_name(self, #item_name: <#field_type as ::core::iter::IntoIterator>::Item) -> #builder_name<#(#target_generics),*> {
                            let from_item = #value;
                            let _: &dyn FnOnce(<#field_type as ::core::iter::IntoIterator>::Item) -> #field_type = &from_item;
                            let #field_name: (#field_type,) = ((from_item)(#item_name),);
                            let ( #(#descructuring,)* ) = self.fields;
                            #builder_name {
                                fields: ( #(#reconstructing,)* ),
                                _phantom: self._phantom,
                            }
                        }
                    }
                }});
            let from_iter = from_iter.clone()
            .into_configured(|span| {
                syn::parse2(quote_spanned!(span.resolved_at(Span::mixed_site())=> |iter| iter.collect()))
                    .expect("extend from_iter collect")
            }).map(|Configured { keyword_span, value }| {
                // A plain closure like `|iter| iter.collect()` would require a generic type annotation on the parameter type,
                // which unfortunately isn't possible. This means the closure is repackaged as nested function.
                let syn::ExprClosure {
                    attrs,
                    asyncness,
                    movability,
                    capture: _, // No locals are in scope here, so this doesn't matter.
                    or1_token,
                    inputs,
                    or2_token,
                    output,
                    body,
                } = value;

                // A few error messages can be made much more direct on the way:

                if let Some(asyncness) = asyncness {
                    return Err(Error::new_spanned(asyncness, "Expected synchronous closure (Remove `async`)"))
                }

                if let Some(movability) = movability {
                    return Err(Error::new_spanned(movability, "Expected movable closure (Remove `static`)"))
                }

                let input = match inputs.len() {
                    0 => return Err(Error::new_spanned(quote!(#or1_token #or2_token), "Expected one argument")),
                    1 => inputs.into_iter().next().unwrap(),
                    _ => return Err(Error::new_spanned(inputs, "Expected only one argument")),
                };

                if let syn::Pat::Type(syn::PatType { colon_token, ty, .. }) = input {
                    return Err(Error::new_spanned(quote!(#colon_token #ty), "Did not expect argument type, since it is set explicitly by the macro"))
                }

                // This is a bit roundabout. In short:
                // Iff the closure has an explicit return type, and this return type mismatches the builder field type,
                // then the "mismatched types" error is located on that explicit closure type rather than on `from_iter`.
                let body = match output {
                    syn::ReturnType::Default => {
                        // Transforming this like below would bread the `unused_braces` warning on the closure body.
                        body.into_token_stream()
                    }
                    syn::ReturnType::Type(arrow, ty) => {
                        let colon = syn::Token![:](arrow.span());
                        quote_spanned!{ty.span()=>
                            let output#colon #ty = #body;
                            output
                        }
                    }
                };

                let generics = self.modify_generics(|g| {
                    g.params.push(syn::parse2(quote_spanned!{keyword_span.resolved_at(Span::mixed_site())=>
                        __x: ::core::iter::Iterator<Item = <#field_type as ::core::iter::IntoIterator>::Item>
                    }).expect("extend from_iter __x"))
                });
                let where_clause = generics.where_clause.as_ref();
                let generics_for_function_generics = self.modify_generics(|g| {
                    g.params.push(syn::GenericParam::Type(syn::TypeParam {
                        attrs: vec![],
                        ident: syn::Ident::new("_", keyword_span.resolved_at(Span::mixed_site())),
                        colon_token: None,
                        bounds: syn::punctuated::Punctuated::default(),
                        eq_token: None,
                        default: None,
                    }))
                });
                let (_, type_generics, _) = generics_for_function_generics.split_for_impl();
                let type_generics = type_generics.into_token_stream();
                let function_generics = if type_generics.is_empty() {
                    type_generics
                } else {
                    quote_spanned!(keyword_span.resolved_at(Span::mixed_site())=> ::#type_generics)
                };

                Ok(quote_spanned! {keyword_span.resolved_at(Span::mixed_site())=>
                    #doc
                    pub fn #field_name(self, #field_name: impl ::core::iter::IntoIterator<Item = <#field_type as ::core::iter::IntoIterator>::Item>) -> #builder_name<#(#target_generics),*> {
                        #(#attrs)*
                        #asyncness fn from_iter#generics(#input: __x) -> #field_type #where_clause {
                            #body
                        }

                        let #field_name: (#field_type,) = (from_iter#function_generics(#field_name.into_iter()),);
                        let ( #(#descructuring,)* ) = self.fields;
                        #builder_name {
                            fields: ( #(#reconstructing,)* ),
                            _phantom: self._phantom,
                        }
                    }
                })
            })
            .transpose()?;

            Ok(quote_spanned! {keyword_span.resolved_at(Span::mixed_site())=>
                #[allow(non_camel_case_types, missing_docs)]
                impl #impl_generics #builder_name<#(#ty_generics),*> #where_clause {
                    #from_first
                    #from_iter
                }

                impl #impl_generics #builder_name<#(#target_generics),*> #where_clause {
                    #doc
                    pub fn #item_name<Item>(self, #item_name: Item) -> #builder_name<#(#target_generics),*>
                    where
                        #field_type: ::core::iter::Extend<Item>,
                    {
                        let ( #(#reconstructing,)* ) = self.fields;
                        let mut #field_name = #field_name;
                        ::core::iter::Extend::extend(&mut #field_name.0, ::core::iter::once(#item_name));
                        #builder_name {
                            fields: ( #(#reconstructing,)* ),
                            _phantom: self._phantom,
                        }
                    }

                    #doc
                    pub fn #field_name<Iter, Item>(self, #field_name: Iter) -> #builder_name<#(#target_generics),*>
                    where
                        Iter: ::core::iter::IntoIterator<Item = Item>,
                        #field_type: ::core::iter::Extend<Item>,
                    {
                        let items = #field_name;
                        let ( #(#reconstructing,)* ) = self.fields;
                        let mut #field_name = #field_name;
                        ::core::iter::Extend::extend(&mut #field_name.0, items);
                        #builder_name {
                            fields: ( #(#reconstructing,)* ),
                            _phantom: self._phantom,
                        }
                    }
                }
            })
        } else {
            //Not `extend`.

            // NOTE: both auto_into and strip_option affect `arg_type` and `arg_expr`, but the order of
            // nesting is different so we have to do this little dance.
            let arg_type = if field.builder_attr.setter.strip_option {
                field.type_from_inside_option()?
            } else {
                field_type
            };
            let (arg_type, arg_expr) = if field.builder_attr.setter.auto_into {
                (quote!(impl core::convert::Into<#arg_type>), quote!(#field_name.into()))
            } else {
                (quote!(#arg_type), quote!(#field_name))
            };
            let arg_expr = if field.builder_attr.setter.strip_option {
                quote!(Some(#arg_expr))
            } else {
                arg_expr
            };

            let repeated_fields_error_type_name = syn::Ident::new(
                &format!(
                    "{}_Error_Repeated_field_{}",
                    builder_name,
                    strip_raw_ident_prefix(field_name.to_string())
                ),
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
    }

    pub fn required_field_impl(&self, field: &FieldInfo) -> Result<TokenStream, Error> {
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
        let generics = self.try_modify_generics(|g| {
            for f in self.included_fields() {
                if f.builder_attr.default.is_some() {
                    // `f` is not mandatory - it does not have it's own fake `build` method, so `field` will need
                    // to warn about missing `field` whether or not `f` is set.
                    assert!(
                        f.ordinal != field.ordinal,
                        "`required_field_impl` called for optional field {}",
                        field.name
                    );
                    g.params.push(f.generic_ty_param());
                    builder_generics_tuple.elems.push_value(f.type_ident());
                } else if f.ordinal < field.ordinal {
                    // Only add a `build` method that warns about missing `field` if `f` is set. If `f` is not set,
                    // `f`'s `build` method will warn, since it appears earlier in the argument list.
                    builder_generics_tuple.elems.push_value(f.tuplized_type_ty_param()?);
                } else if f.ordinal == field.ordinal {
                    builder_generics_tuple.elems.push_value(empty_type());
                } else {
                    // `f` appears later in the argument list after `field`, so if they are both missing we will
                    // show a warning for `field` and not for `f` - which means this warning should appear whether
                    // or not `f` is set.
                    g.params.push(f.generic_ty_param());
                    builder_generics_tuple.elems.push_value(f.type_ident());
                }

                builder_generics_tuple.elems.push_punct(Default::default());
            }
            Ok(())
        })?;

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
        let mut early_build_error_message = format!("Missing required field {}", field_name);
        if field.builder_attr.setter.extend.is_some() {
            write!(&mut early_build_error_message, " (single item setter: {})", field.item_name()).unwrap();
        }

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

    pub fn build_method_impl(&self) -> Result<TokenStream, Error> {
        let StructInfo {
            ref name,
            ref builder_name,
            ..
        } = *self;

        let generics = self.try_modify_generics(|g| {
            for field in self.included_fields() {
                let strip_option_extends = matches!(
                    field.builder_attr.setter,
                    SetterSettings {
                        extend: Some(_),
                        strip_option: true,
                        ..
                    }
                );

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
                                args: make_punctuated_single(syn::GenericArgument::Type(if strip_option_extends {
                                    field.type_from_inside_option()?.clone()
                                } else {
                                    field.ty.clone()
                                })),
                                gt_token: Default::default(),
                            }),
                        }
                        .into(),
                    };
                    let mut generic_param: syn::TypeParam = field.generic_ident.clone().into();
                    generic_param.bounds.push(trait_ref.into());
                    g.params.push(generic_param.into());
                }
            }
            Ok(())
        })?;
        let (impl_generics, _, _) = generics.split_for_impl();

        let (_, ty_generics, where_clause) = self.generics.split_for_impl();

        let modified_ty_generics = try_modify_types_generics_hack(&ty_generics, |args| {
            args.insert(
                0,
                syn::GenericArgument::Type(
                    type_tuple(
                        self.included_fields()
                            .map(|field| {
                                if field.builder_attr.default.is_some() {
                                    Ok(field.type_ident())
                                } else {
                                    field.tuplized_type_ty_param()
                                }
                            })
                            .collect::<Result<Vec<_>, _>>()?
                            .into_iter(),
                    )
                    .into(),
                ),
            );
            Ok(())
        })?;

        let descructuring = self.included_fields().map(|f| f.name);

        let helper_trait_name = &self.conversion_helper_trait_name;
        // The default of a field can refer to earlier-defined fields, which we handle by
        // writing out a bunch of `let` statements first, which can each refer to earlier ones.
        // This means that field ordering may actually be significant, which isn’t ideal. We could
        // relax that restriction by calculating a DAG of field default dependencies and
        // reordering based on that, but for now this much simpler thing is a reasonable approach.
        let assignments = self.fields.iter().map(|field| {
            let name = field.name;
            let wrap_some = matches!(
                field.builder_attr.setter,
                SetterSettings {
                    extend: Some(_),
                    strip_option: true,
                    ..
                }
            );
            if let Some(ref default) = field.builder_attr.default {
                if field.builder_attr.setter.skip {
                    quote!(let #name = #default;)
                } else if wrap_some {
                    quote!(let #name = #helper_trait_name::into_some_or_else(#name, || #default);)
                } else {
                    quote!(let #name = #helper_trait_name::into_value(#name, || #default);)
                }
            } else if wrap_some {
                quote!(let #name = ::core::option::Option::Some(#name.0);)
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
        Ok(quote!(
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
        ))
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

    pub field_defaults: FieldBuilderAttr,
}

impl TypeBuilderAttr {
    pub fn new(attrs: &[syn::Attribute]) -> Result<TypeBuilderAttr, Error> {
        let mut result = TypeBuilderAttr::default();
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
