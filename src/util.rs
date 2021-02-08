use quote::ToTokens;

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
    if let syn::Expr::Path(path) = &*expr {
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

pub fn make_punctuated_single<T, P: Default>(value: T) -> syn::punctuated::Punctuated<T, P> {
    let mut punctuated = syn::punctuated::Punctuated::new();
    punctuated.push(value);
    punctuated
}

pub fn modify_types_generics_hack<F>(ty_generics: &syn::TypeGenerics, mut mutator: F) -> syn::AngleBracketedGenericArguments
where
    F: FnMut(&mut syn::punctuated::Punctuated<syn::GenericArgument, syn::token::Comma>),
{
    let mut abga: syn::AngleBracketedGenericArguments = syn::parse(ty_generics.clone().into_token_stream().into())
        .unwrap_or_else(|_| syn::AngleBracketedGenericArguments {
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
