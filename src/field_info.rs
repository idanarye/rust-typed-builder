use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::Error;
use syn::spanned::Spanned;

use crate::util::{expr_to_single_string, ident_to_type, path_to_single_string, strip_raw_ident_prefix};

#[derive(Debug)]
pub struct FieldInfo<'a> {
    pub ordinal: usize,
    pub name: &'a syn::Ident,
    pub generic_ident: syn::Ident,
    pub ty: &'a syn::Type,
    pub builder_attr: FieldBuilderAttr,
}

impl<'a> FieldInfo<'a> {
    pub fn new(ordinal: usize, field: &syn::Field, field_defaults: FieldBuilderAttr) -> Result<FieldInfo, Error> {
        if let Some(ref name) = field.ident {
            FieldInfo {
                ordinal,
                name: &name,
                generic_ident: syn::Ident::new(&format!("__{}", strip_raw_ident_prefix(name.to_string())), Span::call_site()),
                ty: &field.ty,
                builder_attr: field_defaults.with(&field.attrs)?,
            }
            .post_process()
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
        }
        .into()
    }

    pub fn type_from_inside_option(&self) -> Option<&syn::Type> {
        let path = if let syn::Type::Path(type_path) = self.ty {
            if type_path.qself.is_some() {
                return None;
            } else {
                &type_path.path
            }
        } else {
            return None;
        };
        let segment = path.segments.last()?;
        if segment.ident != "Option" {
            return None;
        }
        let generic_params = if let syn::PathArguments::AngleBracketed(generic_params) = &segment.arguments {
            generic_params
        } else {
            return None;
        };
        if let syn::GenericArgument::Type(ty) = generic_params.args.first()? {
            Some(ty)
        } else {
            None
        }
    }

    fn post_process(mut self) -> Result<Self, Error> {
        if let Some(ref strip_bool_span) = self.builder_attr.setter.strip_bool {
            if let Some(default_span) = self.builder_attr.default.as_ref().map(|d| d.span()) {
                let mut error = Error::new(
                    *strip_bool_span,
                    "cannot set both strip_bool and default - default is assumed to be false",
                );
                error.combine(Error::new(default_span, "default set here"));
                return Err(error);
            }
            self.builder_attr.default = Some(syn::Expr::Lit(syn::ExprLit {
                attrs: Default::default(),
                lit: syn::Lit::Bool(syn::LitBool {
                    value: false,
                    span: *strip_bool_span,
                }),
            }));
        }
        Ok(self)
    }
}

#[derive(Debug, Default, Clone)]
pub struct FieldBuilderAttr {
    pub default: Option<syn::Expr>,
    pub setter: SetterSettings,
}

#[derive(Debug, Default, Clone)]
pub struct SetterSettings {
    pub doc: Option<syn::Expr>,
    pub skip: Option<Span>,
    pub auto_into: Option<Span>,
    pub strip_option: Option<Span>,
    pub strip_bool: Option<Span>,
    pub transform: Option<Transform>,
}

impl FieldBuilderAttr {
    pub fn with(mut self, attrs: &[syn::Attribute]) -> Result<Self, Error> {
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
                    self.apply_meta(*body.expr)?;
                }
                syn::Expr::Tuple(body) => {
                    for expr in body.elems.into_iter() {
                        self.apply_meta(expr)?;
                    }
                }
                _ => {
                    return Err(Error::new_spanned(attr.tokens.clone(), "Expected (<...>)"));
                }
            }
        }

        self.inter_fields_conflicts()?;

        Ok(self)
    }

    pub fn apply_meta(&mut self, expr: syn::Expr) -> Result<(), Error> {
        match expr {
            syn::Expr::Assign(assign) => {
                let name =
                    expr_to_single_string(&assign.left).ok_or_else(|| Error::new_spanned(&assign.left, "Expected identifier"))?;
                match name.as_str() {
                    "default" => {
                        self.default = Some(*assign.right);
                        Ok(())
                    }
                    "default_code" => {
                        if let syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(code),
                            ..
                        }) = *assign.right
                        {
                            use std::str::FromStr;
                            let tokenized_code = TokenStream::from_str(&code.value())?;
                            self.default =
                                Some(syn::parse(tokenized_code.into()).map_err(|e| Error::new_spanned(code, format!("{}", e)))?);
                        } else {
                            return Err(Error::new_spanned(assign.right, "Expected string"));
                        }
                        Ok(())
                    }
                    _ => Err(Error::new_spanned(&assign, format!("Unknown parameter {:?}", name))),
                }
            }
            syn::Expr::Path(path) => {
                let name = path_to_single_string(&path.path).ok_or_else(|| Error::new_spanned(&path, "Expected identifier"))?;
                match name.as_str() {
                    "default" => {
                        self.default = Some(syn::parse(quote!(::core::default::Default::default()).into()).unwrap());
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
                match subsetting_name.as_ref() {
                    "setter" => {
                        for arg in call.args {
                            self.setter.apply_meta(arg)?;
                        }
                        Ok(())
                    }
                    _ => Err(Error::new_spanned(
                        &call.func,
                        format!("Illegal builder setting group name {}", subsetting_name),
                    )),
                }
            }
            syn::Expr::Unary(syn::ExprUnary {
                op: syn::UnOp::Not(_),
                expr,
                ..
            }) => {
                if let syn::Expr::Path(path) = *expr {
                    let name =
                        path_to_single_string(&path.path).ok_or_else(|| Error::new_spanned(&path, "Expected identifier"))?;
                    match name.as_str() {
                        "default" => {
                            self.default = None;
                            Ok(())
                        }
                        _ => Err(Error::new_spanned(path, "Unknown setting".to_owned())),
                    }
                } else {
                    Err(Error::new_spanned(expr, "Expected simple identifier".to_owned()))
                }
            }
            _ => Err(Error::new_spanned(expr, "Expected (<...>=<...>)")),
        }
    }

    fn inter_fields_conflicts(&self) -> Result<(), Error> {
        if let (Some(skip), None) = (&self.setter.skip, &self.default) {
            return Err(Error::new(
                *skip,
                "#[builder(skip)] must be accompanied by default or default_code",
            ));
        }

        let conflicting_transformations = [
            ("transform", self.setter.transform.as_ref().map(|t| &t.span)),
            ("strip_option", self.setter.strip_option.as_ref()),
            ("strip_bool", self.setter.strip_bool.as_ref()),
        ];
        let mut conflicting_transformations = conflicting_transformations
            .iter()
            .filter_map(|(caption, span)| span.map(|span| (caption, span)))
            .collect::<Vec<_>>();

        if 1 < conflicting_transformations.len() {
            let (first_caption, first_span) = conflicting_transformations.pop().unwrap();
            let conflicting_captions = conflicting_transformations
                .iter()
                .map(|(caption, _)| **caption)
                .collect::<Vec<_>>();
            let mut error = Error::new(
                *first_span,
                format_args!("{} conflicts with {}", first_caption, conflicting_captions.join(", ")),
            );
            for (caption, span) in conflicting_transformations {
                error.combine(Error::new(*span, format_args!("{} set here", caption)));
            }
            return Err(error);
        }
        Ok(())
    }
}

