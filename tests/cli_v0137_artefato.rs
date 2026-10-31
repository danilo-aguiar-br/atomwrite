// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 E5/E7 — artifact signal extraction and the one-shot default build.
//!
//! Every fixture is generated inside a temporary directory, so the suite has no
//! external file dependency and runs on Linux, macOS and Windows.

use std::io::Write;
use std::path::Path;

use tempfile::TempDir;
use zip::ZipArchive;
use zip::write::SimpleFileOptions;

/// Build an OOXML container from `(part name, part body)` pairs.
fn build_package(parts: &[(&str, &str)]) -> Vec<u8> {
    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut buffer);
        let options = SimpleFileOptions::default();
        for (name, body) in parts {
            writer.start_file(*name, options).expect("start OOXML part");
            writer.write_all(body.as_bytes()).expect("write OOXML part");
        }
        writer.finish().expect("finish OOXML container");
    }
    buffer.into_inner()
}

/// The smallest valid `PDF`: one page with an explicit `MediaBox` and an Info dict.
///
/// The cross-reference table carries the real byte offset of every object, and
/// `startxref` points at the real table offset. A fixture with guessed offsets
/// would still parse far enough to answer a page count, which is exactly the
/// kind of false green this suite must not produce.
fn minimal_pdf() -> Vec<u8> {
    let objects = "\
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Contents 4 0 R>>endobj
4 0 obj<</Length 44>>stream
BT /F1 12 Tf 72 720 Td (Laudo medico) Tj ET
endstream
endobj
5 0 obj<</Title(Laudo 2026)/Author(Clinica Central)/Producer(atomwrite test)>>endobj
";

    let mut pdf = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();
    for object in objects.split_inclusive('\n') {
        offsets.push(pdf.len());
        pdf.push_str(object);
    }

    let xref_offset = pdf.len();
    pdf.push_str(&format!("xref\n0 {}\n", offsets.len() + 1));
    pdf.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        pdf.push_str(&format!("{offset:010} 00000 n \n"));
    }
    pdf.push_str(&format!(
        "trailer<</Root 1 0 R/Info 5 0 R/Size {}>>\nstartxref\n{xref_offset}\n%%EOF\n",
        offsets.len() + 1
    ));
    pdf.into_bytes()
}

/// A 2x3 PNG built from its own CRC32-checked chunks.
fn small_png() -> Vec<u8> {
    // Width 2, height 3, 8-bit truecolour, then one empty IDAT and no IEND data.
    let mut png: Vec<u8> = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&2u32.to_be_bytes());
    ihdr.extend_from_slice(&3u32.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    png.extend_from_slice(&png_chunk(b"IHDR", &ihdr));
    png.extend_from_slice(&png_chunk(
        b"IDAT",
        &[0x78, 0x9C, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01],
    ));
    png.extend_from_slice(&png_chunk(b"IEND", &[]));
    png
}

/// One PNG chunk: length, type, data, CRC32 over type and data.
fn png_chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut chunk = Vec::with_capacity(data.len() + 12);
    chunk.extend_from_slice(&(data.len() as u32).to_be_bytes());
    chunk.extend_from_slice(kind);
    chunk.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    chunk.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    chunk
}

/// CRC32 (IEEE) as the PNG specification defines it.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Write `bytes` into a fresh temporary directory and return the directory.
fn staged(name: &str, bytes: &[u8]) -> (TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join(name);
    std::fs::write(&path, bytes).expect("write fixture");
    (dir, path)
}

/// Run `extract-artifact` and return `(exit code, NDJSON events, stdout size)`.
fn run_extract(path: &Path) -> (i32, Vec<serde_json::Value>, usize) {
    let workspace = path.parent().expect("fixture parent");
    let output = common::atomwrite()
        .arg("--workspace")
        .arg(workspace)
        .arg("extract-artifact")
        .arg(path)
        .output()
        .expect("run extract-artifact");
    let code = output.status.code().expect("process exit code");
    (
        code,
        common::parse_ndjson(&output.stdout),
        output.stdout.len(),
    )
}

