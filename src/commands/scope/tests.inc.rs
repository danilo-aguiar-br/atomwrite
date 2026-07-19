#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titlecase_basic() {
        assert_eq!(titlecase("hello world"), "Hello World");
    }

    #[test]
    fn titlecase_underscore() {
        assert_eq!(titlecase("foo_bar"), "Foo_Bar");
    }

    #[test]
    fn squeeze_whitespace() {
        assert_eq!(squeeze("a  b   c"), "a b c");
    }

    #[test]
    fn squeeze_preserves_non_whitespace() {
        assert_eq!(squeeze("aabbcc"), "aabbcc");
    }

    #[test]
    fn lookup_rust_queries_known() {
        let result = lookup_rust_queries("fn");
        assert!(result.is_ok());
        let qs = result.unwrap();
        assert_eq!(
            qs.len(),
            4,
            "fn should produce 4 patterns (pub/non-pub × with/without return type)"
        );
        assert!(qs.iter().all(|q| q.contains("fn $NAME")));
    }

    #[test]
    fn lookup_rust_queries_comments_multi() {
        let result = lookup_rust_queries("comments");
        assert!(result.is_ok());
        let qs = result.unwrap();
        assert_eq!(
            qs.len(),
            2,
            "comments should produce 2 patterns (line + block)"
        );
    }

    #[test]
    fn lookup_rust_queries_unknown() {
        let result = lookup_rust_queries("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn lookup_js_query_known() {
        let result = lookup_js_query("class");
        assert!(result.is_ok());
    }

    #[test]
    fn lookup_go_queries_known() {
        let result = lookup_go_queries("struct");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
        let var_result = lookup_go_queries("var");
        assert!(var_result.is_ok());
        assert_eq!(
            var_result.unwrap().len(),
            2,
            "var should have typed + inferred patterns"
        );
    }

    #[test]
    fn resolve_patterns_custom() {
        let result = resolve_patterns(&None, &Some("custom_pattern".into()), "rust");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec!["custom_pattern"]);
    }

    #[test]
    fn resolve_patterns_requires_query_or_pattern() {
        let result = resolve_patterns(&None, &None, "rust");
        assert!(result.is_err());
    }

    #[test]
    fn apply_scope_action_delete() {
        assert_eq!(apply_scope_action("hello", true, None, None), "");
    }

    #[test]
    fn apply_scope_action_upper() {
        assert_eq!(
            apply_scope_action("hello", false, Some(ScopeAction::Upper), None),
            "HELLO"
        );
    }

    #[test]
    fn apply_scope_action_replace() {
        assert_eq!(apply_scope_action("old", false, None, Some("new")), "new");
    }
}
