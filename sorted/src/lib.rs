use proc_macro::TokenStream;
use syn::spanned::Spanned;

#[proc_macro_attribute]
pub fn sorted(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let _ = input;

    let parsed = syn::parse_macro_input!(input as syn::Item);
    match __sorted(args, parsed) {
        Ok(res) => res,
        Err(e) => e.to_compile_error().into(),
    }
}

fn __sorted(args: TokenStream, input: syn::Item) -> Result<TokenStream, syn::Error> {
    let _ = args;
    let _ = input;

    let variants = match input {
        syn::Item::Enum(item) => item.variants,
        _ => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "expected enum or match expression",
            ))
        }
    };
    let variants = variants.iter().collect::<Vec<_>>();

    let mut sorted = variants.clone();
    sorted.sort_by(|v1, v2| v1.ident.to_string().cmp(&v2.ident.to_string()));

    for i in 0..variants.len() {
        if variants[i].ident != sorted[i].ident {
            return Err(syn::Error::new(
                sorted[i].span(),
                format!(
                    "{} should sort before {}",
                    sorted[i].ident, variants[i].ident
                ),
            ));
        }
    }

    Ok(TokenStream::new())
}
