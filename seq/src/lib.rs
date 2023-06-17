use proc_macro::TokenStream;

#[derive(Debug)]
struct Seq {
    ident: syn::Ident,
    start: syn::LitInt,
    inclusive: bool,
    end: syn::LitInt,
    inner: proc_macro2::TokenStream,
}

impl syn::parse::Parse for Seq {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse()?;
        _ = input.parse::<syn::Token![in]>()?;
        let start = input.parse()?;
        let inclusive = if input.peek(syn::Token![..=]) {
            input.parse::<syn::Token![..=]>()?;
            true
        } else {
            input.parse::<syn::Token![..]>()?;
            false
        };
        //_ = input.parse::<syn::Token![..]>()?;
        let end = input.parse()?;
        let inner;
        syn::braced!(inner in input);
        let inner = inner.parse()?;

        Ok(Seq {
            ident,
            start,
            inclusive,
            end,
            inner,
        })
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Mode {
    Outer { i: usize }, // find and repeat directly
    Inner,              // find inner and then start a range repeating that.
}
#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let seq = syn::parse_macro_input!(input as Seq);

    let inner = seq.inner.clone();
    let stream = if Seq::peek_repitition(inner.clone()) {
        seq.lets_do_it(Mode::Inner, inner)
    } else {
        println!("starting with after");
        seq.range()
            .flat_map(|i| seq.lets_do_it(Mode::Outer { i }, inner.clone()))
            .collect()
    };

    stream
        .into_iter()
        .collect::<proc_macro2::TokenStream>()
        .into()
}

impl Seq {
    fn range(&self) -> core::ops::Range<usize> {
        let start = self
            .start
            .base10_parse::<usize>()
            .expect("failed parsing as usize");
        let end = self
            .end
            .base10_parse::<usize>()
            .expect("failed parsing as usize");

        if self.inclusive {
            start..end + 1
        } else {
            start..end
        }
    }

    fn lets_do_it(
        &self,
        mode: Mode,
        input: proc_macro2::TokenStream,
    ) -> Vec<proc_macro2::TokenTree> {
        let mut tts = vec![];
        let tb = syn::buffer::TokenBuffer::new2(input);
        let mut cursor = tb.begin();

        while !cursor.eof() {
            let position = cursor.token_tree().expect("unreachable!");
            let tt = match position.clone() {
                (proc_macro2::TokenTree::Ident(ident), next) if ident == self.ident => {
                    cursor = next;
                    if let Mode::Outer { i } = mode {
                        let mut lit = proc_macro2::Literal::usize_unsuffixed(i);
                        lit.set_span(ident.span());
                        proc_macro2::TokenTree::Literal(lit)
                    } else {
                        position.0
                    }
                }
                (proc_macro2::TokenTree::Group(g), next) => {
                    let extend = self.lets_do_it(mode, g.stream()).into_iter().collect();
                    let mut g2 = proc_macro2::Group::new(g.delimiter(), extend);
                    g2.set_span(g.span());
                    cursor = next;
                    proc_macro2::TokenTree::Group(g2)
                }
                (proc_macro2::TokenTree::Punct(punct), next)
                    if mode == Mode::Inner && punct.as_char() == '#' =>
                {
                    let delim = proc_macro2::Delimiter::Parenthesis;
                    match next.group(delim) {
                        Some((g, _, next2)) => match next2.token_tree() {
                            Some((proc_macro2::TokenTree::Punct(p), _next3))
                                if p.as_char() == '*' =>
                            {
                                let extend = self
                                    .range()
                                    .flat_map(|i| {
                                        self.lets_do_it(Mode::Outer { i }, g.token_stream())
                                    })
                                    .collect();
                                let mut g2 =
                                    proc_macro2::Group::new(proc_macro2::Delimiter::None, extend);
                                g2.set_span(g.span());
                                tts.push(proc_macro2::TokenTree::Group(g2));
                                break;
                            }
                            _ => panic!("unimplemented!"), // found # and group but no *
                        },
                        _ => {
                            cursor = next;
                            proc_macro2::TokenTree::Punct(punct)
                        }
                    }
                }
                (proc_macro2::TokenTree::Ident(ident), next) => {
                    if let Mode::Outer { i } = mode {
                        match next.punct() {
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
                        }
                    } else {
                        cursor = next;
                        position.0
                    }
                }
                (any_tt, next) => {
                    cursor = next;
                    any_tt
                }
            };
            tts.push(tt);
        }

        tts
    }

    fn peek_repitition(input: proc_macro2::TokenStream) -> bool {
        let buf = syn::buffer::TokenBuffer::new2(input);
        let mut cursor = buf.begin();
        let found = false;
        while !cursor.eof() {
            let token_tree = cursor.token_tree().expect("");
            match token_tree {
                (proc_macro2::TokenTree::Group(g), next) => {
                    if Self::peek_repitition(g.stream()) {
                        return true;
                    } else {
                        cursor = next;
                    }
                }
                (proc_macro2::TokenTree::Punct(punct), next) if punct.as_char() == '#' => {
                    let delim = proc_macro2::Delimiter::Parenthesis;
                    match next.group(delim) {
                        Some((inner, _, next2)) => match next2.token_tree() {
                            Some((proc_macro2::TokenTree::Punct(p), _)) if p.as_char() == '*' => {
                                return true;
                            }
                            _ => {
                                if Self::peek_repitition(inner.token_stream()) {
                                    return true;
                                } else {
                                    cursor = next;
                                }
                            }
                        },
                        _ => {
                            cursor = next;
                        }
                    }
                }
                (_, next) => {
                    cursor = next;
                    continue;
                }
            }
        }
        found
    }
}
