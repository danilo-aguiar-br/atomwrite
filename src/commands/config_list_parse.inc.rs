// SPDX-License-Identifier: MIT OR Apache-2.0
// Included by config_cmd_keys.inc.rs (SRP split — keeps the key table under
// 500 lines while the list grammar lives next to the `Kind` it parses).

/// Parse the value of a [`Kind::List`] key.
///
/// Accepts a comma-separated list, with or without the surrounding brackets a
/// TOML array carries, so the operator can type the natural form and still get
/// an array in the file. An empty value is the empty list, which every reader
/// treats as "use the built-in default"; an empty ITEM is refused, because an
/// empty pattern would match every content and silently classify everything as
/// risky.
fn parse_list(kind_key: &str, raw: &str) -> Result<Parsed, AtomwriteError> {
    let inner = raw
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(raw);
    let mut items: Vec<String> = Vec::new();
    if !inner.trim().is_empty() {
        for part in inner.split(crate::constants::CONFIG_LIST_SEPARATOR) {
            let item = part.trim().trim_matches('"').trim_matches('\'').trim();
            if item.is_empty() {
                return Err(bad_value(
                    kind_key,
                    "a list of non-empty strings",
                    raw,
                ));
            }
            if item.len() > crate::constants::CONFIG_LIST_ITEM_MAX_BYTES {
                return Err(bad_value(
                    kind_key,
                    &format!(
                        "items of at most {} bytes",
                        crate::constants::CONFIG_LIST_ITEM_MAX_BYTES
                    ),
                    raw,
                ));
            }
            items.push(item.to_string());
        }
    }
    if items.len() > crate::constants::CONFIG_LIST_MAX_ITEMS {
        return Err(out_of_range(
            kind_key,
            raw,
            &format!("at most {} items", crate::constants::CONFIG_LIST_MAX_ITEMS),
        ));
    }
    let toml_text = toml_array_text(&items);
    Ok(Parsed {
        json: serde_json::json!(items),
        toml_text,
    })
}

/// Render the items as a TOML array literal.
///
/// Built by hand because an empty list has no single-item form to copy, and a
/// quote inside an item would otherwise end the string early.
fn toml_array_text(items: &[String]) -> String {
    let quoted: Vec<String> = items
        .iter()
        .map(|item| format!("\"{}\"", item.replace('\\', "\\\\").replace('"', "\\\"")))
        .collect();
    format!("[{}]", quoted.join(", "))
}
