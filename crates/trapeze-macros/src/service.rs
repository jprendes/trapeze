use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::{parse, Token};

struct ServiceInput {
    expr: syn::Expr,
    traits: Punctuated<syn::Path, Token![+]>,
}

impl Parse for ServiceInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let expr = input.parse()?;
        let _ = input.parse::<Token![:]>()?;
        let traits = syn::punctuated::Punctuated::parse_terminated(input)?;
        Ok(Self { expr, traits })
    }
}

pub fn service(input: TokenStream) -> syn::Result<TokenStream> {
    let ServiceInput { expr, traits, .. } = parse(input)?;
    let traits_vec = traits.iter().cloned().collect::<Vec<_>>();
    let out = quote! {
        {
            struct Service<T: #traits> {
                target: std::sync::Arc<T>
            }
            impl<T: #traits> Service<T> {
                pub fn new(target: impl std::convert::Into<std::sync::Arc<T>>) -> Self {
                    let target = target.into();
                    Self { target }
                }
            }
            impl<T: #traits> trapeze::__codegen_prelude::Service for Service<T> {
                fn methods(&self) -> std::vec::Vec<(&'static str, std::sync::Arc<dyn trapeze::__codegen_prelude::MethodHandler + Send + Sync>)> {
                    let target = &self.target;
                    [#(#traits_vec::<T>(target.clone()).methods(),)*].into_iter().flatten().collect()
                }
            }
            Service::new(#expr)
        }
    };

    Ok(out.into())
}
