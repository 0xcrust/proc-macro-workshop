use proc_macro::TokenStream;

#[derive(Debug)]
struct Seq {
    ident: syn::Ident,
    in_token: syn::Token![in],
    start: syn::LitInt,
    range_token: syn::Token![..],
    end: syn::LitInt,
    content: proc_macro2::TokenStream,
}

impl syn::parse::Parse for Seq {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Seq {
            ident: input.parse()?,
            in_token: input.parse()?,
            start: input.parse()?,
            range_token: input.parse()?,
            end: input.parse()?,
            content: input.parse()?,
        })
    }
}

struct IdentVisitor;

impl syn::visit_mut::VisitMut for IdentVisitor {
    fn visit_gro
}

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as Seq);
    println!("Parsed: {:#?}", parsed);

    //let content = parsed.content;
    //TokenStream::from(quote::quote!{#content})
    TokenStream::new()
}
