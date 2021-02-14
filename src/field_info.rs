use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::Error;
use syn::spanned::Spanned;

use crate::util::{expr_to_single_string, ident_to_type, path_to_single_ident, path_to_single_string, strip_raw_ident_prefix};

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
            Ok(FieldInfo {
                ordinal,
                name: &name,
                generic_ident: syn::Ident::new(
                    &format!("__{}", strip_raw_ident_prefix(name.to_string())),
                    proc_macro2::Span::call_site(),
                ),
                ty: &field.ty,
                builder_attr: field_defaults.with(&field.attrs)?,
            })
        } else {
            Err(Error::new(field.span(), "Nameless field in struct"))
        }
    }

    pub fn generic_ty_param(&self) -> syn::GenericParam {
        syn::GenericParam::Type(self.generic_ident.clone().into())
    }

    pub fn item_name(&self) -> String {
        self.builder_attr
            .setter
            .extend
            .as_ref()
            .expect("Tried to retrieve item_name() on FieldInfo without extend.")
            .item_name
            .as_ref()
            .map_or_else(
                || self.name.to_string().trim_start().trim_start_matches("r#").to_string() + "_item",
                |item_name| item_name.to_string(),
            )
    }

    pub fn type_ident(&self) -> syn::Type {
        ident_to_type(self.generic_ident.clone())
    }

    pub fn tuplized_type_ty_param(&self) -> Result<syn::Type, Error> {
        let mut types = syn::punctuated::Punctuated::default();
        types.push(
            if matches!(
                self.builder_attr.setter,
                SetterSettings {
                    extend: Some(_),
                    strip_option: true,
                    ..
                }
            ) {
                self.type_from_inside_option()?
            } else {
                self.ty
            }
            .clone(),
        );
        types.push_punct(Default::default());
        Ok(syn::TypeTuple {
            paren_token: Default::default(),
            elems: types,
        }
        .into())
    }

    pub fn type_from_inside_option(&self) -> Result<&syn::Type, Error> {
        assert!(self.builder_attr.setter.strip_option);

        pub fn try_<'a>(field_info: &'a FieldInfo) -> Option<&'a syn::Type> {
            let path = if let syn::Type::Path(type_path) = field_info.ty {
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

        try_(self).ok_or_else(|| Error::new_spanned(&self.ty, "can't `strip_option` - field is not `Option<...>`"))
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
    pub skip: bool,
    pub auto_into: bool,
    pub extend: Option<ExtendField>,
    pub strip_option: bool,
}

#[derive(Debug, Clone)]
pub enum Configurable<T> {
    Unset,
    Auto { keyword_span: Span },
    Custom { keyword_span: Span, value: T },
}

#[derive(Debug, Clone)]
pub struct Configured<T> {
    pub keyword_span: Span,
    pub value: T,
}

impl<T> Configurable<T> {
    pub fn into_configured(self, auto: impl FnOnce(Span) -> T) -> Option<Configured<T>> {
        match self {
            Configurable::Unset => None,
            Configurable::Auto { keyword_span } => Some(Configured {
                keyword_span,
                value: auto(keyword_span),
            }),
            Configurable::Custom { keyword_span, value } => Some(Configured { keyword_span, value }),
        }
    }

