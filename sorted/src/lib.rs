use proc_macro::TokenStream;
use quote::ToTokens;
use syn::spanned::Spanned;

#[proc_macro_attribute]
pub fn check(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut parsed = syn::parse_macro_input!(input as syn::ItemFn);

    struct ExprVisitor {
        ts: proc_macro2::TokenStream,
    }
    impl ExprVisitor {
        pub fn new() -> ExprVisitor {
            ExprVisitor {
                ts: proc_macro2::TokenStream::new(),
            }
        }
    }

    use syn::visit_mut::VisitMut;

    impl VisitMut for ExprVisitor {
        fn visit_expr_match_mut(&mut self, node: &mut syn::ExprMatch) {
            let attr_index = node.attrs.iter().enumerate().find_map(|(index, attr)| {
                if let syn::Meta::Path(path) = &attr.meta {
                    Some((index, path))
                } else {
                    None
                }
            });

            #[derive(Clone)]
            struct Triplet {
                ident: syn::Ident,
                tokens: proc_macro2::TokenStream,
                fq_path: String,
            }
            impl Triplet {
                pub fn new(
                    ident: syn::Ident,
                    tokens: proc_macro2::TokenStream,
                    fq_path: String,
                ) -> Self {
                    Triplet {
                        ident,
                        tokens,
                        fq_path,
                    }
                }
                pub fn from_path(path: &syn::Path) -> Self {
                    let ident = &path
                        .segments
                        .last()
                        .expect("expected at least a path")
                        .ident;
                    Triplet::new(
                        ident.clone(),
                        path.to_token_stream(),
                        full_path(&path.segments),
                    )
                }
            }

            fn full_path<T>(segments: &syn::punctuated::Punctuated<syn::PathSegment, T>) -> String {
                let mut qualified = String::new();

                for x in 0..segments.len() {
                    qualified.push_str(&format!("{}", segments[x].ident));
                    if x != segments.len() - 1 {
                        qualified.push_str("::");
                    }
                }

                qualified
            }

            if let Some((index, path)) = attr_index {
                if &path.segments[0].ident == "sorted" {
                    node.attrs.remove(index);
                    let mut unsorted = vec![];
                    let mut arms_iter = node.arms.iter();
                    let unsupported_err = |arm: &syn::Arm| {
                        syn::Error::new(arm.pat.span(), "unsupported by #[sorted]")
                            .to_compile_error()
                    };

                    while let Some(arm) = arms_iter.next() {
                        let pattern = match &arm.pat {
                            syn::Pat::Ident(syn::PatIdent { ident, .. }) => Triplet::new(
                                ident.clone(),
                                arm.pat.to_token_stream(),
                                ident.to_string(),
                            ),
                            syn::Pat::TupleStruct(syn::PatTupleStruct { path, .. }) => {
                                Triplet::from_path(path)
                            }
                            syn::Pat::Path(syn::PatPath { path, .. }) => Triplet::from_path(path),
                            syn::Pat::Struct(syn::PatStruct { path, .. }) => {
                                Triplet::from_path(path)
                            }
                            syn::Pat::Wild(_) => {
                                if arms_iter.next().is_some() {
                                    let err = unsupported_err(arm);
                                    self.ts.extend(std::iter::once(quote::quote! {#err}));
                                    return;
                                } else {
                                    break;
                                }
                            }
                            _ => {
                                let err = unsupported_err(arm);
                                self.ts.extend(std::iter::once(quote::quote! {#err}));
                                return;
                            }
                        };

                        unsorted.push(pattern)
                    }

                    let mut sorted = unsorted.clone();
                    sorted.sort_by(|a, b| a.ident.cmp(&b.ident));

                    for i in 0..unsorted.len() {
                        if unsorted[i].ident != sorted[i].ident {
                            let err = syn::Error::new_spanned(
                                sorted[i].tokens.clone(),
                                format!(
                                    "{} should sort before {}",
                                    sorted[i].fq_path, unsorted[i].fq_path
                                ),
                            )
                            .to_compile_error();
                            self.ts.extend(std::iter::once(quote::quote! {#err}));
                            break;
                        }
                    }
                }
            }
            syn::visit_mut::visit_expr_match_mut(self, node);
        }
    }

    let mut visitor = ExprVisitor::new();
    visitor.visit_item_fn_mut(&mut parsed);
    let ts = visitor.ts;
    TokenStream::from(quote::quote! {#parsed #ts})
}

#[proc_macro_attribute]
pub fn sorted(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let _ = input;

    let parsed = syn::parse_macro_input!(input as syn::Item);

    match __sorted(args, &parsed) {
        Ok(()) => TokenStream::from(quote::quote! {#parsed}),
        Err(e) => {
            let e = e.to_compile_error();
            TokenStream::from(quote::quote! {#parsed #e})
        }
    }
}

fn __sorted(_args: TokenStream, parsed: &syn::Item) -> Result<(), syn::Error> {
    let variants = match parsed {
        syn::Item::Enum(item) => &item.variants,
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

    Ok(())
}
