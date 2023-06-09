use quote::ToTokens;
use std::collections::HashSet;
use syn::Token;

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut parsed = syn::parse_macro_input!(input as syn::DeriveInput);

    let ident = &parsed.ident;
    let fields = if let syn::Data::Struct(syn::DataStruct { ref fields, .. }) = parsed.data {
        fields
    } else {
        panic!("Expected struct");
    };

    let fields = if let syn::Fields::Named(syn::FieldsNamed { named, .. }) = fields {
        named
    } else {
        panic!("Expected named fields");
    };
    let fmt_fields = fields.iter().map(|field| {
        let field_ident = field
            .ident
            .as_ref()
            .expect("Expected identifier for named field");
        let quoted_field_ident = format!("{}", field_ident).to_token_stream();

        if let Some(attr) = field
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("debug"))
        {
            let expr = &attr
                .meta
                .require_name_value()
                .expect("Expected name-value pattern")
                .value;
            let str_literal = if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(literal),
                ..
            }) = expr
            {
                literal
            } else {
                panic!("Expected literal string");
            };
            let args = str_literal.value();
            // why does format_args! work but format! doesn't?
            quote::quote! { .field(#quoted_field_ident, &format_args!(#args, &self.#field_ident))}
        } else {
            quote::quote! { .field(#quoted_field_ident, &self.#field_ident)}
        }
    });

    let attr_meta = parsed.attrs.iter().find_map(|attr| {
        if let syn::Meta::List(syn::MetaList { path, .. }) = &attr.meta {
            if path.is_ident("debug") {
                let parsed_meta: syn::MetaNameValue = attr.parse_args().expect("");
                Some(parsed_meta)
            } else {
                None
            }
        } else {
            None
        }
    });

    let bound_literal = if let Some(meta) = &attr_meta {
        if let syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(literal),
            ..
        }) = &meta.value
        {
            Some(literal)
        } else {
            unimplemented!()
        }
    } else {
        None
    };

    let mut associated_bounds = vec![];
    parsed.generics.params.iter_mut().for_each(|ref mut param| {
        let fields = if let syn::Data::Struct(syn::DataStruct { ref fields, .. }) = &parsed.data {
            fields
        } else {
            panic!("expected struct fields");
        };

        if let syn::GenericParam::Type(syn::TypeParam {
            ident: generic_ident,
            bounds,
            ..
        }) = param
        {
            let mut bounds_set: HashSet<syn::TypeParamBound> = std::collections::HashSet::new();
            if let Some(bound) = bound_literal {
                let value: proc_macro2::TokenStream =
                    bound.value().parse().expect("failed parsing to string");
                associated_bounds.push(quote::quote! {#value});
            } else {
                for field in fields {
                    let path = if let syn::Type::Path(syn::TypePath { path, .. }) = &field.ty {
                        path
                    } else {
                        unimplemented!()
                    };

                    if let Some(segments) = contains_inner_t(&path.segments, generic_ident) {
                        if segments.len() > 1 {
                            associated_bounds.push(quote::quote! {#segments: std::fmt::Debug});
                        } else {
                            bounds_set.insert(syn::parse_quote!(std::fmt::Debug));
                        }
                    }
                }
            }
            bounds.extend(bounds_set.into_iter())
        } else {
            unimplemented!();
        }
    });

    let (impl_generics, ty_generics, where_clause) = &mut parsed.generics.split_for_impl();
    let clause = if where_clause.is_some() {
        quote::quote! {, #(#associated_bounds,)*}
    } else {
        quote::quote! {where #(#associated_bounds,)*}
    };

    let quoted_struct_ident = format!("{}", ident);
    let res = quote::quote! {
        impl #impl_generics std::fmt::Debug for #ident #ty_generics #where_clause #clause {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                fmt.debug_struct(#quoted_struct_ident)
                #(#fmt_fields)*
                .finish()
            }
        }
    };

    proc_macro::TokenStream::from(res)
}

/// Walks the type recursively and returns either when it finds some reference to T or the end.
/// * T contains T.
/// * Box<Vec<T>> contains T.
/// * NonNull<Vec<HashMap<K, A>>> does not contain T.
/// * Vec<Phantom<Box<Vec<T>>>> does not contain T.
fn contains_inner_t<'a>(
    segments: &'a syn::punctuated::Punctuated<syn::PathSegment, Token!(::)>,
    t_ident: &syn::Ident,
) -> Option<&'a syn::punctuated::Punctuated<syn::PathSegment, Token!(::)>> {
    if segments
        .iter()
        .any(|segment| segment.ident == "PhantomData")
    {
        return None;
    }

    if &segments[0].ident == t_ident {
        return Some(segments);
    }

    let res = segments
        .iter()
        .find_map(|segment| match &segment.arguments {
            syn::PathArguments::None => None,
            syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                args,
                ..
            }) => args.iter().find_map(|arg| {
                if let syn::GenericArgument::Type(syn::Type::Path(syn::TypePath { path, .. })) = arg
                {
                    contains_inner_t(&path.segments, t_ident)
                } else {
                    None
                }
            }),
            _ => unimplemented!(),
        });

    res
}
