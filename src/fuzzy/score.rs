// SPDX-License-Identifier: MIT OR Apache-2.0

//! Similarity scoring: gestalt (Ratcliff/Obershelp) and line-vote (Hermes).
//!
//! Workload: CPU-bound. Prefer calling only after cheap prefilters.
//! Memory: stack-friendly char vectors; no process-wide heap caches.

/// Ratcliff/Obershelp (gestalt) ratio — Hermes SequenceMatcher-compatible (G-FZZ-077).
pub(crate) fn gestalt_ratio(a: &str, b: &str) -> f64 {
    let aa: Vec<char> = a.chars().collect();
    let bb: Vec<char> = b.chars().collect();
    if aa.is_empty() && bb.is_empty() {
        return 1.0;
    }
    if aa.is_empty() || bb.is_empty() {
        return 0.0;
    }
    let m = matching_blocks_len(&aa, &bb);
    (2.0 * m as f64) / (aa.len() + bb.len()) as f64
}

fn matching_blocks_len(a: &[char], b: &[char]) -> usize {
    if a.is_empty() || b.is_empty() {
        return 0;
    }
    let (i, j, n) = longest_match(a, b);
    if n == 0 {
        return 0;
    }
    matching_blocks_len(&a[..i], &b[..j])
        + n
        + matching_blocks_len(&a[i + n..], &b[j + n..])
}

fn longest_match(a: &[char], b: &[char]) -> (usize, usize, usize) {
    let mut best = (0usize, 0usize, 0usize);
    for i in 0..a.len() {
        for j in 0..b.len() {
            let mut n = 0usize;
            while i + n < a.len() && j + n < b.len() && a[i + n] == b[j + n] {
                n += 1;
            }
            if n > best.2 {
                best = (i, j, n);
            }
        }
    }
    best
}

/// Line-vote: mean gestalt of aligned lines (Hermes multi-line).
pub(crate) fn line_vote_ratio(a: &str, b: &str) -> f64 {
    let al: Vec<&str> = a.lines().collect();
    let bl: Vec<&str> = b.lines().collect();
    if al.is_empty() && bl.is_empty() {
        return 1.0;
    }
    let n = al.len().max(bl.len()).max(1);
    let mut sum = 0.0;
    for i in 0..n {
        let la = al.get(i).copied().unwrap_or("");
        let lb = bl.get(i).copied().unwrap_or("");
        sum += gestalt_ratio(la, lb);
    }
    sum / n as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gestalt_identical() {
        assert!((gestalt_ratio("hello", "hello") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn line_vote_partial() {
        let r = line_vote_ratio("a\nb\nc", "a\nx\nc");
        assert!(r > 0.5 && r < 1.0);
    }
}