impl SetterSettings {
    fn apply_meta(&mut self, expr: syn::Expr) -> Result<(), Error> {
        match expr {
            syn::Expr::Assign(assign) => {
                let name =
                    expr_to_single_string(&assign.left).ok_or_else(|| Error::new_spanned(&assign.left, "Expected identifier"))?;
                match name.as_str() {
                    "doc" => {
                        self.doc = Some(*assign.right);
                        Ok(())
                    }
                    "transform" => {
                        self.transform = Some(parse_transform_closure(assign.left.span(), &assign.right)?);
                        Ok(())
                    }
                    _ => Err(Error::new_spanned(&assign, format!("Unknown parameter {:?}", name))),
                }
            }
            syn::Expr::Path(path) => {
                let name = path_to_single_string(&path.path).ok_or_else(|| Error::new_spanned(&path, "Expected identifier"))?;
                macro_rules! handle_fields {
                    ( $( $flag:expr, $field:ident, $already:expr, $checks:expr; )* ) => {
                        match name.as_str() {
                            $(
                                $flag => {
                                    if self.$field.is_some() {
                                        Err(Error::new(path.span(), concat!("Illegal setting - field is already ", $already)))
                                    } else {
                                        $checks;
                                        self.$field = Some(path.span());
                                        Ok(())
                                    }
                                }
                            )*
                            _ => Err(Error::new_spanned(
                                    &path,
                                    format!("Unknown setter parameter {:?}", name),
                            ))
                        }
                    }
                }
                handle_fields!(
                    "skip", skip, "skipped", {};
                    "into", auto_into, "calling into() on the argument", {};
                    "strip_option", strip_option, "putting the argument in Some(...)", {};
                    "strip_bool", strip_bool, "zero arguments setter, sets the field to true", {};
                )
            }
            syn::Expr::Unary(syn::ExprUnary {
                op: syn::UnOp::Not(_),
                expr,
                ..
            }) => {
                if let syn::Expr::Path(path) = *expr {
                    let name =
                        path_to_single_string(&path.path).ok_or_else(|| Error::new_spanned(&path, "Expected identifier"))?;
                    match name.as_str() {
                        "doc" => {
                            self.doc = None;
                            Ok(())
                        }
                        "skip" => {
                            self.skip = None;
                            Ok(())
                        }
                        "auto_into" => {
                            self.auto_into = None;
                            Ok(())
                        }
                        "strip_option" => {
                            self.strip_option = None;
                            Ok(())
                        }
                        "strip_bool" => {
                            self.strip_bool = None;
                            Ok(())
                        }
                        _ => Err(Error::new_spanned(path, "Unknown setting".to_owned())),
                    }
                } else {
                    Err(Error::new_spanned(expr, "Expected simple identifier".to_owned()))
                }
            }
            _ => Err(Error::new_spanned(expr, "Expected (<...>=<...>)")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Transform {
    pub params: Vec<(syn::Pat, syn::Type)>,
    pub body: syn::Expr,
    span: Span,
}

fn parse_transform_closure(span: Span, expr: &syn::Expr) -> Result<Transform, Error> {
    let closure = match expr {
        syn::Expr::Closure(closure) => closure,
        _ => return Err(Error::new_spanned(expr, "Expected closure")),
    };
    if let Some(kw) = &closure.asyncness {
        return Err(Error::new(kw.span, "Transform closure cannot be async"));
    }
    if let Some(kw) = &closure.capture {
        return Err(Error::new(kw.span, "Transform closure cannot be move"));
    }

    let params = closure
        .inputs
        .iter()
        .map(|input| match input {
            syn::Pat::Type(pat_type) => Ok((syn::Pat::clone(&pat_type.pat), syn::Type::clone(&pat_type.ty))),
            _ => Err(Error::new_spanned(input, "Transform closure must explicitly declare types")),
        })
        .collect::<Result<Vec<_>, _>>()?;

    let body = &closure.body;

    Ok(Transform {
        params,
        body: syn::Expr::clone(body),
        span,
    })
}
