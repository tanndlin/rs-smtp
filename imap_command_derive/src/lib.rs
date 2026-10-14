use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, LitStr, parse_macro_input};

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

fn expand(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            name,
            "ImapCommands can only be derived on an enum",
        ));
    };

    let mut from_impls = Vec::new();
    let mut arms = Vec::new();
    for variant in &data.variants {
        let ident = &variant.ident;
        let ty = match &variant.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => &fields.unnamed[0].ty,
            _ => {
                return Err(syn::Error::new_spanned(
                    variant,
                    "each variant must wrap exactly one command type, e.g. `Noop(NoopCommand)`",
                ));
            }
        };

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

        from_impls.push(quote! {
            impl ::core::convert::From<#ty> for #name {
                fn from(cmd: #ty) -> Self {
                    #name::#ident(cmd)
                }
            }
        });
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
