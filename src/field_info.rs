use proc_macro2::TokenStream;
use quote::quote;
use syn;
use syn::parse::Error;
use syn::spanned::Spanned;

use crate::util::ident_to_type;
use crate::util::{expr_to_single_string, path_to_single_string};

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
        }
        .into()
    }
}

#[derive(Debug, Default)]
pub struct FieldBuilderAttr {
    pub doc: Option<syn::Expr>,
    pub skip: bool,
    pub default: Option<syn::Expr>,
}

impl FieldBuilderAttr {
    pub fn new(attrs: &[syn::Attribute]) -> Result<FieldBuilderAttr, Error> {
        let mut result = FieldBuilderAttr::default();
        let mut skip_tokens = None;
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
            // Stash its span for later (we don’t yet know if it’ll be an error)
            if result.skip && skip_tokens.is_none() {
                skip_tokens = Some(attr.tokens.clone());
            }
        }

        if result.skip && result.default.is_none() {
            return Err(Error::new_spanned(
                skip_tokens.unwrap(),
                "#[builder(skip)] must be accompanied by default or default_code",
            ));
        }

        Ok(result)
    }

    fn apply_meta(&mut self, expr: syn::Expr) -> Result<(), Error> {
        match expr {
            syn::Expr::Assign(assign) => {
                let name = expr_to_single_string(&assign.left)
                    .ok_or_else(|| Error::new_spanned(&assign.left, "Expected identifier"))?;
                match name.as_str() {
                    "doc" => {
                        self.doc = Some(*assign.right);
                        Ok(())
                    }
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
                            self.default = Some(
                                syn::parse(tokenized_code.into())
                                    .map_err(|e| Error::new_spanned(code, format!("{}", e)))?,
                            );
                        } else {
                            return Err(Error::new_spanned(assign.right, "Expected string"));
                        }
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
                    "skip" => {
                        self.skip = true;
                        Ok(())
                    }
                    "default" => {
                        self.default = Some(syn::parse(quote!(Default::default()).into()).unwrap());
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
