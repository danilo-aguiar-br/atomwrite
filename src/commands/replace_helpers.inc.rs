fn compile_pattern(args: &ReplaceArgs) -> Result<Regex> {
    if args.pattern.is_empty() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "pattern must not be empty".to_string(),
        }
        .into());
    }

    let pattern_str = if args.literal || !args.regex {
        regex::escape(&args.pattern)
    } else {
        args.pattern.clone()
    };

    let pattern_str = if args.word {
        format!(r"\b{pattern_str}\b")
    } else {
        pattern_str
    };

    let pattern_str = if args.preserve_case {
        format!("(?i){pattern_str}")
    } else {
        pattern_str
    };

    Regex::new(&pattern_str).with_context(|| format!("invalid pattern: {}", args.pattern))
}

fn adapt_case(original: &str, replacement: &str) -> String {
    if original
        .chars()
        .all(|c| !c.is_alphabetic() || c.is_uppercase())
        && original.chars().any(|c| c.is_alphabetic())
    {
        replacement.to_uppercase()
    } else if original
        .chars()
        .all(|c| !c.is_alphabetic() || c.is_lowercase())
    {
        replacement.to_lowercase()
    } else if original.starts_with(|c: char| c.is_uppercase()) {
        let mut chars = replacement.chars();
        match chars.next() {
            Some(first) => {
                let mut s = first.to_uppercase().to_string();
                s.push_str(chars.as_str());
                s
            }
            None => String::new(),
        }
    } else {
        replacement.to_owned()
    }
}

fn apply_replacement<'a>(
    pattern: &Regex,
    content: &'a str,
    replacement: &str,
    max_replacements: Option<usize>,
    preserve_case: bool,
) -> (Cow<'a, str>, u64) {
    let count = pattern.find_iter(content).count() as u64;

    if count == 0 {
        return (Cow::Borrowed(content), 0);
    }

    if preserve_case {
        let limit = max_replacements.unwrap_or(usize::MAX);
        let mut result = String::with_capacity(content.len());
        let mut last_end = 0;
        let mut applied = 0u64;
        for m in pattern.find_iter(content) {
            if applied >= limit as u64 {
                break;
            }
            result.push_str(&content[last_end..m.start()]);
            result.push_str(&adapt_case(m.as_str(), replacement));
            last_end = m.end();
            applied += 1;
        }
        result.push_str(&content[last_end..]);
        return (Cow::Owned(result), applied);
    }

    let replaced = match max_replacements {
        Some(n) => {
            let actual_count = count.min(n as u64);
            let result = pattern.replacen(content, n, replacement);
            return (Cow::Owned(result.into_owned()), actual_count);
        }
        None => pattern.replace_all(content, replacement),
    };

    match replaced {
        Cow::Borrowed(_) => (Cow::Borrowed(content), 0),
        Cow::Owned(s) => (Cow::Owned(s), count),
    }
}

fn build_walker(
    args: &ReplaceArgs,
    canonical_paths: &[std::path::PathBuf],
    global: &GlobalArgs,
) -> Result<ignore::WalkBuilder> {
    let mut builder = ignore::WalkBuilder::new(&canonical_paths[0]);

    for path in canonical_paths.iter().skip(1) {
        builder.add(path);
    }

    builder
        .hidden(!global.hidden)
        .git_ignore(!global.no_gitignore)
        .follow_links(global.follow_symlinks);

    crate::concurrency::apply_walk_threads(&mut builder, global.threads);

    if !args.include.is_empty() || !args.exclude.is_empty() {
        let mut overrides = ignore::overrides::OverrideBuilder::new(&canonical_paths[0]);
        for glob in &args.include {
            overrides.add(glob)?;
        }
        for glob in &args.exclude {
            overrides.add(&format!("!{glob}"))?;
        }
        builder.overrides(overrides.build()?);
    }

    Ok(builder)
}