mod common;

const DOCX_PARTS: &[(&str, &str)] = &[
    (
        "docProps/core.xml",
        r#"<cp:coreProperties xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Contrato</dc:title><dc:creator>Juridico</dc:creator></cp:coreProperties>"#,
    ),
    (
        "docProps/app.xml",
        r#"<Properties><Application>Microsoft Word</Application></Properties>"#,
    ),
    (
        "word/document.xml",
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Clausula primeira</w:t></w:r></w:p><w:p><w:r><w:t>Clausula segunda</w:t></w:r></w:p></w:body></w:document>"#,
    ),
];

const XLSX_PARTS: &[(&str, &str)] = &[
    (
        "xl/workbook.xml",
        r#"<workbook><sheets><sheet name="Estoque" sheetId="1" r:id="rId1"/><sheet name="Vendas" sheetId="2" r:id="rId2"/></sheets></workbook>"#,
    ),
    (
        "xl/sharedStrings.xml",
        r#"<sst><si><t>Produto</t></si><si><t>Preco</t></si><si><t>Dipirona</t></si><si><t>Paracetamol</t></si></sst>"#,
    ),
    (
        "xl/worksheets/sheet1.xml",
        concat!(
            r#"<worksheet><sheetData>"#,
            r#"<row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c></row>"#,
            r#"<row r="2"><c r="A2" t="s"><v>2</v></c><c r="B2"><v>7.49</v></c></row>"#,
            r#"<row r="3"><c r="A3" t="s"><v>3</v></c><c r="B3"><v>12.9</v></c></row>"#,
            r#"</sheetData></worksheet>"#
        ),
    ),
    (
        "xl/worksheets/sheet2.xml",
        r#"<worksheet><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Mes</t></is></c></row><row r="2"><c r="A2" t="inlineStr"><is><t>Janeiro</t></is></c></row></sheetData></worksheet>"#,
    ),
];

const PPTX_PARTS: &[(&str, &str)] = &[
    (
        "ppt/presentation.xml",
        r#"<p:presentation xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><p:sldIdLst><p:sldId id="256" r:id="rId2"/><p:sldId id="257" r:id="rId1"/></p:sldIdLst></p:presentation>"#,
    ),
    (
        "ppt/_rels/presentation.xml.rels",
        r#"<Relationships><Relationship Id="rId1" Target="slides/slide1.xml"/><Relationship Id="rId2" Target="slides/slide2.xml"/></Relationships>"#,
    ),
    (
        "ppt/slides/slide1.xml",
        r#"<p:sld xmlns:a="x"><p:cSld><a:t>Primeiro assunto</a:t></p:cSld></p:sld>"#,
    ),
    (
        "ppt/slides/slide2.xml",
        r#"<p:sld xmlns:a="x"><p:cSld><a:t>Titulo de abertura</a:t><a:t>corpo do slide</a:t></p:cSld></p:sld>"#,
    ),
];

#[test]
fn docx_returns_bounded_document_signal() {
    let payload = build_package(DOCX_PARTS);
    let (dir, path) = staged("contrato.docx", &payload);
    let (code, events, stdout_bytes) = run_extract(&path);

    assert_eq!(code, 0);
    assert_eq!(events.len(), 1, "exactly one NDJSON record");
    let event = &events[0];
    assert_eq!(event["type"], "artifact");
    assert_eq!(event["kind"], "document");
    assert_eq!(event["format"], "ooxml-docx");
    assert_eq!(event["title"], "Contrato");
    assert_eq!(event["author"], "Juridico");
    assert_eq!(event["producer"], "Microsoft Word");
    assert_eq!(event["paragraphs"], 2);
    assert!(
        event["text_excerpt"]
            .as_str()
            .expect("excerpt")
            .contains("Clausula primeira")
    );
    // The signal never carries the body: the payload is far smaller than input.
    assert!(
        stdout_bytes < payload.len(),
        "signal {stdout_bytes} bytes must be smaller than the {}-byte artifact",
        payload.len()
    );
    drop(dir);
}

