// SPDX-License-Identifier: MIT OR Apache-2.0
// The `KEYS` block of the `config --help` page, built from `KEYS` itself.
// Included from config_cmd_keys.inc.rs.

impl Kind {
    /// Column of the `--help` table that names the type a value must have.
    ///
    /// Different from `label`, which is the wire name `config list` reports:
    /// there `Choice` and `AbsolutePath` both read `string`, and the help page
    /// says `choice` and `path`, because that is what the user types.
    fn help_type(&self) -> &'static str {
        match self {
            Kind::Integer { .. } => "integer",
            Kind::Float { .. } => "float",
            Kind::Boolean => "boolean",
            Kind::Choice(_) => "choice",
            Kind::AbsolutePath => "path",
            Kind::Locale => "locale",
            Kind::List => "list",
        }
    }

    /// Column of the `--help` table that names the accepted values.
    fn accepted(&self) -> String {
        match *self {
            Kind::Integer { min, max } => {
                if min == 1 && max == INT_MAX {
                    "1 or more".to_string()
                } else {
                    format!("{min} to {max}")
                }
            }
            Kind::Float { min, max } => format!("{min} to {max}"),
            Kind::Boolean => "true, false".to_string(),
            Kind::Choice(words) => words.join(", "),
            Kind::AbsolutePath => "absolute directory path".to_string(),
            Kind::Locale => "en, pt-BR".to_string(),
            Kind::List => format!(
                "comma separated, at most {} items",
                crate::constants::CONFIG_LIST_MAX_ITEMS
            ),
        }
    }
}

impl Scopes {
    /// Column of the `--help` table that names where the key may be written.
    fn label(&self) -> &'static str {
        match self {
            Scopes::Both => "both",
            Scopes::GlobalOnly => "global",
        }
    }
}

/// The table of `config --help`, generated from `KEYS` row by row.
///
/// The page used to carry a hand-written copy of this table, and the copy
/// drifted: 22 rows against the 35 the verb answers. Generating the page from
/// the table removes the second source of truth, so a key added here shows up
/// in `--help` in the same edit that makes it writable.
pub fn keys_help_block() -> String {
    let mut out = String::from("KEYS (key, type, scope, accepted values):\n");
    for spec in KEYS {
        out.push_str(&format!(
            "  {:<33} {:<8} {:<7} {}\n",
            spec.name(),
            spec.kind.help_type(),
            spec.scopes.label(),
            spec.kind.accepted(),
        ));
    }
    out
}
