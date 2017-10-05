use syn;

pub fn make_identifier(kind: &str, name: &syn::Ident) -> syn::Ident {
    format!("TypedBuilder_{}_{}", kind, name).into()
}

// Panic if there is more than one.
pub fn map_only_one<S, T, F>(iter: &[S], dlg: F) -> Result<Option<T>, String>
    where F: Fn(&S) -> Result<Option<T>, String>
{
    let mut result = None;
    for item in iter {
        if let Some(answer) = dlg(item)? {
            if result.is_some() {
                return Err("multiple defaults".into());
            }
            result = Some(answer);
        }
    }
    Ok(result)
}