#[test]
fn xlsx_returns_header_types_and_sample_not_rows() {
    let payload = build_package(XLSX_PARTS);
    let (dir, path) = staged("estoque.xlsx", &payload);
    let (code, events, stdout_bytes) = run_extract(&path);

    assert_eq!(code, 0);
    let event = &events[0];
    assert_eq!(event["kind"], "spreadsheet");
    assert_eq!(event["format"], "ooxml-xlsx");

    let sheets = event["sheets"].as_array().expect("sheets array");
    assert_eq!(sheets.len(), 2);
    assert_eq!(sheets[0]["name"], "Estoque");
    assert_eq!(sheets[0]["rows"], 3);
    assert_eq!(sheets[0]["header"][0], "Produto");
    assert_eq!(sheets[0]["header"][1], "Preco");
    assert_eq!(sheets[0]["types"][0], "string");
    assert_eq!(sheets[0]["types"][1], "number");
    assert_eq!(sheets[1]["name"], "Vendas");
    assert_eq!(sheets[1]["sample"][0][0], "Janeiro");
    assert!(
        stdout_bytes < payload.len(),
        "signal {stdout_bytes} bytes must be smaller than the {}-byte artifact",
        payload.len()
    );
    drop(dir);
}

#[test]
fn xlsx_sample_rows_are_capped() {
    let mut sheet =
        String::from(r#"<worksheet><sheetData><row r="1"><c r="A1"><v>0</v></c></row>"#);
    for row in 2..500 {
        sheet.push_str(&format!(
            r#"<row r="{row}"><c r="A{row}"><v>{row}</v></c></row>"#
        ));
    }
    sheet.push_str("</sheetData></worksheet>");

    let payload = build_package(&[
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="Longa" sheetId="1"/></sheets></workbook>"#,
        ),
        ("xl/worksheets/sheet1.xml", sheet.as_str()),
    ]);
    let (dir, path) = staged("longa.xlsx", &payload);
    let (code, events, stdout_bytes) = run_extract(&path);

    assert_eq!(code, 0);
    let sheet = &events[0]["sheets"][0];
    assert_eq!(sheet["rows"], 499);
    assert_eq!(
        sheet["sample"].as_array().expect("sample").len(),
        10,
        "default --max-sample-rows caps the echo at 10"
    );
    // 499 rows in the artifact, ten echoed: the reduction is the whole point.
    assert!(stdout_bytes < payload.len());
    drop(dir);
}

#[test]
fn pptx_lists_slide_titles_in_presentation_order() {
    let payload = build_package(PPTX_PARTS);
    let (dir, path) = staged("aula.pptx", &payload);
    let (code, events, stdout_bytes) = run_extract(&path);

    assert_eq!(code, 0);
    let event = &events[0];
    assert_eq!(event["kind"], "presentation");
    assert_eq!(event["format"], "ooxml-pptx");

    let slides = event["slides"].as_array().expect("slides array");
    assert_eq!(slides.len(), 2);
    // `rId2` points at slide2, so that slide is first in presentation order.
    assert_eq!(slides[0]["index"], 1);
    assert_eq!(slides[0]["title"], "Titulo de abertura");
    assert_eq!(slides[1]["title"], "Primeiro assunto");
    assert!(stdout_bytes < payload.len());
    drop(dir);
}

#[test]
fn pdf_returns_page_count_geometry_and_info() {
    let payload = minimal_pdf();
    let (dir, path) = staged("laudo.pdf", &payload);
    let (code, events, stdout_bytes) = run_extract(&path);

    assert_eq!(code, 0);
    let event = &events[0];
    assert_eq!(event["kind"], "pdf");
    assert_eq!(event["format"], "pdf");
    assert_eq!(event["pages"], 1);
    assert_eq!(event["width"], 612);
    assert_eq!(event["height"], 792);
    assert_eq!(event["title"], "Laudo 2026");
    assert_eq!(event["author"], "Clinica Central");
    assert_eq!(event["producer"], "atomwrite test");
    // A MediaBox of 612x792 points is reported, never the content stream bytes.
    let record = serde_json::to_string(event).expect("serialize record");
    assert!(
        !record.contains("Laudo medico"),
        "the page text must not reach the signal channel"
    );
    assert!(stdout_bytes < payload.len());
    drop(dir);
}

