use syn;

use syn::parse::Error;
use syn::spanned::Spanned;

pub fn make_identifier(kind: &str, name: &syn::Ident) -> syn::Ident {
    syn::Ident::new(&format!("TypedBuilder_{}_{}", kind, name), proc_macro2::Span::call_site())
}

// Panic if there is more than one.
pub fn map_only_one<S, T, F>(iter: &[S], dlg: F) -> Result<Option<T>, Error>
where F: Fn(&S) -> Result<Option<T>, Error>,
      S: Spanned,
{
    let mut result = None;
    for item in iter {
        if let Some(answer) = dlg(item)? {
            if result.is_some() {
                return Err(Error::new(item.span(), "Multiple #[builder] on the same field"))
            }
            result = Some(answer);
        }
    }
    Ok(result)
}

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
