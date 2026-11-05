# ADR-0095 — search submatch offsets are bytes, and the text is rendered lossily


## Status
- Accepted (2026-11-03)
- Records the `Submatch` contract of `search` and the local fix of the multibyte panic


## Context
- `search` publishes the submatch offsets that `grep_regex::RegexMatcher::find_iter` produced, and the matcher runs over bytes
- `grep-regex 0.1.14` compiles the pattern with `utf8(false)` and configures the meta regex with `utf8_empty(false)`, because the searcher reads raw file bytes
- `grep-matcher 0.1.9` advances one byte after every empty match, so a pattern that can match empty is searched at every byte offset
- A zero-width match can land between the code units of a multi-byte character, and a byte-oriented pattern such as `(?-u)\xA7` can match a single byte of one
- `extract_submatches` sliced `&line[start..end]`, which requires character boundaries, so the panic aborted the process with exit 134 under `panic = "abort"`
- `--format text` and NDJSON both reached the same function, so both could abort


## Decision
- `Submatch.start` and `Submatch.end` stay byte offsets into the reported `lines` string, because that is what the matcher returns
- The `match` text is rendered with `String::from_utf8_lossy` over the byte range, never by indexing a `&str`
- A zero-width match publishes `match: ""` with `start == end`, even inside a multi-byte character
- A match that is not valid UTF-8 publishes the replacement character for that fragment instead of taking the process down
- The helper is total, so a range outside the line yields an empty fragment instead of a second panic
- No second matcher is compiled, because the public `RegexMatcherBuilder` exposes no `utf8` option and a duplicate would fork the semantics
- The offsets, the event shapes and the exit codes do not change


## Consequences
- A consumer that reads `match` gets a lossy rendering for byte-oriented patterns, and exact text for every match on character boundaries
- A consumer that reads `start` and `end` keeps the same byte offsets it received before
- The panic disappears from both output formats, including `recipe search-replace-verify`, which calls the same search
- A pattern that can match empty now answers instead of aborting, so the answer set is larger than the crash was
- The `match` field can contain U+FFFD for a fragment inside a multi-byte character, which is the honest rendering of bytes that are not a character


## Alternatives
- Compile a second matcher with `utf8_empty(true)` just for submatch extraction, rejected because `RegexMatcherBuilder` has no `utf8` method and two matchers would fork the semantics
- Skip matches whose offsets are not character boundaries, rejected because it drops a match the matcher reported and loses the `--column` value of a byte-oriented pattern
- Strip byte-oriented flags from the pattern, rejected because it silently changes what the caller asked to match
- Convert every production panic to a typed error, rejected as out of scope because this defect is local and the global hook is a different contract


## Trigger to revisit
- Revisit if `grep-regex` exposes a UTF-8-safe matcher configuration
- Revisit if `Submatch` starts publishing offsets relative to the raw file bytes instead of the lossy `lines` string
