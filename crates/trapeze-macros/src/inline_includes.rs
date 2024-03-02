use std::fs::read_to_string;
use std::path::PathBuf;

use anyhow::{Context, Result};
use syn::parse_str;
use syn::visit_mut::{visit_item_mut, VisitMut};
use syn::{File, Item, LitStr};

struct InlineIncludesVisitor {
    out_dir: PathBuf,
    error: Option<anyhow::Error>,
}

impl InlineIncludesVisitor {
    fn resolve_item(&mut self, item: &mut Item) -> Result<()> {
        let Item::Macro(mac) = item else {
            return Ok(());
        };

        let mac = &mac.mac;
        if !mac.path.is_ident("include") {
            return Ok(());
        }
        let path: LitStr =
            syn::parse2(mac.tokens.clone()).context("Expected single literal string")?;

        let buffer = read_to_string(self.out_dir.join(path.value()))?;
        let content: proc_macro2::TokenStream = parse_str(&buffer)?;

        *item = Item::Verbatim(content);

        Ok(())
    }
}

impl VisitMut for InlineIncludesVisitor {
    fn visit_item_mut(&mut self, item: &mut Item) {
        if self.error.is_some() {
            return;
        }

        if let Err(err) = self.resolve_item(item) {
            self.error = Some(err);
        }

        visit_item_mut(self, item);
    }
}

pub fn inline_includes(file: PathBuf) -> Result<File> {
    let out_dir = file.parent().context("File has no parent")?.to_owned();
    let mut resolver = InlineIncludesVisitor {
        out_dir,
        error: None,
    };
    let file = read_to_string(file)?;
    let mut file: File = parse_str(&file)?;

    resolver.visit_file_mut(&mut file);

    if let Some(err) = resolver.error {
        return Err(err);
    }

    Ok(file)
}
