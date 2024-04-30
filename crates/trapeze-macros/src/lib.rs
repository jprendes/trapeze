use proc_macro::TokenStream;

mod as_client;
mod service;
mod include_protos;
mod inline_includes;

#[proc_macro]
pub fn include_protos(input: TokenStream) -> TokenStream {
    include_protos::include_protos(input)
}

#[proc_macro]
pub fn service(input: TokenStream) -> TokenStream {
    service::service(input)
}

#[proc_macro]
pub fn as_client(input: TokenStream) -> TokenStream {
    as_client::as_client(input)
}
