#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);

    let ident = ast.ident;

    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(fields_named),
        ..
    }) = ast.data
    {
        fields_named.named
    } else {
        unimplemented!()
    };

    let builder_ident = quote::format_ident!("{}Builder", &ident);

    let builder_fields_definition = generate_builder_fields_definition(&fields);
    let builder_fields_init = generate_builder_fields_init(&fields);
    let builder_methods = generate_builder_methods(&fields);
    let build_fn_definition = generate_build_fn_definition(&fields);
    let attr_methods = generate_attr_methods(&fields);

    let generated = quote::quote! {
        pub struct #builder_ident {
            #(#builder_fields_definition,)*
        }

        impl #ident {
            pub fn builder() -> #builder_ident {
                #builder_ident {
                    #(#builder_fields_init,)*
                }
            }
        }

        impl #builder_ident {
            #(#builder_methods)*

            #(#attr_methods)*

            pub fn build(&mut self) -> std::result::Result<#ident, Box<dyn std::error::Error>> {
                Ok(#ident {
                    #(#build_fn_definition,)*
                })
            }
        }
    };

    proc_macro::TokenStream::from(generated)
}

fn is_option(field: &syn::Field) -> (bool, Option<syn::PathSegment>) {
    if let syn::Type::Path(syn::TypePath {
        qself: None,
        path: syn::Path { segments, .. },
    }) = &field.ty
    {
        if let Some(found) = segments
            .iter()
            .find(|segment| segment.ident.to_string() == "Option".to_string())
        {
            (true, Some(found.clone()))
        } else {
            (false, None)
        }
    } else {
        //(false, None)
        unimplemented!()
    }
}

fn generate_builder_fields_definition(
    original_fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
    original_fields.iter().map(|field| {
        let ident = &field.ident;
        let ty = &field.ty;
        if let (true, _) = is_option(field) {
            quote::quote! {
                #ident: #ty
            }
        } else {
            quote::quote! {
                #ident: std::option::Option<#ty>
            }
        }
    })
}

fn generate_builder_fields_init(
    original_fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
    original_fields.iter().map(|field| {
        let ident = &field.ident;
        quote::quote! {
            #ident: None
        }
    })
}

fn generate_builder_methods(
    original_fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
    original_fields.iter().map(|field| {
        let ident = &field.ident;
        let ty = &field.ty;

        let arg = if let (true, Some(segment)) = is_option(field) {
            angle_bracketed_inner_type_from_segment(&segment).unwrap_or(quote::quote! { #ty})
        } else {
            quote::quote! {
                #ty
            }
        };

        quote::quote! {
            fn #ident(&mut self, #ident: #arg) -> &mut Self {
                self.#ident = Some(#ident);
                self
            }
        }
    })
}

fn generate_build_fn_definition(
    original_fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
    original_fields.iter().map(|field| {
        let ident = &field.ident;
        if let (true, _) = is_option(field) {
            quote::quote! {
                #ident: self.#ident.clone()
            }
        } else {
            quote::quote! {
                #ident: self.#ident.clone().unwrap_or_default()
            }
        }
    })
}

fn generate_attr_methods(
    original_fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
    original_fields.iter().map(|field| {
        let field_ident = field.ident.as_ref().expect("Expected struct field");

        if let Some(attr) = field
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("builder"))
        {
            let parsed: syn::MetaNameValue = attr
                .parse_args()
                .expect("failed parsing as name-value expr");

            if quote::format_ident!("{}", parsed.path.get_ident().unwrap()) != "each".to_string() {
                panic!("Unexpected attribute key");
            }

            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(literal),
                ..
            }) = parsed.value
            {
                let new_fn_ident = literal.value();
                if new_fn_ident == field_ident.to_string() {
                    quote::quote! {}
                } else {
                    let inner_type = if let syn::Type::Path(syn::TypePath {
                        qself: None,
                        path: syn::Path { segments, .. },
                    }) = &field.ty
                    {
                        if let Some(segment) = segments
                            .iter()
                            .find(|segment| segment.ident.to_string() == "Vec".to_string())
                        {
                            angle_bracketed_inner_type_from_segment(segment)
                                .expect("Expected inner type for Vec<>")
                        } else {
                            panic!("This attribute is only applicable to vectors");
                        }
                    } else {
                        unimplemented!();
                    };

                    let new_fn_ident: proc_macro2::TokenStream = new_fn_ident.parse().unwrap();
                    quote::quote! {
                        fn #new_fn_ident(&mut self, #new_fn_ident: #inner_type) -> &mut Self {
                            if let Some(x) = self.#field_ident.as_mut() {
                                x.push(#new_fn_ident);
                            } else {
                                self.#field_ident = Some(vec![#new_fn_ident]);
                            }
                            self
                        }
                    }
                }
            } else {
                unimplemented!("Expected value to be string literal")
            }
        } else {
            quote::quote! {}
        }
    })
}

fn angle_bracketed_inner_type_from_segment(
    segment: &syn::PathSegment,
) -> Option<proc_macro2::TokenStream> {
    if let syn::PathSegment {
        ident: _,
        arguments:
            syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments { args, .. }),
    } = segment
    {
        Some(quote::quote! {#args})
    } else {
        None
    }
}
