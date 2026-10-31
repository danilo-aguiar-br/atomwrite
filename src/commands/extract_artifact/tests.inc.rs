// SPDX-License-Identifier: MIT OR Apache-2.0
// Included by `extract_artifact/mod.rs` (E5 — unit tests for the parsers and
// for the whole extraction pipeline over real fixtures).
//
// `include!` places these items inside the module, so the parsers under test
// are already in scope and no `use super::*` is needed. The `cfg(test)` gate is
// required: without it the fixture builders and constants would be compiled
// into every build and raise `dead_code`.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_container_reads_pdf_magic() {
        assert_eq!(detect_container(b"%PDF-1.7\n%..."), Some(Container::Pdf));
    }

    #[test]
    fn detect_container_reads_ooxml_magic() {
        assert_eq!(
            detect_container(b"PK\x03\x04\x14\x00\x00\x00"),
            Some(Container::Zip)
        );
    }

    #[test]
    fn detect_container_reads_png_magic() {
        assert_eq!(
            detect_container(b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR"),
            Some(Container::Image)
        );
    }

    #[test]
    fn detect_container_rejects_plain_text() {
        assert_eq!(detect_container(b"hello world, this is text\n"), None);
    }

    #[test]
    fn truncate_chars_counts_characters_not_bytes() {
        assert_eq!(truncate_chars("áéíóú", 3), "áéí");
        assert_eq!(truncate_chars("short", 10), "short");
    }

    #[test]
    fn column_index_parses_a1_references() {
        assert_eq!(column_index("A1"), Some(0));
        assert_eq!(column_index("B7"), Some(1));
        assert_eq!(column_index("AA3"), Some(26));
        assert_eq!(column_index("$C$9"), Some(2));
        assert_eq!(column_index("12"), None);
    }

    #[test]
    fn scalar_type_classifies_cell_values() {
        assert_eq!(scalar_type("42.5"), "number");
        assert_eq!(scalar_type("true"), "boolean");
        assert_eq!(scalar_type(""), "empty");
        assert_eq!(scalar_type("São Paulo"), "string");
    }

    #[test]
    fn resolve_cell_follows_shared_string_index() {
        let shared = vec!["alpha".to_string(), "beta".to_string()];
        assert_eq!(resolve_cell(CellKind::SharedString, "1", &shared), "beta");
        assert_eq!(resolve_cell(CellKind::Boolean, "1", &shared), "true");
        assert_eq!(resolve_cell(CellKind::Number, " 12 ", &shared), "12");
    }

    #[test]
    fn megapixels_is_derived_from_extent() {
        assert!((megapixels(1920, 1080) - 2.074).abs() < 0.001);
        assert!((megapixels(1000, 1000) - 1.0).abs() < 0.001);
    }

    #[test]
    fn parse_workbook_reads_sheet_names_in_order() {
        let xml = concat!(
            r#"<?xml version="1.0"?>"#,
            r#"<workbook xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
            r#"<sheets><sheet name="Resumo" sheetId="1" r:id="rId1"/>"#,
            r#"<sheet name="Detalhe &amp; Extras" sheetId="2" r:id="rId2"/></sheets></workbook>"#
        );
        assert_eq!(
            parse_workbook(xml),
            vec!["Resumo".to_string(), "Detalhe & Extras".to_string()]
        );
    }

    #[test]
    fn parse_shared_strings_concatenates_runs() {
        let xml = concat!(
            r#"<sst><si><t>Primeiro</t></si>"#,
            r#"<si><r><t>Seg</t></r><r><t>undo</t></r></si></sst>"#
        );
        assert_eq!(
            parse_shared_strings(xml),
            vec!["Primeiro".to_string(), "Segundo".to_string()]
        );
    }

    #[test]
    fn parse_sheet_takes_header_types_and_sample() {
        let xml = concat!(
            r#"<worksheet><sheetData>"#,
            r#"<row r="1"><c r="A1" t="inlineStr"><is><t>Produto</t></is></c>"#,
            r#"<c r="B1" t="inlineStr"><is><t>Preco</t></is></c></row>"#,
            r#"<row r="2"><c r="A2" t="inlineStr"><is><t>Dipirona</t></is></c><c r="B2"><v>7.49</v></c></row>"#,
            r#"<row r="3"><c r="A3" t="inlineStr"><is><t>Paracetamol</t></is></c><c r="B3"><v>12.9</v></c></row>"#,
            r#"</sheetData></worksheet>"#
        );
        let parsed = parse_sheet(xml, &[], 10);
        assert_eq!(parsed.rows, 3);
        assert_eq!(parsed.header, vec!["Produto", "Preco"]);
        assert_eq!(parsed.types, vec!["string", "number"]);
        assert_eq!(parsed.sample.len(), 2);
        assert_eq!(parsed.sample[0], vec!["Dipirona", "7.49"]);
    }

    #[test]
    fn parse_sheet_caps_the_sample_rows() {
        let mut xml =
            String::from(r#"<worksheet><sheetData><row r="1"><c r="A1"><v>0</v></c></row>"#);
        for row in 2..40 {
            xml.push_str(&format!(
                r#"<row r="{row}"><c r="A{row}"><v>{row}</v></c></row>"#
            ));
        }
        xml.push_str("</sheetData></worksheet>");
        let parsed = parse_sheet(&xml, &[], 3);
        assert_eq!(parsed.rows, 39);
        assert_eq!(parsed.sample.len(), 3);
    }

    #[test]
    fn parse_document_counts_paragraphs_and_text() {
        let xml = concat!(
            r#"<w:document xmlns:w="x"><w:body>"#,
            r#"<w:p><w:r><w:t>Laudo</w:t></w:r></w:p>"#,
            r#"<w:p><w:r><w:t>Paciente</w:t></w:r><w:r><w:t>Joao</w:t></w:r></w:p>"#,
            r#"</w:body></w:document>"#
        );
        let signal = parse_document(xml, 400).expect("well-formed document part");
        assert_eq!(signal.paragraphs, 2);
        assert_eq!(
            signal.text_chars,
            "LaudoPacienteJoao".chars().count() as u64
        );
        assert!(signal.excerpt.contains("Laudo"));
    }

    #[test]
    fn parse_document_rejects_malformed_xml() {
        let xml = "<w:document><w:body><w:p></w:document>";
        assert!(parse_document(xml, 400).is_err());
    }

    #[test]
    fn parse_document_truncates_the_excerpt() {
        let xml = concat!(
            r#"<w:document><w:body><w:p><w:r><w:t>"#,
            "abcdefghij",
            r#"</w:t></w:r></w:p></w:body></w:document>"#
        );
        let signal = parse_document(xml, 4).expect("well-formed document part");
        assert_eq!(signal.text_chars, 10);
        assert_eq!(signal.excerpt, "abcd");
    }

    #[test]
    fn parse_core_props_reads_title_and_creator() {
        let xml = concat!(
            r#"<cp:coreProperties xmlns:dc="x"><dc:title>Receita Federal</dc:title>"#,
            r#"<dc:creator>Danilo</dc:creator></cp:coreProperties>"#
        );
        assert_eq!(
            parse_core_props(xml),
            (
                Some("Receita Federal".to_string()),
                Some("Danilo".to_string())
            )
        );
    }

    #[test]
    fn parse_app_props_reads_the_producing_application() {
        let xml = r#"<Properties><Application>Microsoft Excel</Application></Properties>"#;
        assert_eq!(parse_app_props(xml), Some("Microsoft Excel".to_string()));
    }

    #[test]
    fn parse_slide_order_keeps_presentation_order() {
        let presentation = concat!(
            r#"<p:presentation xmlns:r="x"><p:sldIdLst>"#,
            r#"<p:sldId id="256" r:id="rId2"/><p:sldId id="257" r:id="rId1"/>"#,
            r#"</p:sldIdLst></p:presentation>"#
        );
        let rels = concat!(
            r#"<Relationships><Relationship Id="rId1" Target="slides/slide1.xml"/>"#,
            r#"<Relationship Id="rId2" Target="slides/slide2.xml"/></Relationships>"#
        );
        assert_eq!(
            parse_slide_order(presentation, rels),
            vec![
                "ppt/slides/slide2.xml".to_string(),
                "ppt/slides/slide1.xml".to_string()
            ]
        );
    }

    #[test]
    fn parse_slide_reads_title_and_text_length() {
        let xml = concat!(
            r#"<p:sld xmlns:a="x"><p:cSld><a:t>Apresentacao</a:t>"#,
            r#"<a:t>em duas linhas</a:t></p:cSld></p:sld>"#
        );
        let (title, text_chars) = parse_slide(xml);
        assert_eq!(title, "Apresentacao");
        assert_eq!(
            text_chars,
            "Apresentacaoem duas linhas".chars().count() as u64
        );
    }

    #[test]
    fn is_worksheet_part_skips_relationship_parts() {
        assert!(is_worksheet_part("xl/worksheets/sheet1.xml"));
        assert!(!is_worksheet_part("xl/worksheets/_rels/sheet1.xml.rels"));
        assert!(!is_worksheet_part("xl/chartsheets/sheet1.xml"));
    }

    #[test]
    fn sheet_label_falls_back_to_the_part_index() {
        assert_eq!(sheet_label(&["Resumo".to_string()], 0), "Resumo");
        assert_eq!(sheet_label(&[], 2), "sheet3");
    }

    // End-to-end coverage over real fixtures written into a temporary directory.
    // These drive [`extract_signal`] directly, so they prove the whole pipeline
    // (magic-byte detection, container decode, signal shaping) without depending
    // on the clap surface.

    /// Build a ZIP container from `(part name, part body)` pairs.
    fn build_package(parts: &[(&str, &str)]) -> Vec<u8> {
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let options = zip::write::SimpleFileOptions::default();
            for (name, body) in parts {
                writer.start_file(*name, options).expect("start part");
                writer.write_all(body.as_bytes()).expect("write part");
            }
            writer.finish().expect("finish container");
        }
        buffer.into_inner()
    }

    /// Write `bytes` into a fresh temporary directory and return the handle and path.
    fn staged(name: &str, bytes: &[u8]) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(name);
        std::fs::write(&path, bytes).expect("write fixture");
        (dir, path)
    }

    /// A one-page `PDF` with an explicit `MediaBox` and a document information dict.
    fn minimal_pdf() -> Vec<u8> {
        let objects = "\
    1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
    2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
    3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]>>endobj
    4 0 obj<</Title(Memoria de calculo)/Author(Engenharia)/Producer(atomwrite)>>endobj
    ";
        let mut pdf = String::from("%PDF-1.4\n");
        let mut offsets = Vec::new();
        for object in objects.split_inclusive('\n') {
            offsets.push(pdf.len());
            pdf.push_str(object);
        }
        let trailer_offset = pdf.len();
        pdf.push_str(&format!("xref\n0 {}\n", offsets.len() + 1));
        pdf.push_str("0000000000 65535 f \n");
        for offset in &offsets {
            pdf.push_str(&format!("{offset:010} 00000 n \n"));
        }
        pdf.push_str(&format!(
            "trailer<</Root 1 0 R/Info 4 0 R/Size {}>>\nstartxref\n{trailer_offset}\n%%EOF\n",
            offsets.len() + 1
        ));
        pdf.into_bytes()
    }

    /// A valid PNG signature followed by an IHDR chunk carrying 2x3 pixels.
    fn small_png() -> Vec<u8> {
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
        fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
            let mut out = Vec::new();
            out.extend_from_slice(&(data.len() as u32).to_be_bytes());
            out.extend_from_slice(kind);
            out.extend_from_slice(data);
            let mut crc_input = kind.to_vec();
            crc_input.extend_from_slice(data);
            out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
            out
        }

        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&2u32.to_be_bytes());
        ihdr.extend_from_slice(&3u32.to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
        png.extend_from_slice(&chunk(b"IHDR", &ihdr));
        png.extend_from_slice(&chunk(
            b"IDAT",
            &[0x78, 0x9C, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01],
        ));
        png.extend_from_slice(&chunk(b"IEND", &[]));
        png
    }

    const DOCX_PARTS: &[(&str, &str)] = &[
        (
            "docProps/core.xml",
            r#"<cp:coreProperties xmlns:dc="x"><dc:title>Contrato</dc:title><dc:creator>Juridico</dc:creator></cp:coreProperties>"#,
        ),
        (
            "docProps/app.xml",
            r#"<Properties><Application>Microsoft Word</Application></Properties>"#,
        ),
        (
            "word/document.xml",
            r#"<w:document xmlns:w="x"><w:body><w:p><w:r><w:t>Clausula primeira</w:t></w:r></w:p><w:p><w:r><w:t>Clausula segunda</w:t></w:r></w:p></w:body></w:document>"#,
        ),
    ];

    const XLSX_PARTS: &[(&str, &str)] = &[
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="Estoque" sheetId="1"/><sheet name="Vendas" sheetId="2"/></sheets></workbook>"#,
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
            concat!(
                r#"<worksheet><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Mes</t></is></c></row>"#,
                r#"<row r="2"><c r="A2" t="inlineStr"><is><t>Janeiro</t></is></c></row></sheetData></worksheet>"#
            ),
        ),
    ];

    const PPTX_PARTS: &[(&str, &str)] = &[
        (
            "ppt/presentation.xml",
            r#"<p:presentation xmlns:r="x"><p:sldIdLst><p:sldId id="256" r:id="rId2"/><p:sldId id="257" r:id="rId1"/></p:sldIdLst></p:presentation>"#,
        ),
        (
            "ppt/_rels/presentation.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Target="slides/slide1.xml"/><Relationship Id="rId2" Target="slides/slide2.xml"/></Relationships>"#,
        ),
        (
            "ppt/slides/slide1.xml",
            r#"<p:sld xmlns:a="x"><a:t>Primeiro assunto</a:t></p:sld>"#,
        ),
        (
            "ppt/slides/slide2.xml",
            r#"<p:sld xmlns:a="x"><a:t>Titulo de abertura</a:t><a:t>corpo</a:t></p:sld>"#,
        ),
    ];

    /// Signal with the command defaults applied.
    fn signal_of(path: &std::path::Path) -> Result<ArtifactOutput> {
        extract_signal(
            path,
            DEFAULT_MAX_SAMPLE_ROWS,
            DEFAULT_MAX_TEXT_CHARS,
            u64::MAX,
        )
    }

    #[test]
    fn docx_signal_is_bounded_and_never_carries_the_body() {
        let payload = build_package(DOCX_PARTS);
        let (dir, path) = staged("contrato.docx", &payload);
        let signal = signal_of(&path).expect("docx signal");

        assert_eq!(signal.kind, "document");
        assert_eq!(signal.format, "ooxml-docx");
        assert_eq!(signal.title.as_deref(), Some("Contrato"));
        assert_eq!(signal.author.as_deref(), Some("Juridico"));
        assert_eq!(signal.producer.as_deref(), Some("Microsoft Word"));
        assert_eq!(signal.paragraphs, Some(2));

        let record = serde_json::to_string(&signal).expect("serialize");
        let reduction = payload.len() as f64 / record.len() as f64;
        assert!(
            reduction > 1.0,
            "signal {} bytes must beat the {}-byte artifact",
            record.len(),
            payload.len()
        );
        drop(dir);
    }

    #[test]
    fn xlsx_signal_carries_header_types_and_sample_not_rows() {
        let payload = build_package(XLSX_PARTS);
        let (dir, path) = staged("estoque.xlsx", &payload);
        let signal = signal_of(&path).expect("xlsx signal");

        assert_eq!(signal.kind, "spreadsheet");
        assert_eq!(signal.format, "ooxml-xlsx");
        let sheets = signal.sheets.as_ref().expect("sheets");
        assert_eq!(sheets.len(), 2);
        assert_eq!(sheets[0].name, "Estoque");
        assert_eq!(sheets[0].rows, 3);
        assert_eq!(sheets[0].header, vec!["Produto", "Preco"]);
        assert_eq!(sheets[0].types, vec!["string", "number"]);
        assert_eq!(sheets[0].sample[0], vec!["Dipirona", "7.49"]);
        assert_eq!(sheets[1].name, "Vendas");
        assert_eq!(sheets[1].sample[0][0], "Janeiro");
        drop(dir);
    }

    #[test]
    fn xlsx_signal_stays_bounded_on_a_large_sheet() {
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
        let signal = signal_of(&path).expect("large xlsx signal");

        let sheets = signal.sheets.as_ref().expect("sheets");
        assert_eq!(sheets[0].rows, 2_999);
        assert_eq!(sheets[0].sample.len(), DEFAULT_MAX_SAMPLE_ROWS);

        // The archive read back proves the 3000-row fixture is a real container,
        // so the reduction measured next is not an artefact of a broken input.
        let archive =
            zip::ZipArchive::new(std::io::Cursor::new(payload.clone())).expect("read back");
        assert_eq!(archive.len(), 2, "workbook plus one worksheet");
        drop(archive);

        let record = serde_json::to_string(&signal).expect("serialize");
        // DERIVED: reduction factor = artifact bytes divided by signal bytes.
        let reduction = payload.len() as f64 / record.len() as f64;
        assert!(
            reduction > 20.0,
            "expected a >20x reduction, measured {reduction:.1}x \
             ({} signal bytes for {} artifact bytes)",
            record.len(),
            payload.len()
        );
        drop(dir);
    }

    #[test]
    fn pptx_signal_lists_slide_titles_in_presentation_order() {
        let payload = build_package(PPTX_PARTS);
        let (dir, path) = staged("aula.pptx", &payload);
        let signal = signal_of(&path).expect("pptx signal");

        assert_eq!(signal.kind, "presentation");
        assert_eq!(signal.format, "ooxml-pptx");
        let slides = signal.slides.as_ref().expect("slides");
        assert_eq!(slides.len(), 2);
        // `rId2` targets slide2, so that slide is first in presentation order.
        assert_eq!(slides[0].index, 1);
        assert_eq!(slides[0].title, "Titulo de abertura");
        assert_eq!(slides[1].title, "Primeiro assunto");
        drop(dir);
    }

    #[test]
    fn pdf_signal_carries_pages_geometry_and_info() {
        let payload = minimal_pdf();
        let (dir, path) = staged("memoria.pdf", &payload);
        let signal = signal_of(&path).expect("pdf signal");

        assert_eq!(signal.kind, "pdf");
        assert_eq!(signal.format, "pdf");
        assert_eq!(signal.pages, Some(1));
        assert_eq!(signal.width, Some(612));
        assert_eq!(signal.height, Some(792));
        assert_eq!(signal.title.as_deref(), Some("Memoria de calculo"));
        assert_eq!(signal.author.as_deref(), Some("Engenharia"));
        assert_eq!(signal.producer.as_deref(), Some("atomwrite"));
        drop(dir);
    }

    #[test]
    fn png_signal_carries_dimensions_and_no_pixels() {
        let payload = small_png();
        let (dir, path) = staged("icone.png", &payload);
        let signal = signal_of(&path).expect("png signal");

        assert_eq!(signal.kind, "image");
        assert_eq!(signal.format, "png");
        assert_eq!(signal.width, Some(2));
        assert_eq!(signal.height, Some(3));

        // The schema carries geometry only. Checked per key, not by substring:
        // `megapixels` legitimately contains the word "pixels".
        let record = serde_json::to_value(&signal).expect("serialize");
        assert!(record.get("pixels").is_none(), "no pixel field exists");
        assert!(record.get("data").is_none(), "no blob field exists");
        assert!(record.get("pixels_data").is_none());
        drop(dir);
    }

    #[test]
    fn corrupt_container_fails_closed() {
        // Local-file header then garbage: looks like OOXML, is not.
        let mut payload = b"PK\x03\x04".to_vec();
        payload.extend_from_slice(&[0u8; 200]);
        let (dir, path) = staged("quebrado.xlsx", &payload);
        let error = signal_of(&path).expect_err("corrupt container must fail");

        let aw = error
            .downcast_ref::<AtomwriteError>()
            .expect("domain error, not a bare io error");
        assert_eq!(aw.error_code(), "INVALID_INPUT");
        assert_eq!(aw.exit_code(), 65);
        assert!(
            aw.to_string().contains("corrupt artifact"),
            "the cause must be stated, got {aw}"
        );
        drop(dir);
    }

    #[test]
    fn truncated_pdf_fails_closed() {
        let (dir, path) = staged("cortado.pdf", b"%PDF-1.4\ntrailer<</Root");
        let error = signal_of(&path).expect_err("truncated PDF must fail");
        let aw = error
            .downcast_ref::<AtomwriteError>()
            .expect("domain error, not a bare io error");
        assert_eq!(aw.error_code(), "INVALID_INPUT");
        assert_eq!(aw.exit_code(), 65);
        assert!(aw.to_string().contains("corrupt artifact"), "got {aw}");
        drop(dir);
    }

    #[test]
    fn truncated_png_fails_closed() {
        // Valid signature, then a header too short to carry an extent.
        let (dir, path) = staged(
            "cortado.png",
            b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00",
        );
        let error = signal_of(&path).expect_err("truncated PNG must fail");
        let aw = error
            .downcast_ref::<AtomwriteError>()
            .expect("domain error, not a bare io error");
        assert_eq!(aw.error_code(), "INVALID_INPUT");
        assert_eq!(aw.exit_code(), 65);
        assert!(aw.to_string().contains("corrupt artifact"), "got {aw}");
        drop(dir);
    }

    #[test]
    fn unsupported_bytes_name_the_supported_set() {
        let (dir, path) = staged("notas.txt", b"plain text, no artifact magic here\n");
        let error = signal_of(&path).expect_err("plain text is not an artifact");
        let aw = error
            .downcast_ref::<AtomwriteError>()
            .expect("domain error, not a bare io error");
        assert_eq!(aw.error_code(), "INVALID_INPUT");
        let message = aw.to_string();
        assert!(message.contains("unsupported artifact"), "got {message}");
        assert!(message.contains("pdf"), "the supported list must be stated");
        drop(dir);
    }

    #[test]
    fn missing_artifact_fails_closed_as_not_found() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("ausente.pdf");
        let error = signal_of(&path).expect_err("missing file must fail");
        let aw = error
            .downcast_ref::<AtomwriteError>()
            .expect("domain error");
        assert_eq!(aw.error_code(), "FILE_NOT_FOUND");
        assert_eq!(aw.exit_code(), 4);
        drop(dir);
    }

    #[test]
    fn directory_target_fails_closed_naming_the_directive_command() {
        let dir = tempfile::tempdir().expect("temp dir");
        let error = signal_of(dir.path()).expect_err("a directory is not an artifact");
        let aw = error
            .downcast_ref::<AtomwriteError>()
            .expect("domain error");
        let message = aw.to_string();
        assert!(message.contains("directory"), "got {message}");
        assert!(
            message.contains("list"),
            "the corrective command must be named"
        );
        drop(dir);
    }

    #[test]
    fn max_text_chars_bounds_the_excerpt_not_the_measured_length() {
        let long_text: String = "palavra ".repeat(2_000);
        let document = format!(
            r#"<w:document><w:body><w:p><w:r><w:t>{long_text}</w:t></w:r></w:p></w:body></w:document>"#
        );
        let payload = build_package(&[("word/document.xml", document.as_str())]);
        let (dir, path) = staged("longo.docx", &payload);
        let signal = extract_signal(&path, DEFAULT_MAX_SAMPLE_ROWS, 50, u64::MAX)
            .expect("bounded document signal");

        let excerpt = signal.text_excerpt.as_deref().expect("excerpt");
        assert_eq!(excerpt.chars().count(), 50);
        let total = signal.text_chars.expect("text_chars");
        assert!(
            total > 50,
            "the true length is reported, not the truncated one"
        );
        drop(dir);
    }

    #[test]
    fn declared_part_size_above_the_budget_fails_closed() {
        let payload = build_package(DOCX_PARTS);
        let (dir, path) = staged("contrato.docx", &payload);
        // A 32-byte budget is below every part, so the bound must refuse the read.
        let error = extract_signal(&path, DEFAULT_MAX_SAMPLE_ROWS, DEFAULT_MAX_TEXT_CHARS, 32)
            .expect_err("an oversized part must be refused");
        let aw = error
            .downcast_ref::<AtomwriteError>()
            .expect("domain error");
        assert_eq!(aw.error_code(), "FILE_TOO_LARGE");
        drop(dir);
    }
}
