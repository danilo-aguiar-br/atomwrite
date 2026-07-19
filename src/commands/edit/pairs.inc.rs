fn find_str(haystack: &str, needle: &str) -> Option<usize> {
    memchr::memmem::find(haystack.as_bytes(), needle.as_bytes())
}

fn strip_file_trailing_newline(s: String) -> String {
    if s.ends_with("\r\n") {
        s[..s.len() - 2].to_string()
    } else if s.ends_with('\n') {
        s[..s.len() - 1].to_string()
    } else {
        s
    }
}

fn resolve_edit_pairs(
    args: &EditArgs,
    workspace: &Path,
    max_size: u64,
) -> Result<(Vec<String>, Vec<String>)> {
    if (!args.old.is_empty() && !args.new_file.is_empty())
        || (!args.old_file.is_empty() && !args.new.is_empty())
    {
        return Err(AtomwriteError::InvalidInput {
            reason: "cannot mix --old with --new-file or --old-file with --new; \
                     use both from the same source (--old/--new or --old-file/--new-file)"
                .into(),
        }
        .into());
    }
    if !args.old_file.is_empty() {
        if args.old_file.len() != args.new_file.len() {
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "--old-file count ({}) must match --new-file count ({})",
                    args.old_file.len(),
                    args.new_file.len()
                ),
            }
            .into());
        }
        let pairs: Vec<(&std::path::PathBuf, &std::path::PathBuf)> = args
            .old_file
            .iter()
            .zip(args.new_file.iter())
            .collect();
        let loaded: Vec<Result<(String, String), anyhow::Error>> =
            if crate::concurrency::should_parallelize(pairs.len()) {
                use rayon::prelude::*;
                pairs
                    .par_iter()
                    .map(|(of, nf)| {
                        let of_path = crate::path_safety::validate_path(of, workspace)?;
                        let nf_path = crate::path_safety::validate_path(nf, workspace)?;
                        // Independent dual-file I/O within each pair.
                        let (old_raw, new_raw) = rayon::join(
                            || crate::file_io::read_file_string(&of_path, max_size),
                            || crate::file_io::read_file_string(&nf_path, max_size),
                        );
                        Ok((
                            strip_file_trailing_newline(old_raw?),
                            strip_file_trailing_newline(new_raw?),
                        ))
                    })
                    .collect()
            } else {
                pairs
                    .iter()
                    .map(|(of, nf)| {
                        let of_path = crate::path_safety::validate_path(of, workspace)?;
                        let nf_path = crate::path_safety::validate_path(nf, workspace)?;
                        // Even single-pair: dual independent reads via join.
                        let (old_raw, new_raw) = rayon::join(
                            || crate::file_io::read_file_string(&of_path, max_size),
                            || crate::file_io::read_file_string(&nf_path, max_size),
                        );
                        Ok((
                            strip_file_trailing_newline(old_raw?),
                            strip_file_trailing_newline(new_raw?),
                        ))
                    })
                    .collect()
            };
        let mut olds = Vec::with_capacity(loaded.len());
        let mut news = Vec::with_capacity(loaded.len());
        for item in loaded {
            let (o, n) = item?;
            olds.push(o);
            news.push(n);
        }
        Ok((olds, news))
    } else {
        Ok((args.old.clone(), args.new.clone()))
    }
}
