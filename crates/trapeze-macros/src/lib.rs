use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Error};
use tempfile::tempdir;
use trapeze_codegen::Config;

mod inline_includes;
use inline_includes::inline_includes;

mod input;
use input::{parse, Input, PunctuatedList};

fn env_path(var: &str) -> Result<PathBuf> {
    Ok(PathBuf::from(env::var_os(var).with_context(|| {
        format!("Environment variable `{var}` not set")
    })?))
}

#[proc_macro]
pub fn include_protos(input: TokenStream) -> TokenStream {
    let Input {
        files,
        includes,
        span,
    } = match parse(input) {
        Ok(input) => input,
        Err(err) => return err.to_compile_error().into(),
    };

    include_protos_impl(&files, &includes).unwrap_or_else(|err| {
        let err = Error::new(span, err);
        err.into_compile_error().into()
    })
}

fn include_protos_impl(
    files: &[impl AsRef<Path>],
    includes: &[impl AsRef<Path>],
) -> Result<TokenStream> {
    let root = env_path("CARGO_MANIFEST_DIR")?;
    let out_dir = tempdir()?;

    let protos: Vec<_> = files.iter().map(|p| root.join(p)).collect();
    let includes: Vec<_> = includes.iter().map(|p| root.join(p)).collect();

    Config::new()
        .enable_type_names()
        .include_file("mod.rs")
        .out_dir(out_dir.path())
        .compile_protos(&protos, &includes)?;

    let file = inline_includes(out_dir.path().join("mod.rs"))?;

    let tokens: proc_macro2::TokenStream = quote! {
        #file
    };

    Ok(tokens.into())
}

#[proc_macro_attribute]
pub fn service(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = item.clone();
    let trait_names = parse_macro_input!(attr as PunctuatedList<syn::Path>).to_vec();
    let item_parsed = parse_macro_input!(item as syn::DeriveInput);

    let name = item_parsed.ident.clone();
    let (impl_generics, ty_generics, where_clause) = item_parsed.generics.split_for_impl();

    let tokens = quote! {
        #item_parsed

        impl #impl_generics trapeze::__codegen_prelude::Service for #name #ty_generics #where_clause {
            fn methods(self: std::sync::Arc<Self>) -> std::vec::Vec<(&'static str, std::sync::Arc<dyn trapeze::__codegen_prelude::MethodHandler + Send + Sync>)> {
                [#(#trait_names(self.clone()),)*].into_iter().flatten().collect()
            }
        }
    };

    tokens.into()
}
