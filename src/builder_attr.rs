use syn;
use proc_macro2::TokenStream;
use syn::parse::Error;
use quote::quote;

use crate::util::{expr_to_single_string, path_to_single_string};

#[derive(Debug, Default)]
pub struct FieldBuilderAttr {
    pub doc: Option<syn::Expr>,
    pub exclude: bool,
    pub default: Option<syn::Expr>,
}

impl FieldBuilderAttr {
    pub fn new(tts: &TokenStream) -> Result<FieldBuilderAttr, Error> {
        let mut result = FieldBuilderAttr {
            doc: None,
            exclude: false,
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

        if result.exclude && result.default.is_none() {
            return Err(Error::new_spanned(tts, "#[builder(exclude)] must be accompanied by default or default_code"));
        }

        Ok(result)
    }

    fn apply_meta(&mut self, expr: syn::Expr) -> Result<(), Error> {
        match expr {
            syn::Expr::Assign(assign) => {
                let name = expr_to_single_string(&assign.left).ok_or_else(
                    || Error::new_spanned(&assign.left, "Expected identifier"))?;
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
                    "exclude" => {
                        self.exclude = true;
                        Ok(())
                    }
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
    pub fn new(tts: &TokenStream) -> Result<TypeBuilderAttr, Error> {
        let mut result = TypeBuilderAttr {
            doc: false,
            builder_method_doc: None,
            builder_type_doc: None,
            build_method_doc: None,
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
                    _ => {
                        Err(Error::new_spanned(&assign, format!("Unknown parameter {:?}", name)))
                    }
                }
            }
            syn::Expr::Path(path) => {
                let name = path_to_single_string(&path.path).ok_or_else(
                    || Error::new_spanned(&path, "Expected identifier"))?;
                match name.as_str() {
                    "doc" => {
                        self.doc = true;
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
