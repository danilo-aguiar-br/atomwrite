// SPDX-License-Identifier: MIT OR Apache-2.0

//! Text of a PDF, one page at a time.
//!
//! A PDF is a container, not a text file: the bytes on disk are object streams,
//! and a reader that dumps them returns a header and nothing a model can use.
//! This module decompresses the page contents with a hard ceiling, returns the
//! text of each requested page, and marks a page that carries no text at all,
//! because a scanned page is an instruction to look at the picture and not a
//! statement that the document is empty.

use std::path::Path;

use anyhow::{Context, Result};
use rayon::prelude::*;

/// One page of text, in the order the caller asked for it.
#[derive(Debug, Clone)]
pub(crate) struct PageText {
    /// 1-based page number, as the caller names it.
    pub page: u32,
    /// Text the page carries, empty when the page has none.
    pub text: String,
}

impl PageText {
    /// Whether the page carries no text and therefore needs the drawn page.
    pub(crate) fn needs_vision(&self) -> bool {
        self.text.trim().is_empty()
    }
}

/// Number of pages in the document.
///
/// # Errors
///
/// Returns an error when the file cannot be parsed as a PDF.
pub(crate) fn page_count(path: &Path) -> Result<u32> {
    let document = lopdf::Document::load(path)
        .with_context(|| format!("cannot parse {} as a PDF", path.display()))?;
    Ok(document.get_pages().len() as u32)
}

/// Extract the text of every requested page, in request order.
///
/// # Errors
///
/// Returns an error when the file cannot be parsed, or when a page number is
/// outside the document. A page whose content stream exceeds the decompressed
/// ceiling reports the ceiling as its error rather than inflating the process.
pub(crate) fn extract_pages(
    path: &Path,
    pages: &[u32],
    max_decompressed_bytes: usize,
) -> Result<Vec<PageText>> {
    let document = lopdf::Document::load(path)
        .with_context(|| format!("cannot parse {} as a PDF", path.display()))?;
    let total = document.get_pages().len() as u32;
    if let Some(outside) = pages.iter().find(|page| **page == 0 || **page > total) {
        anyhow::bail!(
            "page {outside} is outside {} (the document has {total} pages)",
            path.display()
        );
    }
    // Pages are independent decompressions, so they fan out and the join puts
    // them back in the order the caller asked for.
    let extracted: Vec<Result<PageText>> = pages
        .par_iter()
        .map(|page| {
            let chunks = document.extract_text_chunks_with_limit(&[*page], max_decompressed_bytes);
            let mut text = String::new();
            for chunk in chunks {
                let chunk = chunk.with_context(|| {
                    format!("cannot decompress page {page} of {}", path.display())
                })?;
                text.push_str(&chunk);
            }
            Ok(PageText { page: *page, text })
        })
        .collect();
    extracted.into_iter().collect()
}

/// Page numbers `first` through `last`, clamped to the document.
///
/// A range the caller writes backwards is read forwards, so `3:1` means the
/// same three pages as `1:3` and never an empty answer.
pub(crate) fn page_range(first: u32, last: u32, total: u32) -> Vec<u32> {
    let low = first.max(1).min(total.max(1));
    let high = last.max(1).min(total.max(1)).max(low);
    (low..=high).collect()
}