#[test]
fn png_returns_dimensions_without_pixels() {
    let payload = small_png();
    let (dir, path) = staged("icone.png", &payload);
    let (code, events, stdout_bytes) = run_extract(&path);

    assert_eq!(code, 0);
    let event = &events[0];
    assert_eq!(event["kind"], "image");
    assert_eq!(event["format"], "png");
    assert_eq!(event["width"], 2);
    assert_eq!(event["height"], 3);
    assert_eq!(event["megapixels"], 0.0);
    // A 2x3 image is tiny, so the signal is compared against the record only:
    // the point is that no pixel payload exists in the schema at all.
    assert!(event.get("pixels").is_none());
    assert!(event.get("data").is_none());
    assert!(stdout_bytes < 4096);
    drop(dir);
}

#[test]
fn pptx_signal_stays_bounded_on_a_many_slide_deck() {
    let mut parts: Vec<(String, String)> = Vec::new();
    let mut ids = String::new();
    let mut rels = String::new();
    for index in 1..=200u32 {
        ids.push_str(&format!(
            r#"<p:sldId id="{}" r:id="rId{index}"/>"#,
            255 + index
        ));
        rels.push_str(&format!(
            r#"<Relationship Id="rId{index}" Target="slides/slide{index}.xml"/>"#
        ));
        parts.push((
            format!("ppt/slides/slide{index}.xml"),
            format!(r#"<p:sld xmlns:a="x"><p:cSld><a:t>Assunto {index}</a:t></p:cSld></p:sld>"#),
        ));
    }
    let presentation = format!(
        r#"<p:presentation xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><p:sldIdLst>{ids}</p:sldIdLst></p:presentation>"#
    );

    let mut borrowed: Vec<(&str, &str)> = vec![
        ("ppt/presentation.xml", presentation.as_str()),
        ("ppt/_rels/presentation.xml.rels", rels.as_str()),
    ];
    for (name, body) in &parts {
        borrowed.push((name.as_str(), body.as_str()));
    }
    let payload = build_package(&borrowed);

    let (dir, path) = staged("longa.pptx", &payload);
    let (code, events, _) = run_extract(&path);

    assert_eq!(code, 0);
    let slides = events[0]["slides"].as_array().expect("slides array");
    assert_eq!(slides.len(), 64, "MAX_SLIDES caps the reported inventory");
    drop(dir);
}

#[test]
fn corrupt_zip_fails_closed_with_a_clear_error() {
    // Local-file header, then garbage: the container looks like OOXML and is not.
    let mut payload = b"PK\x03\x04".to_vec();
    payload.extend_from_slice(&[0u8; 200]);
    let (dir, path) = staged("quebrado.xlsx", &payload);
    let (code, events, _) = run_extract(&path);

    assert_ne!(code, 0, "a corrupt container must not exit 0");
    assert_eq!(code, 65, "INVALID_INPUT owns the corrupt-artifact case");
    let event = &events[0];
    assert_eq!(event["ok"], false);
    assert_eq!(event["error"], true);
    assert_eq!(event["code"], "INVALID_INPUT");
    assert_eq!(event["exit"], 65);
    assert!(
        event["message"]
            .as_str()
            .expect("message")
            .contains("corrupt artifact"),
        "the cause must be stated, got {}",
        event["message"]
    );
    drop(dir);
}

#[test]
fn truncated_pdf_fails_closed_with_a_clear_error() {
    let (dir, path) = staged("cortado.pdf", b"%PDF-1.4\ntrailer<</Root");
    let (code, events, _) = run_extract(&path);

    assert_ne!(code, 0);
    assert_eq!(code, 65);
    assert_eq!(events[0]["code"], "INVALID_INPUT");
    assert!(
        events[0]["message"]
            .as_str()
            .expect("message")
            .contains("corrupt artifact")
    );
    drop(dir);
}

#[test]
fn truncated_png_fails_closed_with_a_clear_error() {
    // Valid PNG signature, then a header too short to carry an extent.
    let (dir, path) = staged(
        "cortado.png",
        b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00",
    );
    let (code, events, _) = run_extract(&path);

    assert_ne!(code, 0);
    assert_eq!(code, 65);
    assert_eq!(events[0]["code"], "INVALID_INPUT");
    assert!(
        events[0]["message"]
            .as_str()
            .expect("message")
            .contains("corrupt artifact")
    );
    drop(dir);
}

#[test]
fn unsupported_bytes_fail_closed_with_the_supported_list() {
    let (dir, path) = staged("notas.txt", b"plain text, no artifact magic here\n");
    let (code, events, _) = run_extract(&path);

    assert_ne!(code, 0);
    assert_eq!(code, 65);
    let message = events[0]["message"].as_str().expect("message");
    assert!(message.contains("unsupported artifact"), "got {message}");
    assert!(message.contains("pdf"), "the supported list must be stated");
    drop(dir);
}

#[test]
fn missing_file_fails_closed_as_not_found() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("ausente.pdf");
    let output = common::atomwrite()
        .arg("--workspace")
        .arg(dir.path())
        .arg("extract-artifact")
        .arg(&path)
        .output()
        .expect("run extract-artifact");

    assert_eq!(output.status.code(), Some(4), "FILE_NOT_FOUND owns exit 4");
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["code"], "FILE_NOT_FOUND");
    drop(dir);
}

