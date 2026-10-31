// SPDX-License-Identifier: MIT OR Apache-2.0

//! Dotted-key navigation inside a TOML document.
//!
//! `toml_edit::DocumentMut::get` treats a dotted key as one literal name, so
//! `get`, `set` and `del` walked the document themselves. Each verb carried its
//! own copy of that walk, and every copy descended with `Item::as_table`, which
//! is `None` for an inline table: `pool = { max = 5 }` failed with exit 65 even
//! though the key exists.
//!
//! This module is the single walk. It descends with [`Item::as_table_like`],
//! which covers both a `[table]` and a `{ inline = table }`, and it keeps the
//! document's own shape: a key created inside an inline table is created inline,
//! because a standard table cannot live inside an inline one.

use toml_edit::{Item, Value};

/// Split a dotted key into its segments, dropping empty ones.
///
/// `a..b` and a leading or trailing dot name the same path as `a.b`, because an
/// empty segment is not a key in TOML.
#[must_use]
pub(crate) fn segments(key_path: &str) -> Vec<&str> {
    key_path.split('.').filter(|s| !s.is_empty()).collect()
}

/// Descend to the item a dotted path names.
#[must_use]
pub(crate) fn resolve<'a>(root: &'a Item, segs: &[&str]) -> Option<&'a Item> {
    let mut current = root;
    for seg in segs {
        current = current.as_table_like()?.get(seg)?;
    }
    Some(current)
}

/// Descend to the item a dotted path names, mutably.
pub(crate) fn resolve_mut<'a>(root: &'a mut Item, segs: &[&str]) -> Option<&'a mut Item> {
    let mut current = root;
    for seg in segs {
        current = current.as_table_like_mut()?.get_mut(seg)?;
    }
    Some(current)
}

/// Render an item the way the verbs publish it: a string without its quotes,
/// anything else through its own `Display` with the surrounding space trimmed.
#[must_use]
pub(crate) fn render(item: &Item) -> String {
    match item.as_value() {
        Some(Value::String(s)) => s.value().to_owned(),
        Some(value) => value.to_string().trim().to_owned(),
        None => item.to_string().trim().to_owned(),
    }
}

/// Read the value at `key_path`, if the path resolves.
#[must_use]
pub(crate) fn get(doc: &toml_edit::DocumentMut, key_path: &str) -> Option<String> {
    let segs = segments(key_path);
    if segs.is_empty() {
        return None;
    }
    resolve(doc.as_item(), &segs).map(render)
}

/// Write `value` at `key_path`, creating the intermediate tables it needs.
///
/// The parent of the leaf must already be a table (standard or inline); a
/// scalar in the middle of the path is the caller's mistake and keeps the
/// message the verbs already published. A table created under an inline table
/// is itself inline, because TOML has no standard table inside an inline one.
///
/// # Errors
///
/// Returns [`crate::error::AtomwriteError::InvalidInput`] when a segment before
/// the leaf names a value instead of a table.
pub(crate) fn set(
    doc: &mut toml_edit::DocumentMut,
    key_path: &str,
    value: Item,
) -> Result<(), crate::error::AtomwriteError> {
    let segs = segments(key_path);
    let Some((leaf, parents)) = segs.split_last() else {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "cannot set an empty key path".into(),
        });
    };

    let mut current = doc.as_item_mut();
    for (i, seg) in parents.iter().enumerate() {
        let inline_parent = current.as_inline_table().is_some();
        let Some(table) = current.as_table_like_mut() else {
            let traversed = segs[..=i].join(".");
            return Err(crate::error::AtomwriteError::InvalidInput {
                reason: format!("cannot set '{key_path}': '{traversed}' is not a table"),
            });
        };
        if !table.contains_key(seg) {
            let child = if inline_parent {
                Item::Value(Value::InlineTable(toml_edit::InlineTable::new()))
            } else {
                Item::Table(toml_edit::Table::new())
            };
            table.insert(seg, child);
        }
        let Some(child) = table.get_mut(seg) else {
            // Unreachable while `contains_key` above holds, but a typed error
            // keeps the production path free of panics if that ever changes.
            return Err(crate::error::AtomwriteError::InternalError {
                reason: format!("cannot set '{key_path}': '{seg}' vanished while walking"),
            });
        };
        current = child;
    }

    let Some(table) = current.as_table_like_mut() else {
        let parent_path = parents.join(".");
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!(
                "cannot set '{key_path}': '{}' is a scalar value, not a table",
                parent_path
            ),
        });
    };
    match table.get_mut(leaf) {
        Some(existing) => assign_keeping_decor(existing, value),
        None => {
            table.insert(leaf, value);
        }
    }
    Ok(())
}

