use proc_macro::TokenStream;

#[derive(Debug)]
struct Seq {
    ident: syn::Ident,
    start: syn::LitInt,
    end: syn::LitInt,
    inner: proc_macro2::TokenStream,
}

impl syn::parse::Parse for Seq {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse()?;
        _ = input.parse::<syn::Token![in]>()?;
        let start = input.parse()?;
        _ = input.parse::<syn::Token![..]>()?;
        let end = input.parse()?;
        let inner;
        syn::braced!(inner in input);
        let inner = inner.parse()?;

        Ok(Seq {
            ident,
            start,
            end,
            inner,
        })
    }
}

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as Seq);

    println!("content: {:#?}", parsed.inner);
    let start = parsed
        .start
        .base10_parse::<usize>()
        .expect("failed parsing as usize");
    let end = parsed
        .end
        .base10_parse::<usize>()
        .expect("failed parsing as usize");

    let stream = (start..end)
        .map(|i| {
            let inner = parsed.inner.clone();
            parsed.lets_do_it(i, inner)
        })
        .collect::<proc_macro2::TokenStream>();

    stream.into()
}

impl Seq {
    fn lets_do_it(&self, i: usize, input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        let tb = syn::buffer::TokenBuffer::new2(input);
        let mut tts = vec![];
        let mut cursor = tb.begin();
        while !cursor.eof() {
            let tt = match cursor.token_tree().expect("unreachable!") {
                (proc_macro2::TokenTree::Ident(ident), next) if ident == self.ident => {
                    let mut lit = proc_macro2::Literal::usize_unsuffixed(i);
                    lit.set_span(ident.span());
                    cursor = next;
                    proc_macro2::TokenTree::Literal(lit)
                }
                (proc_macro2::TokenTree::Group(g), next) => {
                    let extend = self.lets_do_it(i, g.stream());
                    let mut new_group = proc_macro2::Group::new(g.delimiter(), extend);
                    new_group.set_span(g.span());
                    cursor = next;
                    proc_macro2::TokenTree::Group(new_group)
                }
                (proc_macro2::TokenTree::Ident(ident), next) => match next.punct() {
                    Some((punct, next2)) if punct.as_char() == '~' => match next2.ident() {
                        Some((ident_n, next3)) if ident_n == self.ident => {
                            let joined = format!("{}{}", ident, i);
                            let tt = syn::Ident::new(&joined, ident.span());
                            cursor = next3;
                            proc_macro2::TokenTree::Ident(tt)
                        }
                        _ => {
                            cursor = next;
                            proc_macro2::TokenTree::Ident(ident)
                        }
                    },
                    _ => {
                        cursor = next;
                        proc_macro2::TokenTree::Ident(ident)
                    }
                },
                (any_tt, next) => {
                    cursor = next;
                    any_tt
                }
            };
            tts.push(tt);
        }

        tts.into_iter().collect()
    }
}