#[test]
fn directory_target_fails_closed_with_a_redirect() {
    let dir = tempfile::tempdir().expect("temp dir");
    let nested = dir.path().join("pasta");
    std::fs::create_dir(&nested).expect("create dir");
    let output = common::atomwrite()
        .arg("--workspace")
        .arg(dir.path())
        .arg("extract-artifact")
        .arg(&nested)
        .output()
        .expect("run extract-artifact");

    assert_eq!(output.status.code(), Some(65));
    let events = common::parse_ndjson(&output.stdout);
    let message = events[0]["message"].as_str().expect("message");
    assert!(message.contains("directory"), "got {message}");
    assert!(
        message.contains("list"),
        "the corrective command must be named"
    );
    drop(dir);
}

#[test]
fn max_text_chars_bounds_the_document_excerpt() {
    let long_text: String = "palavra ".repeat(2_000);
    let document = format!(
        r#"<w:document><w:body><w:p><w:r><w:t>{long_text}</w:t></w:r></w:p></w:body></w:document>"#
    );
    let payload = build_package(&[("word/document.xml", document.as_str())]);
    let (dir, path) = staged("longo.docx", &payload);

    let output = common::atomwrite()
        .arg("--workspace")
        .arg(dir.path())
        .arg("extract-artifact")
        .arg(&path)
        .arg("--max-text-chars")
        .arg("50")
        .output()
        .expect("run extract-artifact");

    assert_eq!(output.status.code(), Some(0));
    let events = common::parse_ndjson(&output.stdout);
    let excerpt = events[0]["text_excerpt"].as_str().expect("excerpt");
    assert_eq!(excerpt.chars().count(), 50);
    let total = events[0]["text_chars"].as_u64().expect("text_chars");
    assert!(
        total > 50,
        "the true length is reported, not the excerpt one"
    );
    drop(dir);
}

