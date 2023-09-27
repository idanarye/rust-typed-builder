use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{
    parenthesized,
    parse::{Parse, Parser},
    punctuated::Punctuated,
    token, Error, Expr, Token,
};

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

pub fn expr_to_lit_string(expr: &syn::Expr) -> Result<String, Error> {
    match expr {
        syn::Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Str(str) => Ok(str.value()),
            _ => Err(Error::new_spanned(expr, "attribute only allows str values")),
        },
        _ => Err(Error::new_spanned(expr, "attribute only allows str values")),
    }
}

pub enum AttrArg {
    Flag(Ident),
    KeyValue(KeyValue),
    Sub(SubAttr),
    Not { not: Token![!], name: Ident },
}

impl AttrArg {
    pub fn name(&self) -> &Ident {
        match self {
            AttrArg::Flag(name) => name,
            AttrArg::KeyValue(KeyValue { name, .. }) => name,
            AttrArg::Sub(SubAttr { name, .. }) => name,
            AttrArg::Not { name, .. } => name,
        }
    }

    pub fn incorrect_type(&self) -> syn::Error {
        let message = match self {
            AttrArg::Flag(name) => format!("{:?} is not supported as a flag", name.to_string()),
            AttrArg::KeyValue(KeyValue { name, .. }) => format!("{:?} is not supported as key-value", name.to_string()),
            AttrArg::Sub(SubAttr { name, .. }) => format!("{:?} is not supported as nested attribute", name.to_string()),
            AttrArg::Not { name, .. } => format!("{:?} cannot be nullified", name.to_string()),
        };
        syn::Error::new_spanned(self, message)
    }

    pub fn flag(self) -> syn::Result<Ident> {
        if let Self::Flag(name) = self {
            Ok(name)
        } else {
            Err(self.incorrect_type())
        }
    }

    pub fn key_value(self) -> syn::Result<KeyValue> {
        if let Self::KeyValue(key_value) = self {
            Ok(key_value)
        } else {
            Err(self.incorrect_type())
        }
    }

    pub fn key_value_or_not(self) -> syn::Result<Option<KeyValue>> {
        match self {
            Self::KeyValue(key_value) => Ok(Some(key_value)),
            Self::Not { .. } => Ok(None),
            _ => Err(self.incorrect_type()),
        }
    }

    pub fn sub_attr(self) -> syn::Result<SubAttr> {
        if let Self::Sub(sub_attr) = self {
            Ok(sub_attr)
        } else {
            Err(self.incorrect_type())
        }
    }

    pub fn apply_flag_to_field(self, field: &mut Option<Span>, caption: &str) -> syn::Result<()> {
        match self {
            AttrArg::Flag(flag) => {
                if field.is_none() {
                    *field = Some(flag.span());
                    Ok(())
                } else {
                    Err(Error::new(
                        flag.span(),
                        format!("Illegal setting - field is already {caption}"),
                    ))
                }
            }
            AttrArg::Not { .. } => {
                *field = None;
                Ok(())
            }
            _ => Err(self.incorrect_type()),
        }
    }
}

pub struct KeyValue {
    pub name: Ident,
    pub eq: Token![=],
    pub value: Expr,
}

impl ToTokens for KeyValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.name.to_tokens(tokens);
        self.eq.to_tokens(tokens);
        self.value.to_tokens(tokens);
    }
}

pub struct SubAttr {
    pub name: Ident,
    pub paren: token::Paren,
    pub args: TokenStream,
}

impl SubAttr {
    pub fn args<T: Parse>(self) -> syn::Result<impl IntoIterator<Item = T>> {
        Punctuated::<T, Token![,]>::parse_terminated.parse2(self.args)
    }
}

impl ToTokens for SubAttr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.name.to_tokens(tokens);
        self.paren.surround(tokens, |t| self.args.to_tokens(t));
    }
}

impl Parse for AttrArg {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Token![!]) {
            Ok(Self::Not {
                not: input.parse()?,
                name: input.parse()?,
            })
        } else {
            let name = input.parse()?;
            if input.peek(Token![,]) || input.is_empty() {
                Ok(Self::Flag(name))
            } else if input.peek(token::Paren) {
                let args;
                Ok(Self::Sub(SubAttr {
                    name,
                    paren: parenthesized!(args in input),
                    args: args.parse()?,
                }))
            } else if input.peek(Token![=]) {
                Ok(Self::KeyValue(KeyValue {
                    name,
                    eq: input.parse()?,
                    value: input.parse()?,
                }))
            } else {
                Err(input.error("expected !<ident>, <ident>=<value> or <ident>(…)"))
            }
        }
    }
}

impl ToTokens for AttrArg {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            AttrArg::Flag(flag) => flag.to_tokens(tokens),
            AttrArg::KeyValue(kv) => kv.to_tokens(tokens),
            AttrArg::Sub(sub) => sub.to_tokens(tokens),
            AttrArg::Not { not, name } => {
                not.to_tokens(tokens);
                name.to_tokens(tokens);
            }
        }
    }
}

pub trait ApplyMeta {
    fn apply_meta(&mut self, expr: AttrArg) -> Result<(), Error>;

    fn apply_sub_attr(&mut self, attr_arg: AttrArg) -> syn::Result<()> {
        for arg in attr_arg.sub_attr()?.args()? {
            self.apply_meta(arg)?;
        }
        Ok(())
    }

    fn apply_subsections(&mut self, list: &syn::MetaList) -> syn::Result<()> {
        if list.tokens.is_empty() {
            return Err(syn::Error::new_spanned(list, "Expected builder(…)"));
        }

        let parser = syn::punctuated::Punctuated::<_, syn::token::Comma>::parse_terminated;
        let exprs = parser.parse2(list.tokens.clone())?;
        for expr in exprs {
            self.apply_meta(expr)?;
        }

        Ok(())
    }
}
