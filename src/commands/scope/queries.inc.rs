fn lookup_prepared_queries(name: &str, lang: &str) -> Result<Vec<String>> {
    match lang.to_lowercase().as_str() {
        "rust" | "rs" => lookup_rust_queries(name),
        "python" | "py" => lookup_python_query(name).map(|s| vec![s]),
        "javascript" | "js" | "typescript" | "ts" | "tsx" | "jsx" => {
            lookup_js_query(name).map(|s| vec![s])
        }
        "go" | "golang" => lookup_go_queries(name),
        _ => Err(AtomwriteError::InvalidInput {
            reason: format!(
                "no prepared queries for language: {lang}. \
                 Supported: rust, python, javascript, typescript, go"
            ),
        }
        .into()),
    }
}

fn lookup_rust_queries(name: &str) -> Result<Vec<String>> {
    let qs: Vec<&str> = match name {
        "comments" => vec!["// $$BODY\\s*", "/* $$$BODY */"],
        "strings" => vec!["\"$$$BODY\""],
        "fn" => vec![
            "pub fn $NAME($$$ARGS) -> $RET { $$$BODY }",
            "pub fn $NAME($$$ARGS) { $$$BODY }",
            "fn $NAME($$$ARGS) -> $RET { $$$BODY }",
            "fn $NAME($$$ARGS) { $$$BODY }",
        ],
        "pub-fn" => vec![
            "pub fn $NAME($$$ARGS) -> $RET { $$$BODY }",
            "pub fn $NAME($$$ARGS) { $$$BODY }",
        ],
        "async-fn" => vec![
            "pub async fn $NAME($$$ARGS) -> $RET { $$$BODY }",
            "pub async fn $NAME($$$ARGS) { $$$BODY }",
            "async fn $NAME($$$ARGS) -> $RET { $$$BODY }",
            "async fn $NAME($$$ARGS) { $$$BODY }",
        ],
        "unsafe-fn" => vec![
            "pub unsafe fn $NAME($$$ARGS) -> $RET { $$$BODY }",
            "pub unsafe fn $NAME($$$ARGS) { $$$BODY }",
            "unsafe fn $NAME($$$ARGS) -> $RET { $$$BODY }",
            "unsafe fn $NAME($$$ARGS) { $$$BODY }",
        ],
        "struct" => vec![
            "pub struct $NAME<$$$GEN> { $$$FIELDS }",
            "pub struct $NAME { $$$FIELDS }",
            "struct $NAME<$$$GEN> { $$$FIELDS }",
            "struct $NAME { $$$FIELDS }",
        ],
        "pub-struct" => vec![
            "pub struct $NAME<$$$GEN> { $$$FIELDS }",
            "pub struct $NAME { $$$FIELDS }",
        ],
        "enum" => vec![
            "pub enum $NAME<$$$GEN> { $$$VARIANTS }",
            "pub enum $NAME { $$$VARIANTS }",
            "enum $NAME<$$$GEN> { $$$VARIANTS }",
            "enum $NAME { $$$VARIANTS }",
        ],
        "pub-enum" => vec![
            "pub enum $NAME<$$$GEN> { $$$VARIANTS }",
            "pub enum $NAME { $$$VARIANTS }",
        ],
        "trait" => vec![
            "pub trait $NAME<$$$GEN> { $$$BODY }",
            "pub trait $NAME { $$$BODY }",
            "trait $NAME<$$$GEN> { $$$BODY }",
            "trait $NAME { $$$BODY }",
        ],
        "impl" => vec![
            "impl $TRAIT for $TYPE { $$$BODY }",
            "impl $TYPE { $$$BODY }",
        ],
        "mod" => vec!["pub mod $NAME { $$$BODY }", "mod $NAME { $$$BODY }"],
        "closure" => vec!["|$$$ARGS| $$$BODY"],
        "unsafe" => vec!["unsafe { $$$BODY }"],
        "use" => vec!["pub use $$$PATH;", "use $$$PATH;"],
        // GAP-134: test-fn pattern is multi-node (#[test] + fn) which
        // ast-grep rejects. Disabled until ast-grep supports composite patterns.
        "test-fn" => {
            return Err(AtomwriteError::InvalidInput {
                reason: "query 'test-fn' is currently unavailable: the pattern '#[test] fn ...' \
                         spans two AST nodes (attribute + function_item) which ast-grep does not \
                         support as a single pattern. Use 'atomwrite scope --pattern \"#[test]\"' \
                         to match test attributes, or 'atomwrite query -Q \
                         \"(function_item (attribute_item) @attr)\"' for tree-sitter queries."
                    .into(),
            }
            .into());
        }
        "attribute" => vec!["#[$$$ATTR]"],
        "return" => vec!["return $$$EXPR"],
        "match" => vec!["match $EXPR { $$$ARMS }"],
        "if-let" => vec!["if let $PAT = $EXPR { $$$BODY }"],
        "while-let" => vec!["while let $PAT = $EXPR { $$$BODY }"],
        "for" => vec!["for $PAT in $ITER { $$$BODY }"],
        "loop" => vec!["loop { $$$BODY }"],
        "const" => vec![
            "pub const $NAME: $TYPE = $$$EXPR;",
            "const $NAME: $TYPE = $$$EXPR;",
        ],
        "static" => vec![
            "pub static $NAME: $TYPE = $$$EXPR;",
            "static $NAME: $TYPE = $$$EXPR;",
        ],
        "type-alias" => vec!["pub type $NAME = $$$TYPE;", "type $NAME = $$$TYPE;"],
        "macro-rules" => vec!["macro_rules! $NAME { $$$BODY }"],
        "derive" => vec!["#[derive($$$TRAITS)]"],
        "doc-comment" => {
            return Err(AtomwriteError::InvalidInput {
                reason: "query 'doc-comment' is currently unavailable: tree-sitter parses \
                         '///' as a plain line_comment node (same as '//'), so ast-grep \
                         cannot distinguish doc-comments structurally. Use 'atomwrite scope \
                         --query comments' to match all comments, or 'rg \"///\"' for text \
                         matching."
                    .into(),
            }
            .into());
        }
        _ => {
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "unknown Rust query: {name}. Available: comments, strings, fn, pub-fn, \
                     async-fn, unsafe-fn, struct, pub-struct, enum, pub-enum, trait, impl, \
                     mod, closure, unsafe, use, attribute, return, match, if-let, \
                     while-let, for, loop, const, static, type-alias, macro-rules, derive, \
                     doc-comment. Note: test-fn is disabled (ast-grep multi-node limitation)"
                ),
            }
            .into());
        }
    };
    Ok(qs.into_iter().map(String::from).collect())
}

