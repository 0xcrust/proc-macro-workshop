use quote::ToTokens;

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut parsed = syn::parse_macro_input!(input as syn::DeriveInput);

    let ident = &parsed.ident;
    let fields = if let syn::Data::Struct(syn::DataStruct { fields, .. }) = parsed.data {
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

    parsed.generics.params.iter_mut().for_each(|param| {
        if let syn::GenericParam::Type(syn::TypeParam { bounds, .. }) = param {
            bounds.push(syn::parse_quote!(std::fmt::Debug));
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
