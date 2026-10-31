// SPDX-License-Identifier: MIT OR Apache-2.0
// Pattern classification and S-expression validation of `query`.
// Included from `query.rs`.

/// Discriminator between legacy kind-filter matching and the new
/// real S-expression matching introduced in v0.1.19 (G122).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueryType {
    /// Plain kind name like `function_item` compared to `node.kind()`.
    KindFilter,
    /// S-expression with optional `@capture` bindings compiled via
    /// `tree_sitter::Query::new`.
    SExpression,
}

/// Detect whether a `--query <PATTERN>` argument is a plain kind
/// name or an S-expression. Presence of `(`, `)`, or `@` is the
/// unambiguous signal documented in ADR-0032.
fn classify_pattern(pattern: &str) -> QueryType {
    if pattern.contains('(') || pattern.contains(')') || pattern.contains('@') {
        QueryType::SExpression
    } else {
        QueryType::KindFilter
    }
}

/// Refuse an S-expression whose parentheses or quotes cannot match, before any
/// grammar is needed.
///
/// A query is compiled against a language, but an unbalanced one is wrong in
/// every language, so the caller is told the real cause even when the parser
/// for the requested language is not installed. Strings (`"..."`) and `;`
/// comments are skipped.
fn check_sexpr_balance(pattern: &str) -> Result<(), crate::error::AtomwriteError> {
    const BACKSLASH: char = '\u{5c}';
    let invalid =
        |cause: String| crate::error::AtomwriteError::invalid("invalid S-expression", cause);
    let mut depth = 0usize;
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => loop {
                match chars.next() {
                    Some(BACKSLASH) => {
                        chars.next();
                    }
                    Some('"') => break,
                    Some(_) => {}
                    None => return Err(invalid("unterminated string".into())),
                }
            },
            ';' => {
                for next in chars.by_ref() {
                    if next == '\n' {
                        break;
                    }
                }
            }
            '(' | '[' => depth += 1,
            ')' | ']' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| invalid("unbalanced closing parenthesis".into()))?;
            }
            _ => {}
        }
    }
    if depth == 0 {
        Ok(())
    } else {
        Err(invalid(format!("{depth} unclosed parenthesis")))
    }
}
