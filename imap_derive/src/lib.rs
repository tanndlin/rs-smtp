use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Ident, LitStr, Type, Variant, parse_macro_input};

/// Derived on the `ClientCommand` enum. Each variant must wrap a single command
/// struct implementing `ClientCommandTrait`. Generates a `From` impl per variant
/// and a `ClientCommand::dispatch` that parses by keyword. The keyword defaults
/// to the uppercased variant name and can be overridden with
/// `#[imap(keyword = "...")]`.
#[proc_macro_derive(ImapCommands, attributes(imap))]
pub fn imap_commands_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derived on the `ServerResponse` enum. Each variant must wrap a single
/// response struct implementing `EncodeTo`. Generates a `From` impl per variant
/// and an `EncodeTo` impl that delegates to the wrapped response.
#[proc_macro_derive(ImapResponses)]
pub fn imap_responses_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_responses(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Returns each variant of an enum along with its single wrapped type.
fn wrapping_variants<'a>(
    input: &'a DeriveInput,
    derive: &str,
) -> syn::Result<Vec<(&'a Variant, &'a Type)>> {
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input.ident,
            format!("{derive} can only be derived on an enum"),
        ));
    };

    data.variants
        .iter()
        .map(|variant| match &variant.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                Ok((variant, &fields.unnamed[0].ty))
            }
            _ => Err(syn::Error::new_spanned(
                variant,
                "each variant must wrap exactly one type, e.g. `Noop(NoopCommand)`",
            )),
        })
        .collect()
}

fn from_impl(name: &Ident, variant: &Ident, ty: &Type) -> proc_macro2::TokenStream {
    quote! {
        impl ::core::convert::From<#ty> for #name {
            fn from(inner: #ty) -> Self {
                #name::#variant(inner)
            }
        }
    }
}

fn expand_responses(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let variants = wrapping_variants(input, "ImapResponses")?;

    let from_impls = variants
        .iter()
        .map(|(variant, ty)| from_impl(name, &variant.ident, ty));
    let arms = variants.iter().map(|(variant, _)| {
        let ident = &variant.ident;
        quote! { #name::#ident(res) => ::util::EncodeTo::encode_to(res, buf), }
    });

    Ok(quote! {
        #(#from_impls)*

        impl ::util::EncodeTo for #name {
            fn encode_to(self, buf: &mut ::std::vec::Vec<u8>) {
                match self {
                    #(#arms)*
                }
            }
        }
    })
}

fn expand(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;

    let mut from_impls = Vec::new();
    let mut arms = Vec::new();
    for (variant, ty) in wrapping_variants(input, "ImapCommands")? {
        let ident = &variant.ident;

        let mut keyword = ident.to_string().to_uppercase();
        for attr in variant.attrs.iter().filter(|a| a.path().is_ident("imap")) {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("keyword") {
                    keyword = meta.value()?.parse::<LitStr>()?.value();
                    Ok(())
                } else {
                    Err(meta.error("unsupported imap attribute, expected `keyword`"))
                }
            })?;
        }

        from_impls.push(from_impl(name, ident, ty));
        arms.push(quote! {
            #keyword => #name::#ident(
                <#ty as crate::command::client_command::ClientCommandTrait>::parse_bytes(tag, cursor)?,
            ),
        });
    }

    Ok(quote! {
        #(#from_impls)*

        impl #name {
            /// Parses the rest of the command for `keyword` (already uppercased).
            fn dispatch(
                keyword: &str,
                tag: ::std::string::String,
                cursor: &mut crate::cursor::Cursor,
            ) -> ::core::result::Result<Self, crate::errors::CommandParseError> {
                ::core::result::Result::Ok(match keyword {
                    #(#arms)*
                    _ => {
                        return ::core::result::Result::Err(
                            crate::errors::CommandParseError::MalformedCommand(::core::option::Option::Some(
                                ::std::format!("unimplemented command {keyword:?}"),
                            )),
                        );
                    }
                })
            }
        }
    })
}
