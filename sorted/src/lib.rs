use proc_macro::TokenStream;
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

            if let Some((index, path)) = attr_index {
                if dbg!(&path.segments[0].ident) == "sorted" {
                    let arms = node
                        .arms
                        .iter()
                        .map(|arm| {
                            if let syn::Pat::TupleStruct(syn::PatTupleStruct { path, .. }) =
                                &arm.pat
                            {
                                let ident = &path
                                    .segments
                                    .last()
                                    .expect("expected at least a path")
                                    .ident;
                                (ident, path)
                            } else {
                                panic!("sorted is unimplemented for non-enum match")
                            }
                        })
                        .collect::<Vec<_>>();

                    let mut sorted = arms.clone();
                    sorted.sort_by(|a, b| a.0.cmp(b.0));

                    for i in 0..arms.len() {
                        if arms[i].0 != sorted[i].0 {
                            let err = syn::Error::new(
                                sorted[i].1.span(),
                                format!("{} should sort before {}", sorted[i].0, arms[i].0),
                            )
                            .to_compile_error();
                            self.ts.extend(std::iter::once(quote::quote! {#err}));
                            break;
                        }
                    }
                    node.attrs.remove(index);
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
