// SPDX-License-Identifier: MIT OR Apache-2.0
// OOXML presentation (pptx) parsing.
// Included from office.inc.rs.

/// Slide part names in presentation order, resolved through the relationships.
fn parse_slide_order(presentation: &str, rels: &str) -> Vec<String> {
    let ids = parse_slide_relation_ids(presentation);
    if ids.is_empty() {
        return Vec::new();
    }
    let targets = parse_relationship_targets(rels);
    ids.iter()
        .filter_map(|id| targets.get(id).cloned())
        .collect()
}

/// `r:id` values of `<p:sldId>` in document order.
fn parse_slide_relation_ids(presentation: &str) -> Vec<String> {
    let mut reader = xml_reader(presentation);
    let mut buffer = Vec::new();
    let mut ids = Vec::new();
    let mut inside_list = false;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(e)) => match local_name(e.name()).as_str() {
                "sldidlst" => inside_list = true,
                "sldid" if inside_list => {
                    if let Some(id) = attr_value(&e, b"r:id") {
                        ids.push(id);
                    }
                }
                _ => {}
            },
            Ok(Event::Empty(e)) if inside_list && local_name(e.name()) == "sldid" => {
                if let Some(id) = attr_value(&e, b"r:id") {
                    ids.push(id);
                }
            }
            Ok(Event::End(e)) => {
                if local_name(e.name()) == "sldidlst" {
                    inside_list = false;
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }
    ids
}

/// Map every `Id` to its `Target` in a `.rels` part.
fn parse_relationship_targets(rels: &str) -> std::collections::HashMap<String, String> {
    let mut reader = xml_reader(rels);
    let mut buffer = Vec::new();
    let mut targets = std::collections::HashMap::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let (Some(id), Some(target)) =
                    (attr_value(&e, b"Id"), attr_value(&e, b"Target"))
                else {
                    buffer.clear();
                    continue;
                };
                // Relationship targets are relative to `ppt/`.
                let normalized = target
                    .trim_start_matches('/')
                    .strip_prefix("ppt/")
                    .map_or_else(|| target.clone(), str::to_string);
                targets.insert(id, format!("ppt/{normalized}"));
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }
    targets
}

/// Read each slide of the presentation, in presentation order.
///
/// # Errors
///
/// Propagates the error of a slide part that cannot be read.
fn read_slides(
    archive: &mut ZipArchive<BufReader<File>>,
    names: &[String],
    path: &std::path::Path,
    max_part: u64,
    order: &[String],
    truncated_parts: &mut u64,
) -> Result<Vec<SlideSignal>> {
    // Fall back to the archive's own ordering when the relationship graph is
    // missing or unreadable; the slide inventory still has to be delivered.
    let mut parts = if order.is_empty() {
        let mut discovered: Vec<String> = names
            .iter()
            .filter(|n| n.starts_with(PART_SLIDE_PREFIX) && n.ends_with(PART_SLIDE_SUFFIX))
            .filter(|n| !n.contains("/_rels/"))
            .cloned()
            .collect();
        discovered.sort();
        discovered
    } else {
        order.to_vec()
    };
    parts.truncate(crate::config::tuning().extract_max_slides);

    let mut slides = Vec::with_capacity(parts.len());
    for (index, part) in parts.iter().enumerate() {
        let (xml, cut) = read_part(archive, part, path, max_part)?;
        *truncated_parts += u64::from(cut);
        let (title, text_chars) = parse_slide(&xml);
        slides.push(SlideSignal {
            index: index as u64 + 1,
            title,
            text_chars,
        });
    }
    Ok(slides)
}

/// First text run and total text length of one `DrawingML` slide.
fn parse_slide(xml: &str) -> (String, u64) {
    let mut reader = xml_reader(xml);
    let mut buffer = Vec::new();
    let mut title = String::new();
    let mut text_chars = 0u64;
    let mut inside_text = false;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(e)) => {
                if local_name(e.name()) == "t" {
                    inside_text = true;
                }
            }
            Ok(Event::Text(t)) if inside_text => {
                if let Some(text) = text_of(&Event::Text(t))
                    && !text.trim().is_empty()
                {
                    text_chars += text.chars().count() as u64;
                    if title.is_empty() {
                        title = truncate_chars(text.trim(), CELL_CHAR_CAP);
                    }
                }
            }
            Ok(Event::End(e)) => {
                if local_name(e.name()) == "t" {
                    inside_text = false;
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }
    (title, text_chars)
}
