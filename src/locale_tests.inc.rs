mod tests {
    use super::*;

    #[test]
    fn normalize_underscore_and_encoding() {
        assert_eq!(normalize_bcp47_tag("pt_BR.UTF-8"), "pt-BR");
        assert_eq!(normalize_bcp47_tag("en_US.utf8"), "en-US");
        assert_eq!(normalize_bcp47_tag("C"), "");
        assert_eq!(normalize_bcp47_tag("POSIX"), "");
        assert_eq!(normalize_bcp47_tag("C.UTF-8"), "");
    }

    #[test]
    fn parse_langid_accepts_normalized() {
        let id = parse_langid("pt_BR.UTF-8").expect("parse");
        assert_eq!(id.language.as_str(), "pt");
        assert_eq!(
            id.region.as_ref().map(|r| r.as_str()),
            Some("BR")
        );
    }

    #[test]
    fn negotiate_pt_to_pt_br() {
        assert_eq!(negotiate_to_idioma(&["pt"]), Idioma::PtBr);
        assert_eq!(negotiate_to_idioma(&["pt-BR"]), Idioma::PtBr);
        assert_eq!(negotiate_to_idioma(&["pt_BR"]), Idioma::PtBr);
        assert_eq!(negotiate_to_idioma(&["en-GB"]), Idioma::En);
        assert_eq!(negotiate_to_idioma(&["ja-JP"]), Idioma::En);
    }

    #[test]
    fn resolve_cli_beats_system() {
        let state = resolve_locale(
            Some("en"),
            Some("pt-BR".into()),
            None,
        );
        assert_eq!(state.idioma, Idioma::En);
        assert_eq!(state.source, LocaleSource::CliFlag);
    }

    #[test]
    fn resolve_system_pt() {
        let state = resolve_locale(None, Some("pt_BR.UTF-8".into()), None);
        assert_eq!(state.idioma, Idioma::PtBr);
        assert_eq!(state.source, LocaleSource::System);
        assert!(!state.detection_failed);
    }

    #[test]
    fn resolve_c_locale_falls_to_default() {
        let state = resolve_locale(None, Some("C.UTF-8".into()), None);
        assert_eq!(state.idioma, Idioma::En);
        assert_eq!(state.source, LocaleSource::Default);
        assert!(state.detection_failed);
    }

    #[test]
    fn resolve_persisted_beats_system() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("locale");
        fs::write(&path, "pt-BR\n").unwrap();
        let state = resolve_locale(None, Some("en-US".into()), Some(path));
        assert_eq!(state.idioma, Idioma::PtBr);
        assert_eq!(state.source, LocaleSource::Persisted);
    }

    #[test]
    fn parse_cli_locale_rejects_unknown_language() {
        assert!(parse_cli_locale("ja-JP").is_err());
        assert_eq!(parse_cli_locale("pt_BR").unwrap(), "pt-BR");
        assert_eq!(parse_cli_locale("en").unwrap(), "en");
    }

    #[test]
    fn idioma_methods() {
        assert_eq!(Idioma::PtBr.fallback(), Idioma::En);
        assert_eq!(Idioma::En.direcao(), TextDirection::Ltr);
        assert_eq!(Idioma::AVAILABLE.len(), 2);
    }
}
