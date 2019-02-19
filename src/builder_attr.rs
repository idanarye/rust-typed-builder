use syn;
use proc_macro2::TokenStream;
use syn::parse::Error;
use quote::quote;

use crate::util::{expr_to_single_string, path_to_single_string};

#[derive(Debug, Default)]
pub struct BuilderAttr {
    pub default: Option<syn::Expr>,
}

impl BuilderAttr {
    pub fn new(tts: &TokenStream) -> Result<BuilderAttr, Error> {
        let mut result = BuilderAttr {
            default: None,
        };
        if tts.is_empty() {
            return Ok(result);
        }
        let as_expr: syn::Expr = syn::parse2(tts.clone())?;

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
                return Err(Error::new_spanned(tts, "Expected (<...>)"));
            }
        }

        Ok(result)
    }

    fn apply_meta(&mut self, expr: syn::Expr) -> Result<(), Error> {
        match expr {
            syn::Expr::Assign(assign) => {
                let name = expr_to_single_string(&assign.left).ok_or_else(
                    || Error::new_spanned(&assign.left, "Expected identifier"))?;
                match name.as_str() {
                    "default" => {
                        self.default = Some(*assign.right);
                        Ok(())
                    }
                    "default_code" => {
                        if let syn::Expr::Lit(syn::ExprLit{lit: syn::Lit::Str(code), ..}) = *assign.right {
                            use std::str::FromStr;
                            let tokenized_code = TokenStream::from_str(&code.value())?;
                            self.default = Some(syn::parse(tokenized_code.into()).map_err(|e| Error::new_spanned(code, format!("{}", e)))?);
                        } else {
                            return Err(Error::new_spanned(assign.right, "Expected string"));
                        }
                        Ok(())
                    },
                    _ => {
                        Err(Error::new_spanned(&assign, format!("Unknown parameter {:?}", name)))
                    }
                }
            }
            syn::Expr::Path(path) => {
                let name = path_to_single_string(&path.path).ok_or_else(
                    || Error::new_spanned(&path, "Expected identifier"))?;
                match name.as_str() {
                    "default" => {
                        self.default = Some(syn::parse(quote!(Default::default()).into()).unwrap());
                        Ok(())
                    }
                    _ => {
                        Err(Error::new_spanned(&path, format!("Unknown parameter {:?}", name)))
                    }
                }
            }
            _ => {
                Err(Error::new_spanned(expr, "Expected (<...>=<...>)"))
            }
        }
    }
}
