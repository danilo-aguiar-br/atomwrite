fn validate_line_num(n: usize, total: usize) -> Result<usize> {
    if n == 0 || n > total {
        return Err(AtomwriteError::InvalidInput {
            reason: format!("line {n} out of range (file has {total} lines)"),
        }
        .into());
    }
    Ok(n - 1)
}

fn parse_range(s: &str, total: usize) -> Result<(usize, usize)> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!("invalid range format: expected N:M, got {s}"),
        }
        .into());
    }
    let start = parts[0]
        .parse::<usize>()
        .context("invalid range start")?
        .saturating_sub(1);
    let end = parts[1]
        .parse::<usize>()
        .context("invalid range end")?
        .min(total);

    if start >= end {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!("invalid range: start ({}) >= end ({})", start + 1, end),
        }
        .into());
    }

    Ok((start, end))
}

fn find_line_with(lines: &[&str], marker: &str) -> Result<usize> {
    for (i, line) in lines.iter().enumerate() {
        if line.contains(marker) {
            return Ok(i);
        }
    }
    Err(AtomwriteError::InvalidInput {
        reason: format!("marker not found: {marker:?}"),
    }
    .into())
}

fn find_line_with_after(lines: &[&str], marker: &str, after: usize) -> Result<usize> {
    for (i, line) in lines.iter().enumerate().skip(after) {
        if line.contains(marker) {
            return Ok(i);
        }
    }
    Err(AtomwriteError::InvalidInput {
        reason: format!("end marker not found after line {after}: {marker:?}"),
    }
    .into())
}

fn lines_from_str(s: &str) -> Vec<String> {
    s.lines().map(String::from).collect()
}

fn lines_to_owned(lines: &[&str]) -> Vec<String> {
    let mut v = Vec::with_capacity(lines.len());
    v.extend(lines.iter().map(|s| String::from(*s)));
    v
}

fn join_lines(lines: &[String]) -> String {
    let mut result = lines.join("\n");
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}
