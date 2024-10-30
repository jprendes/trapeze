use proc_macro::TokenStream;

mod as_client;
mod include_protos;
mod inline_includes;
mod service;

#[proc_macro]
pub fn include_protos(input: TokenStream) -> TokenStream {
    include_protos::include_protos(input).unwrap_or_else(|err| err.into_compile_error().into())
}

#[proc_macro]
pub fn service(input: TokenStream) -> TokenStream {
    service::service(input).unwrap_or_else(|err| err.into_compile_error().into())
}

#[proc_macro]
pub fn as_client(input: TokenStream) -> TokenStream {
    as_client::as_client(input).unwrap_or_else(|err| err.into_compile_error().into())
}
