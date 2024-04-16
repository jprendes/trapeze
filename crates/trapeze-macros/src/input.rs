use std::collections::BTreeSet;
use std::path::PathBuf;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{bracketed, parse_quote, Error, LitStr, Result, Token};

struct BracketedList {
    values: Punctuated<LitStr, Token![,]>,
}

impl BracketedList {
    fn to_vec(&self) -> Vec<PathBuf> {
        self.values
            .iter()
            .map(|f| PathBuf::from(f.value()))
            .collect()
    }
}

impl Parse for BracketedList {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let _ = bracketed!(content in input);
        Ok(BracketedList {
            values: Punctuated::<LitStr, Token![,]>::parse_terminated(&content)?,
        })
    }
}

pub struct Input {
    pub files: Vec<PathBuf>,
    pub includes: Vec<PathBuf>,
    pub span: Span,
}

pub fn parse(input: TokenStream) -> Result<Input> {
    let input: proc_macro2::TokenStream = input.into();
    let span = input.span();
    let files: Punctuated<BracketedList, Token![,]> = parse_quote! { #input };
    let lists: Vec<_> = files.iter().collect();
    let (files, includes) = match lists.as_slice() {
        [files] => {
            let files = files.to_vec();
            let includes = files
                .iter()
                .map(|p| p.parent().unwrap().to_owned())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            (files, includes)
        }
        [files, includes] => (files.to_vec(), includes.to_vec()),
        _ => {
            return Err(Error::new(
                input.span(),
                "Expected a list of files to compile",
            ));
        }
    };
    Ok(Input {
        files,
        includes,
        span,
    })
}
