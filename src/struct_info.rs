use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Error;

use crate::field_info::{FieldBuilderAttr, FieldInfo};
use crate::util::{
    empty_type, empty_type_tuple, expr_to_single_string, first_visibility, make_punctuated_single, modify_types_generics_hack,
    path_to_single_string, public_visibility, strip_raw_ident_prefix, type_tuple,
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
        self.fields.iter().filter(|f| f.builder_attr.setter.skip.is_none())
    }

    pub fn new(ast: &'a syn::DeriveInput, fields: impl Iterator<Item = &'a syn::Field>) -> Result<StructInfo<'a>, Error> {
        let builder_attr = TypeBuilderAttr::new(&ast.attrs)?;
        let builder_name = builder_attr
            .builder_type
            .get_name()
            .map(|name| strip_raw_ident_prefix(name.to_string()))
            .unwrap_or_else(|| strip_raw_ident_prefix(format!("{}Builder", ast.ident)));
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

    pub fn builder_creation_impl(&self) -> Result<TokenStream, Error> {
        let StructInfo {
            vis,
            ref name,
            ref builder_name,
            ..
        } = *self;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let empties_tuple = type_tuple(self.included_fields().map(|_| empty_type()));
        let mut all_fields_param_type: syn::TypeParam =
            syn::Ident::new("TypedBuilderFields", proc_macro2::Span::call_site()).into();
        let all_fields_param = syn::GenericParam::Type(all_fields_param_type.clone());
        all_fields_param_type.default = Some(syn::Type::Tuple(empties_tuple.clone()));
        let b_generics = {
            let mut generics = self.generics.clone();
            generics.params.push(syn::GenericParam::Type(all_fields_param_type));
            generics
        };
        let generics_with_empty = modify_types_generics_hack(&ty_generics, |args| {
            args.push(syn::GenericArgument::Type(empties_tuple.clone().into()));
        });
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

        let builder_method_name = self.builder_attr.builder_method.get_name().unwrap_or_else(|| quote!(builder));
        let builder_method_visibility = first_visibility(&[
            self.builder_attr.builder_method.vis.as_ref(),
            self.builder_attr.builder_type.vis.as_ref(),
            Some(vis),
        ]);
        let builder_method_doc = self.builder_attr.builder_method.get_doc_or(|| {
            format!(
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
            )
        });

        let builder_type_visibility = first_visibility(&[self.builder_attr.builder_type.vis.as_ref(), Some(vis)]);
        let builder_type_doc = if self.builder_attr.doc {
            self.builder_attr.builder_type.get_doc_or(|| {
                format!(
                    "Builder for [`{name}`] instances.\n\nSee [`{name}::builder()`] for more info.",
                    name = name
                )
            })
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
                #builder_method_doc
                #[allow(dead_code, clippy::default_trait_access)]
                #builder_method_visibility fn #builder_method_name() -> #builder_name #generics_with_empty {
                    #builder_name {
                        fields: #empties_tuple,
                        phantom: ::core::default::Default::default(),
                    }
                }
            }

            #[must_use]
            #builder_type_doc
            #[allow(dead_code, non_camel_case_types, non_snake_case)]
            #builder_type_visibility struct #builder_name #b_generics {
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
        let generics = {
            let mut generics = self.generics.clone();
            for f in self.included_fields() {
                if f.ordinal == field.ordinal {
                    ty_generics_tuple.elems.push_value(empty_type());
                    target_generics_tuple.elems.push_value(f.tuplized_type_ty_param());
                } else {
                    generics.params.push(f.generic_ty_param());
                    let generic_argument: syn::Type = f.type_ident();
                    ty_generics_tuple.elems.push_value(generic_argument.clone());
                    target_generics_tuple.elems.push_value(generic_argument);
                }
                ty_generics_tuple.elems.push_punct(Default::default());
                target_generics_tuple.elems.push_punct(Default::default());
            }
            generics
        };
        let mut target_generics = ty_generics.clone();
        target_generics.push(syn::GenericArgument::Type(target_generics_tuple.into()));
        ty_generics.push(syn::GenericArgument::Type(ty_generics_tuple.into()));
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let doc = match field.builder_attr.setter.doc {
            Some(ref doc) => quote!(#[doc = #doc]),
            None => quote!(),
        };

        // NOTE: both auto_into and strip_option affect `arg_type` and `arg_expr`, but the order of
        // nesting is different so we have to do this little dance.
        let arg_type = if field.builder_attr.setter.strip_option.is_some() && field.builder_attr.setter.transform.is_none() {
            field
                .type_from_inside_option()
                .ok_or_else(|| Error::new_spanned(field_type, "can't `strip_option` - field is not `Option<...>`"))?
        } else {
            field_type
        };
        let (arg_type, arg_expr) = if field.builder_attr.setter.auto_into.is_some() {
            (quote!(impl ::core::convert::Into<#arg_type>), quote!(#field_name.into()))
        } else {
            (quote!(#arg_type), quote!(#field_name))
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
                pub fn #field_name (self, #param_list) -> #builder_name < #( #target_generics ),* > {
                    let #field_name = (#arg_expr,);
                    let ( #(#descructuring,)* ) = self.fields;
                    #builder_name {
                        fields: ( #(#reconstructing,)* ),
                        phantom: self.phantom,
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
        let generics = {
            let mut generics = self.generics.clone();
            for f in self.included_fields() {
                if f.builder_attr.default.is_some() {
                    // `f` is not mandatory - it does not have it's own fake `build` method, so `field` will need
                    // to warn about missing `field` whether or not `f` is set.
                    assert!(
                        f.ordinal != field.ordinal,
                        "`required_field_impl` called for optional field {}",
                        field.name
                    );
                    generics.params.push(f.generic_ty_param());
                    builder_generics_tuple.elems.push_value(f.type_ident());
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
                    generics.params.push(f.generic_ty_param());
                    builder_generics_tuple.elems.push_value(f.type_ident());
                }

                builder_generics_tuple.elems.push_punct(Default::default());
            }
            generics
        };

        builder_generics.push(syn::GenericArgument::Type(builder_generics_tuple.into()));
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

        let build_method_name = self.build_method_name();
        let build_method_visibility = self.build_method_visibility();

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
                #build_method_visibility fn #build_method_name(self, _: #early_build_error_type_name) -> #name #ty_generics {
                    panic!();
                }
            }
        }
    }

    fn build_method_name(&self) -> TokenStream {
        self.builder_attr.build_method.common.get_name().unwrap_or(quote!(build))
    }

    fn build_method_visibility(&self) -> TokenStream {
        first_visibility(&[self.builder_attr.build_method.common.vis.as_ref(), Some(&public_visibility())])
    }

    pub fn build_method_impl(&self) -> TokenStream {
        let StructInfo {
            ref name,
            ref builder_name,
            ..
        } = *self;

        let generics = {
            let mut generics = self.generics.clone();
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
                    generics.params.push(generic_param.into());
                }
            }
            generics
        };
        let (impl_generics, _, _) = generics.split_for_impl();

        let (_, ty_generics, where_clause) = self.generics.split_for_impl();

        let modified_ty_generics = modify_types_generics_hack(&ty_generics, |args| {
            args.push(syn::GenericArgument::Type(
                type_tuple(self.included_fields().map(|field| {
                    if field.builder_attr.default.is_some() {
                        field.type_ident()
                    } else {
                        field.tuplized_type_ty_param()
                    }
                }))
                .into(),
            ));
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
                } else {
                    quote!(let #name = #helper_trait_name::into_value(#name, || #default);)
                }
            } else {
                quote!(let #name = #name.0;)
            }
        });
        let field_names = self.fields.iter().map(|field| field.name);

        let build_method_name = self.build_method_name();
        let build_method_visibility = self.build_method_visibility();
        let build_method_doc = if self.builder_attr.doc {
            self.builder_attr
                .build_method
                .common
                .get_doc_or(|| format!("Finalise the builder and create its [`{}`] instance", name))
        } else {
            quote!()
        };
        let (build_method_generic, output_type, build_method_where_clause) = match &self.builder_attr.build_method.into {
            IntoSetting::NoConversion => (None, quote!(#name #ty_generics), None),
            IntoSetting::GenericConversion => (
                Some(quote!(<__R>)),
                quote!(__R),
                Some(quote!(where #name #ty_generics: Into<__R>)),
            ),
            IntoSetting::TypeConversionToSpecificType(into) => (None, quote!(#into), None),
        };

        quote!(
            #[allow(dead_code, non_camel_case_types, missing_docs)]
            impl #impl_generics #builder_name #modified_ty_generics #where_clause {
                #build_method_doc
                #[allow(clippy::default_trait_access)]
                #build_method_visibility fn #build_method_name #build_method_generic (self) -> #output_type #build_method_where_clause {
                    let ( #(#descructuring,)* ) = self.fields;
                    #( #assignments )*
                    #name {
                        #( #field_names ),*
                    }.into()
                }
            }
        )
    }
}

#[derive(Debug, Default, Clone)]
pub struct CommonDeclarationSettings {
    pub vis: Option<syn::Visibility>,
    pub name: Option<syn::Expr>,
    pub doc: Option<syn::Expr>,
}
impl CommonDeclarationSettings {
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
                    "doc" => {
                        self.doc = Some(*assign.right);
                        Ok(())
                    }
                    _ => Err(Error::new_spanned(&assign, format!("Unknown parameter {:?}", name))),
                }
            }
            _ => Err(Error::new_spanned(expr, "Expected (<...>=<...>)")),
        }
    }

    fn get_name(&self) -> Option<TokenStream> {
        self.name.as_ref().map(|name| quote!(#name))
    }

    fn get_doc_or(&self, gen_doc: impl FnOnce() -> String) -> TokenStream {
        if let Some(ref doc) = self.doc {
            quote!(#[doc = #doc])
        } else {
            let doc = gen_doc();
            quote!(#[doc = #doc])
        }
    }
}

/// Setting of the `into` argument.
#[derive(Debug, Clone)]
pub enum IntoSetting {
    /// Do not run any conversion on the built value.
    NoConversion,
    /// Convert the build value into the generic parameter passed to the `build` method.
    GenericConversion,
    /// Convert the build value into a specific type specified in the attribute.
    TypeConversionToSpecificType(syn::ExprPath),
}

impl Default for IntoSetting {
    fn default() -> Self {
        Self::NoConversion
    }
}

#[derive(Debug, Default, Clone)]
pub struct BuildMethodSettings {
    pub common: CommonDeclarationSettings,

    /// Whether to convert the builded type into another while finishing the build.
    pub into: IntoSetting,
}

impl BuildMethodSettings {
    fn apply_meta(&mut self, expr: syn::Expr) -> Result<(), Error> {
        match &expr {
            syn::Expr::Assign(assign) => {
                let name =
                    expr_to_single_string(&assign.left).ok_or_else(|| Error::new_spanned(&assign.left, "Expected identifier"))?;
                if name.as_str() == "into" {
                    let expr_path = match assign.right.as_ref() {
                        syn::Expr::Path(expr_path) => expr_path,
                        _ => return Err(Error::new_spanned(&assign.right, "Expected path expression type")),
                    };
                    self.into = IntoSetting::TypeConversionToSpecificType(expr_path.clone());
                    Ok(())
                } else {
                    self.common.apply_meta(expr)
                }
            }
            syn::Expr::Path(path) => {
                let name = path_to_single_string(&path.path).ok_or_else(|| Error::new_spanned(path, "Expected identifier"))?;
                if name.as_str() == "into" {
                    self.into = IntoSetting::GenericConversion;
                    Ok(())
                } else {
                    self.common.apply_meta(expr)
                }
            }
            _ => self.common.apply_meta(expr),
        }
    }
}

#[derive(Debug, Default)]
pub struct TypeBuilderAttr {
    /// Whether to show docs for the `TypeBuilder` type (rather than hiding them).
    pub doc: bool,

    /// Customize builder method, ex. visibility, name
    pub builder_method: CommonDeclarationSettings,

    /// Customize builder type, ex. visibility, name
    pub builder_type: CommonDeclarationSettings,

    /// Customize build method, ex. visibility, name
    pub build_method: BuildMethodSettings,

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

        if result.builder_type.doc.is_some() || result.build_method.common.doc.is_some() {
            result.doc = true;
        }

        Ok(result)
    }

    fn apply_meta(&mut self, expr: syn::Expr) -> Result<(), Error> {
        match expr {
            syn::Expr::Assign(assign) => {
                let name =
                    expr_to_single_string(&assign.left).ok_or_else(|| Error::new_spanned(&assign.left, "Expected identifier"))?;

                let gen_structure_depracation_error = |put_under: &str, new_name: &str| {
                    Error::new_spanned(
                        &assign.left,
                        format!(
                            "`{} = \"...\"` is deprecated - use `{}({} = \"...\")` instead",
                            name, put_under, new_name
                        ),
                    )
                };
                match name.as_str() {
                    "builder_method_doc" => Err(gen_structure_depracation_error("builder_method", "doc")),
                    "builder_type_doc" => Err(gen_structure_depracation_error("builder_type", "doc")),
                    "build_method_doc" => Err(gen_structure_depracation_error("build_method", "doc")),
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
                    "builder_method" => {
                        for arg in call.args {
                            self.builder_method.apply_meta(arg)?;
                        }
                        Ok(())
                    }
                    "builder_type" => {
                        for arg in call.args {
                            self.builder_type.apply_meta(arg)?;
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
