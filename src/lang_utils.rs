// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared language utilities for AST commands (scope, transform).

/// Return file extensions for a given programming language name.
pub fn lang_extensions(lang: &str) -> Vec<&'static str> {
    match lang.to_lowercase().as_str() {
        "rust" | "rs" => vec!["rs"],
        "javascript" | "js" => vec!["js", "jsx", "mjs"],
        "typescript" | "ts" => vec!["ts", "tsx", "mts"],
        "python" | "py" => vec!["py", "pyi"],
        "go" => vec!["go"],
        "java" => vec!["java"],
        "c" => vec!["c", "h"],
        "cpp" | "c++" => vec!["cpp", "hpp", "cc", "hh", "cxx"],
        "csharp" | "c#" | "cs" => vec!["cs"],
        "ruby" | "rb" => vec!["rb"],
        "swift" => vec!["swift"],
        "kotlin" | "kt" => vec!["kt", "kts"],
        "lua" => vec!["lua"],
        "html" => vec!["html", "htm"],
        "css" => vec!["css"],
        "json" => vec!["json"],
        "yaml" | "yml" => vec!["yaml", "yml"],
        "toml" => vec!["toml"],
        _ => vec![],
    }
}
