// SPDX-License-Identifier: MIT OR Apache-2.0
// OOXML spreadsheet (xlsx) parsing.
// Included from office.inc.rs.

/// Sheet names in workbook order, from `<sheet name="…">` declarations.
fn parse_workbook(xml: &str) -> Vec<String> {
    let mut reader = xml_reader(xml);
    let mut buffer = Vec::new();
    let mut names = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) if local_name(e.name()) == "sheet" => {
                if let Some(name) = attr_value(&e, b"name") {
                    names.push(name);
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }
    names
}

/// One entry per `<si>` in the shared string table, `<t>` runs concatenated.
fn parse_shared_strings(xml: &str) -> Vec<String> {
    let mut reader = xml_reader(xml);
    let mut buffer = Vec::new();
    let mut strings = Vec::new();
    let mut inside_si = false;
    let mut inside_t = false;
    let mut current = String::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(e)) => match local_name(e.name()).as_str() {
                "si" => {
                    inside_si = true;
                    current.clear();
                }
                "t" if inside_si => inside_t = true,
                _ => {}
            },
            Ok(Event::Text(t)) if inside_t => {
                if let Some(text) = text_of(&Event::Text(t)) {
                    current.push_str(&text);
                }
            }
            Ok(Event::End(e)) => match local_name(e.name()).as_str() {
                "t" => inside_t = false,
                "si" => {
                    inside_si = false;
                    strings.push(std::mem::take(&mut current));
                }
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }
    strings
}

/// Bounds of one worksheet read, plus the part-cut counter it feeds.
///
/// Grouped in a struct so `read_sheets` stays under the argument limit the
/// lint enforces, without changing what the caller has to know.
struct SheetScan<'a> {
    max_part: u64,
    max_sample_rows: usize,
    truncated_parts: &'a mut u64,
}

/// Read every worksheet referenced by the workbook, bounded per sheet.
fn read_sheets(
    archive: &mut ZipArchive<BufReader<File>>,
    names: &[String],
    path: &std::path::Path,
    shared: &[String],
    sheet_names: &[String],
    scan: &mut SheetScan<'_>,
) -> Vec<SheetSignal> {
    let mut parts: Vec<String> = names
        .iter()
        .filter(|n| is_worksheet_part(n))
        .cloned()
        .collect();
    parts.sort();
    parts.truncate(crate::config::tuning().extract_max_sheets);

    let mut sheets = Vec::with_capacity(parts.len());
    for (index, part) in parts.iter().enumerate() {
        // A part that fails to inflate or parse is reported as a zero-row
        // sheet instead of sinking the whole workbook: the agent still gets
        // the sheet inventory, which is the point of the command.
        let Ok((xml, cut)) = read_part(archive, part, path, scan.max_part) else {
            sheets.push(SheetSignal {
                name: sheet_label(sheet_names, index),
                rows: 0,
                columns: 0,
                header: Vec::new(),
                types: Vec::new(),
                sample: Vec::new(),
            });
            continue;
        };
        *scan.truncated_parts += u64::from(cut);
        let parsed = parse_sheet(&xml, shared, scan.max_sample_rows);
        sheets.push(SheetSignal {
            name: sheet_label(sheet_names, index),
            rows: parsed.rows,
            columns: parsed.header.len() as u64,
            header: parsed.header,
            types: parsed.types,
            sample: parsed.sample,
        });
    }
    sheets
}

/// True for `xl/worksheets/sheetN.xml`, excluding chartsheets and temp parts.
fn is_worksheet_part(name: &str) -> bool {
    name.starts_with("xl/worksheets/sheet") && name.ends_with(".xml") && !name.contains("/_rels/")
}

/// Declared sheet name when available, otherwise the one-based part index.
fn sheet_label(names: &[String], index: usize) -> String {
    names
        .get(index)
        .cloned()
        .unwrap_or_else(|| format!("sheet{}", index + 1))
}

/// Bounded spreadsheet signal for one worksheet.
struct SheetParse {
    /// Number of `<row>` elements carrying at least one cell.
    rows: u64,
    /// Labels from the first row that carries values.
    header: Vec<String>,
    /// Scalar type inferred per column over the sampled rows.
    types: Vec<String>,
    /// Bounded sample of data rows.
    sample: Vec<Vec<String>>,
}

