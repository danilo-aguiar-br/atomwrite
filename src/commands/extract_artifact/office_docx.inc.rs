// SPDX-License-Identifier: MIT OR Apache-2.0
// OOXML word-processing (docx) body parsing.
// Included from office.inc.rs.

/// Bounded word-processing signal: paragraph count, text length, text head.
struct DocumentSignal {
    /// Number of `<w:p>` paragraph elements.
    paragraphs: u64,
    /// Total characters across all `<w:t>` runs.
    text_chars: u64,
    /// Head of the concatenated text, capped by `--max-text-chars`.
    excerpt: String,
}

/// Walk the `WordprocessingML` body, counting paragraphs and collecting text.
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` when the part is not well-formed XML.
fn parse_document(xml: &str, max_text_chars: usize) -> Result<DocumentSignal> {
    let mut reader = xml_reader(xml);
    let mut buffer = Vec::new();
    let mut paragraphs = 0u64;
    let mut text_chars = 0u64;
    let mut excerpt = String::new();
    let mut inside_text = false;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(e)) => match local_name(e.name()).as_str() {
                "p" => paragraphs += 1,
                "t" => inside_text = true,
                _ => {}
            },
            Ok(Event::Text(t)) if inside_text => {
                if let Some(text) = text_of(&Event::Text(t)) {
                    text_chars += text.chars().count() as u64;
                    if excerpt.chars().count() < max_text_chars {
                        if !excerpt.is_empty() {
                            excerpt.push(' ');
                        }
                        excerpt.push_str(&truncate_chars(&text, max_text_chars));
                    }
                }
            }
            Ok(Event::End(e)) => {
                if local_name(e.name()) == "t" {
                    inside_text = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(corrupt_local("word/document.xml", e)),
            _ => {}
        }
        buffer.clear();
    }

    Ok(DocumentSignal {
        paragraphs,
        text_chars,
        excerpt: truncate_chars(&excerpt, max_text_chars),
    })
}

/// Parse failure inside a named OOXML part.
fn corrupt_local(part: &str, err: impl std::fmt::Display) -> anyhow::Error {
    AtomwriteError::InvalidInput {
        reason: format!("corrupt artifact part {part}: {err}"),
    }
    .into()
}
