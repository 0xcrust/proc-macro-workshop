use quote::ToTokens;
use std::collections::HashSet;

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
            for field in fields {
                let path = if let syn::Type::Path(syn::TypePath { path, .. }) = &field.ty {
                    path
                } else {
                    unimplemented!()
                };

                let segment = &path.segments[0];
                if contains_inner_t(segment, generic_ident) {
                    bounds_set.insert(syn::parse_quote!(std::fmt::Debug));
                }
            }
            bounds.extend(bounds_set.into_iter())
        } else {
            unimplemented!();
        }
    });

    let (impl_generics, ty_generics, where_clause) = &mut parsed.generics.split_for_impl();
    let quoted_struct_ident = format!("{}", ident);
    let res = quote::quote! {
        impl #impl_generics std::fmt::Debug for #ident #ty_generics #where_clause {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                fmt.debug_struct(#quoted_struct_ident)
                #(#fmt_fields)*
                .finish()
            }
        }
    };

    proc_macro::TokenStream::from(res)
}

// Walks the type recursively and returns either when it finds some reference to T or the end.
// Should identify that T contains T.
// Should identify that Box<Vec<T>> contains T.
// Should identify that NonNull<Vec<HashMap<K, A>>> does not contain T.
// Should identify that Vec<Phantom<Box<Vec<T>>>> does not contain T.
fn contains_inner_t(path_segment: &syn::PathSegment, t_ident: &syn::Ident) -> bool {
    if &path_segment.ident == t_ident {
        return true;
    }

    if path_segment.ident == "PhantomData" {
        return false;
    }

    let res = match &path_segment.arguments {
        syn::PathArguments::None => return false,
        syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
            args, ..
        }) => args.iter().any(|arg| {
            if let syn::GenericArgument::Type(syn::Type::Path(syn::TypePath { path, .. })) = arg {
                path.segments
                    .iter()
                    .any(|segment| contains_inner_t(segment, t_ident))
            } else {
                false
            }
        }),
        _ => unimplemented!(),
    };

    res
}

#[allow(dead_code)]
fn extract_inner_type(path_segment: &syn::PathSegment) -> Option<String> {
    let args = if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
        args,
        ..
    }) = &path_segment.arguments
    {
        args
    } else {
        unimplemented!();
    };

    if let syn::GenericArgument::Type(syn::Type::Path(syn::TypePath { path, .. })) = &args[0] {
        Some(path.segments[0].ident.to_string())
    } else {
        None
    }
}
