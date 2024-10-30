use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::{parse, Token};

struct AsClientInput {
    expr: syn::Expr,
    traits: Punctuated<syn::Path, Token![+]>,
}

impl Parse for AsClientInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let expr = input.parse()?;
        let _ = input.parse::<Token![:]>()?;
        let traits = syn::punctuated::Punctuated::parse_terminated(input)?;
        Ok(Self { expr, traits })
    }
}

pub fn as_client(input: TokenStream) -> syn::Result<TokenStream> {
    let AsClientInput { expr, traits, .. } = parse(input)?;
    let out = quote! {
        {
            {
                fn as_client(client: impl AsRef<Client>) -> impl trapeze::ClientExt + #traits {
                    client.as_ref().clone()
                }
                as_client(#expr)
            }
        }
    };

    Ok(out.into())
}