/// Write `new` at `item`, carrying over the decoration of the value it replaces.
///
/// Decoration is the whitespace and comment a value carries (`key = 1 # note`).
/// Replacing the value without it would rewrite every line around the edit for
/// reasons the caller never asked for.
fn assign_keeping_decor(item: &mut Item, mut new: Item) {
    if let (Some(old), Some(new_value)) = (item.as_value(), new.as_value_mut()) {
        *new_value.decor_mut() = old.decor().clone();
    }
    *item = new;
}

/// Remove the value at `key_path` and return it rendered, if it existed.
pub(crate) fn remove(doc: &mut toml_edit::DocumentMut, key_path: &str) -> Option<String> {
    let segs = segments(key_path);
    let (leaf, parents) = segs.split_last()?;
    let parent = resolve_mut(doc.as_item_mut(), parents)?;
    let table = parent.as_table_like_mut()?;
    let rendered = table.get(leaf).map(render);
    table.remove(leaf);
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> toml_edit::DocumentMut {
        text.parse().expect("valid TOML")
    }

    #[test]
    fn reads_a_standard_table() {
        let doc = parse("[package]\nname = \"x\"\n");
        assert_eq!(get(&doc, "package.name").as_deref(), Some("x"));
    }

    #[test]
    fn reads_an_inline_table() {
        let doc = parse("pool = { max = 5 }\n");
        assert_eq!(get(&doc, "pool.max").as_deref(), Some("5"));
    }

    #[test]
    fn reads_an_inline_table_nested_twice() {
        let doc = parse("a = { b = { c = \"deep\" } }\n");
        assert_eq!(get(&doc, "a.b.c").as_deref(), Some("deep"));
    }

    #[test]
    fn writes_into_an_inline_table_keeping_the_form() {
        let mut doc = parse("pool = { max = 5 }\n");
        set(&mut doc, "pool.max", toml_edit::value(9)).expect("set");
        assert_eq!(get(&doc, "pool.max").as_deref(), Some("9"));
        assert!(doc.to_string().contains("pool = {"));
    }

    #[test]
    fn creates_a_new_key_inline_when_the_parent_is_inline() {
        let mut doc = parse("pool = { max = 5 }\n");
        set(&mut doc, "pool.min", toml_edit::value(1)).expect("set");
        assert_eq!(get(&doc, "pool.min").as_deref(), Some("1"));
        assert!(doc.to_string().contains("pool = {"));
    }

    #[test]
    fn creates_a_standard_table_when_the_parent_is_standard() {
        let mut doc = parse("[a]\nb = 1\n");
        set(&mut doc, "a.c.d", toml_edit::value("x")).expect("set");
        assert_eq!(get(&doc, "a.c.d").as_deref(), Some("x"));
        assert!(doc.to_string().contains("[a.c]"));
    }

    #[test]
    fn refuses_a_scalar_in_the_middle_of_the_path() {
        let mut doc = parse("package = 1\n");
        let err = set(&mut doc, "package.name", toml_edit::value("x")).expect_err("scalar parent");
        assert!(err.to_string().contains("not a table"), "{err}");
    }

    #[test]
    fn removes_from_an_inline_table() {
        let mut doc = parse("pool = { max = 5, min = 1 }\n");
        assert_eq!(remove(&mut doc, "pool.max").as_deref(), Some("5"));
        assert_eq!(get(&doc, "pool.max"), None);
        assert_eq!(get(&doc, "pool.min").as_deref(), Some("1"));
    }

    #[test]
    fn empty_segments_are_dropped() {
        assert_eq!(segments("a..b."), vec!["a", "b"]);
    }
}