// E7, the other side: this asserts the DEFAULT binary, so it runs only when the
// continuous mode is off. With `--features watch` present the subcommand is
// live and the closed-feature answer is not the contract.
#[cfg(not(feature = "watch"))]
#[test]
fn default_binary_does_not_carry_the_continuous_mode() {
    // E7: `watch` left the default feature set, so the default binary answers
    // with the closed-feature error instead of a live filesystem watcher.
    let dir = tempfile::tempdir().expect("temp dir");
    let output = common::atomwrite()
        .arg("--workspace")
        .arg(dir.path())
        .arg("watch")
        .arg(dir.path())
        .arg("--max-events")
        .arg("1")
        .output()
        .expect("run watch");

    assert_eq!(
        output.status.code(),
        Some(78),
        "CONFIG_INVALID owns the disabled-feature case"
    );
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["code"], "CONFIG_INVALID");
    let message = events[0]["message"].as_str().expect("message");
    assert!(
        message.contains("--features watch"),
        "the rebuild path must be stated, got {message}"
    );
    // The subcommand stays advertised on the surface: only its capability is
    // gone. `agent-surface` emits one record whose `tools` array is the index.
    let surface = common::atomwrite()
        .arg("agent-surface")
        .output()
        .expect("run agent-surface");
    let events = common::parse_ndjson(&surface.stdout);
    assert_eq!(events.len(), 1, "agent-surface emits a single manifest");
    let tools = events[0]["tools"].as_array().expect("tools array");
    let watch = tools
        .iter()
        .find(|tool| tool["name"] == "watch")
        .expect("watch stays on the surface");
    assert_eq!(
        watch["available"], false,
        "the default binary must not report the continuous mode as available"
    );
    assert!(
        watch["install_hint"]
            .as_str()
            .expect("install hint")
            .contains("--features watch"),
        "the surface must name the opt-in path"
    );
    drop(dir);
}

#[test]
fn artifact_record_carries_no_raw_artifact_bytes() {
    let payload = build_package(DOCX_PARTS);
    let (dir, path) = staged("contrato.docx", &payload);
    let (code, events, _) = run_extract(&path);

    assert_eq!(code, 0);
    let record = serde_json::to_string(&events[0]).expect("serialize record");
    // The channel carries the signal only: no base64 blob, no part body.
    assert!(!record.contains("PK"), "no container bytes in the record");
    assert!(
        !record.contains("w:document"),
        "no raw XML markup in the record"
    );
    assert!(!record.contains("base64"));
    drop(dir);
}

#[test]
fn artifact_signal_is_far_smaller_than_the_artifact() {
    // A workbook whose data dwarfs its signal: 3000 rows of padded labels.
    let mut sheet =
        String::from(r#"<worksheet><sheetData><row r="1"><c r="A1"><v>0</v></c></row>"#);
    for row in 2..3_000 {
        sheet.push_str(&format!(
            r#"<row r="{row}"><c r="A{row}" t="inlineStr"><is><t>Descricao longa do item {row} com texto suficiente para pesar no arquivo final</t></is></c></row>"#
        ));
    }
    sheet.push_str("</sheetData></worksheet>");

    let payload = build_package(&[
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="Itens" sheetId="1"/></sheets></workbook>"#,
        ),
        ("xl/worksheets/sheet1.xml", sheet.as_str()),
    ]);
    let (dir, path) = staged("grande.xlsx", &payload);
    let (code, events, stdout_bytes) = run_extract(&path);

    assert_eq!(code, 0);
    assert_eq!(events[0]["sheets"][0]["rows"], 2_999);
    // The archive itself must be verified readable, so the 3000-row fixture is
    // real and the reduction below is not an artefact of a broken container.
    let archive = ZipArchive::new(std::io::Cursor::new(payload.clone())).expect("read back");
    assert_eq!(archive.len(), 2, "workbook plus one worksheet");
    drop(archive);

    // DERIVED: reduction factor = artifact bytes divided by signal bytes.
    let reduction = payload.len() as f64 / stdout_bytes as f64;
    assert!(
        reduction > 20.0,
        "expected a >20x reduction, measured {reduction:.1}x \
         ({stdout_bytes} signal bytes for {} artifact bytes)",
        payload.len()
    );
    drop(dir);
}
