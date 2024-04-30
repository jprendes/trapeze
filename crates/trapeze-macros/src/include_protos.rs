use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{bracketed, parse_macro_input, Error, LitStr};
use tempfile::tempdir;
use trapeze_codegen::Config;

use crate::inline_includes::inline_includes;

fn env_path(var: &str) -> Result<PathBuf> {
    Ok(PathBuf::from(env::var_os(var).with_context(|| {
        format!("Environment variable `{var}` not set")
    })?))
}

struct Array<T> {
    elems: Punctuated<T, Comma>,
}

impl<T> Default for Array<T> {
    fn default() -> Self {
        let elems = Punctuated::default();
        Self { elems }
    }
}

impl<T: Parse> Parse for Array<T> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        let _ = bracketed!(content in input);
        let elems = syn::punctuated::Punctuated::parse_terminated(&content)?;
        Ok(Self { elems })
    }
}

#[derive(Default)]
struct IncludeProtosInput {
    files: Array<LitStr>,
    includes: Option<Array<LitStr>>,
}

impl Parse for IncludeProtosInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let files = input.parse()?;
        let includes = None;
        if input.is_empty() {
            return Ok(Self { files, includes });
        }
        input.parse::<Comma>()?;
        if input.is_empty() {
            return Ok(Self { files, includes });
        }
        let includes = Some(input.parse()?);
        if input.is_empty() {
            return Ok(Self { files, includes });
        }
        input.parse::<Comma>()?;
        if input.is_empty() {
            return Ok(Self { files, includes });
        }
        Err(input.error("Unexpected token"))
    }
}

pub fn include_protos(input: TokenStream) -> TokenStream {
    let span = proc_macro2::TokenStream::from(input.clone()).span();
    let IncludeProtosInput { files, includes } = parse_macro_input!(input as IncludeProtosInput);

    include_protos_impl(files, includes).unwrap_or_else(|err| {
        let err = Error::new(span, err);
        err.into_compile_error().into()
    })
}

fn include_protos_impl(
    files: Array<LitStr>,
    includes: Option<Array<LitStr>>,
) -> Result<TokenStream> {
    let root = env_path("CARGO_MANIFEST_DIR")?;
    let out_dir = tempdir()?;

    let files: Vec<_> = files.elems.iter().map(|p| root.join(p.value())).collect();
    let mut includes: Vec<_> = match includes {
        Some(inc) => inc.elems.iter().map(|p| root.join(p.value())).collect(),
        None => files
            .iter()
            .map(|p| p.parent().unwrap().to_owned())
            .collect(),
    };

    includes.sort_unstable();
    includes.dedup();

    Config::new()
        .enable_type_names()
        .include_file("mod.rs")
        .out_dir(out_dir.path())
        .compile_protos(&files, &includes)?;

    let file = inline_includes(out_dir.path().join("mod.rs"))?;

    let tokens: proc_macro2::TokenStream = quote! {
        #file
    };

    Ok(tokens.into())
}