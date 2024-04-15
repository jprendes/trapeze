use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use proc_macro::TokenStream;
use quote::quote;
use syn::Error;
use tempfile::tempdir;
use trapeze_codegen::Config;

mod inline_includes;
use inline_includes::inline_includes;

mod input;
use input::{parse_input, Input};

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
    } = match parse_input(input) {
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
