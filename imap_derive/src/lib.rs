use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
    Data, DeriveInput, Fields, Ident, LitStr, Type, Variant, parse_macro_input, spanned::Spanned,
};

/// Derived on the `ClientCommand` enum. Each variant must wrap a single command
/// struct implementing `ClientCommandTrait` and having a `tag: String` field
/// visible to the enum's module. Generates a `From` impl per variant, a
/// `ClientCommand::dispatch` that parses by keyword, and a `ClientCommand::tag`
/// accessor. The keyword defaults to the uppercased variant name and can be
/// overridden with `#[imap(keyword = "...")]`.
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
    let mut parse_bytes_arms = Vec::new();
    let mut tag_arms = Vec::new();
    let mut allowed_arms = Vec::new();
    for (variant, ty) in wrapping_variants(input, "ImapCommands")? {
        let ident = &variant.ident;

        let mut keyword = ident.to_string().to_uppercase();
        let mut states: Option<Vec<Ident>> = None;
        for attr in variant.attrs.iter().filter(|a| a.path().is_ident("imap")) {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("keyword") {
                    keyword = meta.value()?.parse::<LitStr>()?.value();
                    Ok(())
                } else if meta.path.is_ident("states") {
                    let list = states.get_or_insert_with(Vec::new);
                    meta.parse_nested_meta(|state| {
                        list.push(state.path.require_ident()?.clone());
                        Ok(())
                    })
                } else {
                    Err(meta.error("unsupported imap attribute, expected `keyword` or `states`"))
                }
            })?;
        }
        let Some(states) = states else {
            return Err(syn::Error::new_spanned(
                variant,
                "missing `#[imap(states(...))]`: list the session states this command is valid in",
            ));
        };

        from_impls.push(from_impl(name, ident, ty));
        parse_bytes_arms.push(quote! {
            #keyword => #name::#ident(
                <#ty as crate::command::client_command::ClientCommandTrait>::parse_bytes(tag, cursor)?,
            ),
        });
        // Spanned to the wrapped type so a missing or private `tag` field is
        // reported on the variant rather than inside the macro output.
        let tag_access = quote_spanned! {ty.span()=> &c.tag };
        tag_arms.push(quote! {
            #name::#ident(c) => #tag_access,
        });

        allowed_arms.push(quote! {
            #name::#ident(_) => ::core::matches!(
                state,
                #(crate::imap_session::SessionState::#states)|*
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
                    #(#parse_bytes_arms)*
                    _ => {
                        return ::core::result::Result::Err(
                            crate::errors::CommandParseError::MalformedCommand(::core::option::Option::Some(
                                ::std::format!("unimplemented command {keyword:?}"),
                            )),
                        );
                    }
                })
            }

            /// Whether this command may be issued in `state` (RFC 3501 §6).
            pub fn allowed_in(&self, state: crate::imap_session::SessionState) -> bool {
                match self {
                    #(#allowed_arms)*
                }
            }

            pub fn tag(&self) -> &str {
                match self {
                    #(#tag_arms)*
                }
            }
        }
    })
}
