use quote::ToTokens;
use syn::{parse::Parser, Error};

pub fn path_to_single_string(path: &syn::Path) -> Option<String> {
    if path.leading_colon.is_some() {
        return None;
    }
    let mut it = path.segments.iter();
    let segment = it.next()?;
    if it.next().is_some() {
        // Multipart path
        return None;
    }
    if segment.arguments != syn::PathArguments::None {
        return None;
    }
    Some(segment.ident.to_string())
}

pub fn expr_to_single_string(expr: &syn::Expr) -> Option<String> {
    if let syn::Expr::Path(path) = expr {
        path_to_single_string(&path.path)
    } else {
        None
    }
}

pub fn ident_to_type(ident: syn::Ident) -> syn::Type {
    let mut path = syn::Path {
        leading_colon: None,
        segments: Default::default(),
    };
    path.segments.push(syn::PathSegment {
        ident,
        arguments: Default::default(),
    });
    syn::Type::Path(syn::TypePath { qself: None, path })
}

pub fn empty_type() -> syn::Type {
    syn::TypeTuple {
        paren_token: Default::default(),
        elems: Default::default(),
    }
    .into()
}

pub fn type_tuple(elems: impl Iterator<Item = syn::Type>) -> syn::TypeTuple {
    let mut result = syn::TypeTuple {
        paren_token: Default::default(),
        elems: elems.collect(),
    };
    if !result.elems.empty_or_trailing() {
        result.elems.push_punct(Default::default());
    }
    result
}

pub fn empty_type_tuple() -> syn::TypeTuple {
    syn::TypeTuple {
        paren_token: Default::default(),
        elems: Default::default(),
    }
}

pub fn modify_types_generics_hack<F>(ty_generics: &syn::TypeGenerics, mut mutator: F) -> syn::AngleBracketedGenericArguments
where
    F: FnMut(&mut syn::punctuated::Punctuated<syn::GenericArgument, syn::token::Comma>),
{
    let mut abga: syn::AngleBracketedGenericArguments =
        syn::parse2(ty_generics.to_token_stream()).unwrap_or_else(|_| syn::AngleBracketedGenericArguments {
            colon2_token: None,
            lt_token: Default::default(),
            args: Default::default(),
            gt_token: Default::default(),
        });
    mutator(&mut abga.args);
    abga
}

pub fn strip_raw_ident_prefix(mut name: String) -> String {
    if name.starts_with("r#") {
        name.replace_range(0..2, "");
    }
    name
}

pub fn first_visibility(visibilities: &[Option<&syn::Visibility>]) -> proc_macro2::TokenStream {
    let vis = visibilities
        .iter()
        .flatten()
        .next()
        .expect("need at least one visibility in the list");

    vis.to_token_stream()
}

pub fn public_visibility() -> syn::Visibility {
    syn::Visibility::Public(syn::token::Pub::default())
}

pub fn apply_subsections(
    list: &syn::MetaList,
    mut applier: impl FnMut(syn::Expr) -> Result<(), syn::Error>,
) -> Result<(), syn::Error> {
    if list.tokens.is_empty() {
        return Err(syn::Error::new_spanned(list, "Expected builder(…)"));
    }

    let parser = syn::punctuated::Punctuated::<_, syn::token::Comma>::parse_terminated;
    let exprs = parser.parse2(list.tokens.clone())?;
    for expr in exprs {
        applier(expr)?;
    }

    Ok(())
}

pub fn expr_to_lit_string(expr: &syn::Expr) -> Result<String, Error> {
    match expr {
        syn::Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Str(str) => Ok(str.value()),
            _ => return Err(Error::new_spanned(expr, "attribute only allows str values")),
        },
        _ => return Err(Error::new_spanned(expr, "attribute only allows str values")),
    }
}