    pub fn ensure_unset(&self, error_span: Span) -> Result<(), Error> {
        if matches!(self, Configurable::Unset) {
            Ok(())
        } else {
            Err(Error::new(error_span, "Duplicate option"))
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExtendField {
    pub keyword_span: Span,
    pub from_first: Configurable<syn::ExprClosure>,
    pub from_iter: Configurable<syn::ExprClosure>,
    pub item_name: Option<syn::Ident>,
}

impl FieldBuilderAttr {
    pub fn with(mut self, attrs: &[syn::Attribute]) -> Result<Self, Error> {
        let mut skip_tokens = None;
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
            // Stash its span for later (we don’t yet know if it’ll be an error)
            if self.setter.skip && skip_tokens.is_none() {
                skip_tokens = Some(attr.tokens.clone());
            }
        }

        if self.setter.skip && self.default.is_none() {
            return Err(Error::new_spanned(
                skip_tokens.unwrap(),
                "#[builder(skip)] must be accompanied by default or default_code",
            ));
        }

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
                        self.default = Some(syn::parse(quote!(Default::default()).into()).unwrap());
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
                    _ => Err(Error::new_spanned(&assign, format!("Unknown parameter {:?}", name))),
                }
            }
            syn::Expr::Call(call) => {
                let name =
                    expr_to_single_string(&call.func).ok_or_else(|| Error::new_spanned(&call.func, "Expected identifier"))?;
                match name.as_str() {
                    "extend" => {
                        if self.extend.is_some() {
                            Err(Error::new(
                                call.span(),
                                "Illegal setting - field is already calling extend(...) with the argument",
                            ))
                        } else if let Some(attr) = call.attrs.first() {
                            Err(Error::new_spanned(attr, "Unexpected attribute"))
                        } else {
                            let mut extend = ExtendField {
                                keyword_span: name.span(),
                                from_first: Configurable::Unset,
                                from_iter: Configurable::Unset,
                                item_name: None,
                            };
                            for arg in call.args {
                                match arg {
                                    syn::Expr::Assign(assign) => {
                                        let name = expr_to_single_string(&assign.left)
                                            .ok_or_else(|| Error::new_spanned(&assign.left, "Expected identifier"))?;
                                        match name.as_str() {
                                            "from_first" => {
                                                extend.from_first.ensure_unset(assign.left.span())?;
                                                match *assign.right {
                                                    syn::Expr::Closure(closure) => {
                                                        extend.from_first = Configurable::Custom {
                                                            keyword_span: assign.left.span(),
                                                            value: closure,
                                                        }
                                                    }
                                                    other => {
                                                        return Err(Error::new_spanned(other, "Expected closure (|first| <...>)"))
                                                    }
                                                }
                                            }
                                            "from_iter" => {
                                                extend.from_iter.ensure_unset(assign.left.span())?;
                                                match *assign.right {
                                                    syn::Expr::Closure(closure) => {
                                                        extend.from_iter = Configurable::Custom {
                                                            keyword_span: assign.left.span(),
                                                            value: closure,
                                                        }
                                                    }
                                                    other => {
                                                        return Err(Error::new_spanned(other, "Expected closure (|iter| <...>)"))
                                                    }
                                                }
                                            }
                                            "item_name" => {
                                                if extend.item_name.is_some() {
                                                    return Err(Error::new_spanned(assign.left, "Duplicate option"));
                                                }
                                                match *assign.right {
                                                    syn::Expr::Path(path) => {
                                                        let name = path_to_single_ident(&path.path)
                                                            .ok_or_else(|| Error::new_spanned(&path, "Expected identifier"))?;
                                                        extend.item_name = Some(name.clone())
                                                    }
                                                    other => return Err(Error::new_spanned(other, "Expected identifier")),
                                                }
                                            }
                                            _ => {
                                                return Err(Error::new_spanned(
                                                    &assign.left,
                                                    format!("Unknown parameter {:?}", name),
                                                ))
                                            }
                                        }
                                    }
                                    syn::Expr::Path(path) => {
                                        let name = path_to_single_string(&path.path)
                                            .ok_or_else(|| Error::new_spanned(&path, "Expected identifier"))?;
                                        match name.as_str() {
                                            "from_first" => {
                                                extend.from_first.ensure_unset(path.span())?;
                                                extend.from_first = Configurable::Auto {
                                                    keyword_span: path.span(),
                                                };
                                            }
                                            "from_iter" => {
                                                extend.from_iter.ensure_unset(path.span())?;
                                                extend.from_iter = Configurable::Auto {
                                                    keyword_span: path.span(),
                                                };
                                            }
                                            "item_name" => return Err(Error::new_spanned(path, "Expected (item_name = <...>)")),
                                            _ => return Err(Error::new_spanned(path, format!("Unknown parameter {:?}", name))),
                                        }
                                    }
                                    _ => return Err(Error::new_spanned(arg, "Expected (<...>) or (<...>=<...>)")),
                                }
                            }
                            self.extend = Some(extend);
                            Ok(())
                        }
                    }
                    _ => Err(Error::new_spanned(&call.func, format!("Unknown parameter {:?}", name))),
                }
            }
            syn::Expr::Path(path) => {
                let name = path_to_single_string(&path.path).ok_or_else(|| Error::new_spanned(&path, "Expected identifier"))?;
                macro_rules! handle_fields {
                    ( $( $flag:expr, $field:ident, $already:expr; )* ) => {
                        match name.as_str() {
                            $(
                                $flag => {
                                    if self.$field {
                                        Err(Error::new(path.span(), concat!("Illegal setting - field is already ", $already)))
                                    } else {
                                        self.$field = true;
                                        Ok(())
                                    }
                                }
                            )*
                            "extend" => {
                                if self.extend.is_some() {
                                    Err(Error::new(path.span(), "Illegal setting - field is already calling extend(...) with the argument"))
                                } else {
                                    self.extend = Some(ExtendField {
                                        keyword_span: name.span(),
                                        from_first: Configurable::Auto { keyword_span:name.span() },
                                        from_iter: Configurable::Auto { keyword_span:name.span() },
                                        item_name: None,
                                    });
                                    Ok(())
                                }
                            }
                            _ => Err(Error::new_spanned(
                                    &path,
                                    format!("Unknown setter parameter {:?}", name),
                            ))
                        }
                    }
                }
                handle_fields!(
                    "skip", skip, "skipped";
                    "into", auto_into, "calling into() on the argument";
                    "strip_option", strip_option, "putting the argument in Some(...)";
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
                            self.skip = false;
                            Ok(())
                        }
                        "auto_into" => {
                            self.auto_into = false;
                            Ok(())
                        }
                        "extend" => {
                            self.extend = None;
                            Ok(())
                        }
                        "strip_option" => {
                            self.strip_option = false;
                            Ok(())
                        }
                        _ => Err(Error::new_spanned(path, "Unknown setting".to_owned())),
                    }
                } else {
                    Err(Error::new_spanned(expr, "Expected simple identifier".to_owned()))
                }
            }
            _ => Err(Error::new_spanned(expr, "Expected (<...>=<...>) or (<...>(<...>))")),
        }
    }
}
