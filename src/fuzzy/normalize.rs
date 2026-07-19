// SPDX-License-Identifier: MIT OR Apache-2.0

//! Unicode and escape normalization helpers for fuzzy matching.

/// Conditionally unescape `\t` / `\r` in `new` when the matched region has real tabs/CRs (Hermes).
pub(crate) fn maybe_unescape_new_string(new: &str, matched_region: &str) -> String {
    if !new.contains("\\t") && !new.contains("\\r") {
        return new.to_string();
    }
    let mut out = new.to_string();
    if out.contains("\\t") && matched_region.contains('\t') {
        out = out.replace("\\t", "\t");
    }
    if out.contains("\\r") && matched_region.contains('\r') {
        out = out.replace("\\r", "\r");
    }
    out
}

pub(crate) fn normalize_unicode_for_match(s: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    // G-FZZ-009/076/111/135: NFC + strip zero-width + smart-punctuation map + NFKC fullwidth fold.
    let nfc: String = s.nfc().collect();
    let stripped: String = nfc
        .chars()
        .filter(|c| !matches!(c, '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}' | '\u{2060}'))
        .collect();
    let mapped = stripped
        .replace(['—', '–'], "-")
        .replace(['\u{201c}', '\u{201d}'], "\"")
        .replace(['\u{2018}', '\u{2019}'], "'")
        .replace('…', "...")
        .replace('\u{00A0}', " ");
    mapped.nfkc().collect()
}