/// Walk one worksheet, keeping the header, the inferred types and a row sample.
fn parse_sheet(xml: &str, shared: &[String], max_sample_rows: usize) -> SheetParse {
    let mut reader = xml_reader(xml);
    let mut buffer = Vec::new();

    let mut rows = 0u64;
    let mut header: Vec<String> = Vec::new();
    let mut sample: Vec<Vec<String>> = Vec::new();
    let mut types: Vec<String> = Vec::new();

    let mut row_cells = 0u64;
    let mut row_values: Vec<(usize, String)> = Vec::new();
    let mut cell_column = 0usize;
    let mut cell_kind = CellKind::Number;
    let mut inside_value = false;
    let mut inside_inline = false;
    let mut cell_text = String::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(e)) => match local_name(e.name()).as_str() {
                "row" => {
                    row_cells = 0;
                    row_values.clear();
                }
                "c" => {
                    row_cells += 1;
                    cell_column = attr_value(&e, b"r")
                        .as_deref()
                        .and_then(column_index)
                        .unwrap_or(row_cells as usize - 1);
                    cell_kind = match attr_value(&e, b"t").as_deref() {
                        Some("s") => CellKind::SharedString,
                        Some("inlineStr") => CellKind::InlineString,
                        Some("b") => CellKind::Boolean,
                        _ => CellKind::Number,
                    };
                    cell_text.clear();
                }
                "v" => inside_value = true,
                "t" if cell_kind == CellKind::InlineString => inside_inline = true,
                _ => {}
            },
            Ok(Event::Text(t)) => {
                if inside_value || inside_inline {
                    if let Some(text) = text_of(&Event::Text(t)) {
                        cell_text.push_str(&text);
                    }
                }
            }
            Ok(Event::End(e)) => match local_name(e.name()).as_str() {
                "v" => {
                    inside_value = false;
                    row_values.push((cell_column, resolve_cell(cell_kind, &cell_text, shared)));
                }
                "t" => inside_inline = false,
                "c" => {
                    if cell_kind == CellKind::InlineString && !inside_value {
                        row_values.push((cell_column, resolve_cell(cell_kind, &cell_text, shared)));
                    }
                }
                "row" => {
                    rows += 1;
                    finish_row(
                        &mut row_values,
                        &mut header,
                        &mut sample,
                        &mut types,
                        max_sample_rows,
                    );
                }
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }

    SheetParse {
        rows,
        header,
        types,
        sample,
    }
}

/// Scalar kind declared by a cell's `t` attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CellKind {
    /// Numeric cell (also the default when `t` is absent).
    Number,
    /// Index into the shared string table.
    SharedString,
    /// Inline string carried by the cell itself.
    InlineString,
    /// Boolean cell, `1` or `0`.
    Boolean,
}

/// Turn a raw `<v>` payload into the value the agent should see.
fn resolve_cell(kind: CellKind, raw: &str, shared: &[String]) -> String {
    let value = match kind {
        CellKind::SharedString => raw
            .trim()
            .parse::<usize>()
            .ok()
            .and_then(|i| shared.get(i).cloned())
            .unwrap_or_default(),
        CellKind::Boolean => match raw.trim() {
            "1" => "true".to_string(),
            "0" => "false".to_string(),
            other => other.to_string(),
        },
        CellKind::Number | CellKind::InlineString => raw.trim().to_string(),
    };
    truncate_chars(&value, CELL_CHAR_CAP)
}

/// Zero-based column index of an A1-style reference (`B7` → `1`).
fn column_index(reference: &str) -> Option<usize> {
    let mut index: usize = 0;
    let mut seen = false;
    for ch in reference.chars() {
        match ch {
            '$' if !seen => continue,
            'A'..='Z' | 'a'..='z' => {
                seen = true;
                index = index * crate::constants::LATIN_ALPHABET_LEN
                    + (ch.to_ascii_uppercase() as usize - 'A' as usize + 1);
            }
            _ => break,
        }
    }
    seen.then(|| index - 1)
}

/// Fold a completed row into the header, the sample, and the type tally.
fn finish_row(
    values: &mut Vec<(usize, String)>,
    header: &mut Vec<String>,
    sample: &mut Vec<Vec<String>>,
    types: &mut Vec<String>,
    max_sample_rows: usize,
) {
    if values.is_empty() {
        return;
    }
    values.sort_by_key(|(column, _)| *column);
    let width = values.last().map_or(0, |(column, _)| column + 1);

    let mut row = vec![String::new(); width];
    for (column, value) in values.drain(..) {
        if let Some(slot) = row.get_mut(column) {
            *slot = value;
        }
    }

    if header.is_empty() && row.iter().any(|v| !v.is_empty()) {
        header.clone_from(&row);
        types.clone_from(&vec![String::new(); row.len()]);
        return;
    }
    if sample.len() >= max_sample_rows {
        return;
    }
    for (index, value) in row.iter().enumerate() {
        let Some(slot) = types.get_mut(index) else {
            continue;
        };
        let observed = scalar_type(value);
        if slot.is_empty() || (*slot != "string" && *slot != observed) {
            *slot = if slot.is_empty() {
                observed.to_string()
            } else {
                "string".to_string()
            };
        }
    }
    sample.push(row);
}

/// Best-effort scalar type of a single cell value.
fn scalar_type(value: &str) -> &'static str {
    if value.is_empty() {
        return "empty";
    }
    if value == "true" || value == "false" {
        return "boolean";
    }
    if value.parse::<f64>().is_ok() {
        return "number";
    }
    "string"
}