fn lookup_python_query(name: &str) -> Result<String> {
    let q = match name {
        "comments" => "# $$$BODY",
        "strings" => "\"$$$BODY\"",
        "class" => "class $NAME: $$$BODY",
        "def" => "def $NAME($$$ARGS): $$$BODY",
        "async-def" => "async def $NAME($$$ARGS): $$$BODY",
        "lambda" => "lambda $$$ARGS: $BODY",
        "import" => "import $$$NAMES",
        "from-import" => "from $MODULE import $$$NAMES",
        "with" => "with $EXPR as $NAME: $$$BODY",
        "for" => "for $VAR in $ITER: $$$BODY",
        "while" => "while $COND: $$$BODY",
        "decorator" => "@$NAME($$$ARGS)",
        "try-except" => "try: $$$BODY",
        _ => {
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "unknown Python query: {name}. Available: comments, strings, class, def, \
                     async-def, lambda, import, from-import, with, for, while, decorator, \
                     try-except"
                ),
            }
            .into());
        }
    };
    Ok(q.to_owned())
}

fn lookup_js_query(name: &str) -> Result<String> {
    let q = match name {
        "comments" => "// $$BODY\\s*",
        "strings" => "\"$$$BODY\"",
        "fn" => "function $NAME($$$ARGS) { $$$BODY }",
        "arrow-fn" => "const $NAME = ($$$ARGS) => $$$BODY",
        "class" => "class $NAME { $$$BODY }",
        "import" => "import $$$IMPORTS from \"$MODULE\"",
        "export" => "export $$$DECL",
        "async-fn" => "async function $NAME($$$ARGS) { $$$BODY }",
        "try-catch" => "try { $$$BODY } catch ($ERR) { $$$HANDLER }",
        "const" => "const $NAME = $$$EXPR",
        "let" => "let $NAME = $$$EXPR",
        _ => {
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "unknown JS/TS query: {name}. Available: comments, strings, fn, arrow-fn, \
                     class, import, export, async-fn, try-catch, const, let"
                ),
            }
            .into());
        }
    };
    Ok(q.to_owned())
}

fn lookup_go_queries(name: &str) -> Result<Vec<String>> {
    let qs: Vec<&str> = match name {
        "fn" => vec!["func $NAME($$$ARGS) $$$RET { $$$BODY }"],
        "struct" => vec!["type $NAME struct { $$$FIELDS }"],
        "interface" => vec!["type $NAME interface { $$$METHODS }"],
        "goroutine" => vec!["go $$$EXPR"],
        "defer" => vec!["defer $$$EXPR"],
        "import" => vec!["import $$$IMPORTS"],
        "const" => vec!["const $NAME = $$$EXPR"],
        "var" => vec!["var $NAME $TYPE = $$$EXPR", "var $NAME = $$$EXPR"],
        _ => {
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "unknown Go query: {name}. Available: fn, struct, interface, goroutine, \
                     defer, import, const, var"
                ),
            }
            .into());
        }
    };
    Ok(qs.into_iter().map(String::from).collect())
}
