//! Text recognition for a page or a picture that carries no text layer.
//!
//! A scanned document has pixels and no characters, so every verb that reads
//! text answers "nothing" and the caller believes the page is empty. This
//! module turns the rendered page back into text inside the same process, on
//! pure Rust (`rten` runs the models; no Tesseract, no C toolchain).
//!
//! The two model files are fetched once into the cache directory and reused
//! from then on. Recognition never happens unless the caller asks for it, and a
//! page that already carries text is never sent here.

use std::path::{Path, PathBuf};

use anyhow::Result;
use image::DynamicImage;
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;

use crate::error::AtomwriteError;

/// Run the engine over one picture and return its lines, top to bottom.
///
/// A picture with no text yields an empty string, which is a real answer: the
/// engine saw the page and found no text on it.
pub(crate) fn recognize(picture: &DynamicImage) -> Result<String> {
    let engine = engine()?;
    let rgb = picture.to_rgb8();
    let source = ImageSource::from_bytes(rgb.as_raw(), rgb.dimensions()).map_err(|error| {
        AtomwriteError::InvalidInput {
            reason: format!("ocr: the picture cannot be read as RGB pixels: {error}"),
        }
    })?;
    let input = engine
        .prepare_input(source)
        .map_err(|error| AtomwriteError::InternalError {
            reason: format!("ocr: preprocessing failed: {error}"),
        })?;
    let words = engine
        .detect_words(&input)
        .map_err(|error| AtomwriteError::InternalError {
            reason: format!("ocr: word detection failed: {error}"),
        })?;
    let lines = engine.find_text_lines(&input, &words);
    let recognized =
        engine
            .recognize_text(&input, &lines)
            .map_err(|error| AtomwriteError::InternalError {
                reason: format!("ocr: recognition failed: {error}"),
            })?;
    let mut text = String::new();
    for line in recognized.iter().flatten() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&line.to_string());
    }
    Ok(text)
}

/// Build the engine once per call, from the cached model files.
fn engine() -> Result<OcrEngine> {
    let params = OcrEngineParams {
        detection_model: Some(load_model(crate::constants::OCR_DETECTION_MODEL_FILE)?),
        recognition_model: Some(load_model(crate::constants::OCR_RECOGNITION_MODEL_FILE)?),
        ..OcrEngineParams::default()
    };
    OcrEngine::new(params)
        .map_err(|error| AtomwriteError::InternalError {
            reason: format!("ocr: engine refused the models: {error}"),
        })
        .map_err(Into::into)
}

/// Load one model file from the cache, fetching it when it is not there yet.
fn load_model(file: &str) -> Result<Model> {
    let path = model_path(file)?;
    if !path.is_file() {
        download(file, &path)?;
    }
    Model::load_file(&path)
        .map_err(|error| AtomwriteError::InvalidInput {
            reason: format!(
                "ocr: {} is not a usable model file, so remove it and run again: {error}",
                path.display()
            ),
        })
        .map_err(Into::into)
}

/// Where the model files live between runs.
fn model_path(file: &str) -> Result<PathBuf> {
    let cache = crate::storage::cache_dir().ok_or_else(|| AtomwriteError::InvalidInput {
        reason: "ocr: there is no cache directory on this system to keep the models in".to_owned(),
    })?;
    Ok(cache.join(crate::constants::OCR_MODEL_DIR_NAME).join(file))
}

/// Fetch one model file and land it in the cache in a single step.
fn download(file: &str, path: &Path) -> Result<()> {
    let url = format!("{}/{file}", crate::constants::OCR_MODEL_BASE_URL);
    let response = ureq::get(&url)
        .config()
        .timeout_global(Some(crate::constants::OCR_MODEL_DOWNLOAD_TIMEOUT))
        .build()
        .call()
        .map_err(|error| AtomwriteError::InvalidInput {
            reason: format!("ocr: the model at {url} could not be fetched: {error}"),
        })?;
    let mut body = response.into_body();
    let bytes = body
        .with_config()
        .limit(crate::constants::OCR_MODEL_MAX_BYTES)
        .read_to_vec()
        .map_err(|error| AtomwriteError::Io {
            source: std::io::Error::other(format!("ocr: {url}: {error}")),
        })?;
    let parent = path.parent().ok_or_else(|| AtomwriteError::InternalError {
        reason: format!("ocr: {} has no parent directory", path.display()),
    })?;
    std::fs::create_dir_all(parent).map_err(|source| AtomwriteError::Io { source })?;
    let temp = parent.join(format!(".{file}.part"));
    std::fs::write(&temp, &bytes).map_err(|source| AtomwriteError::Io { source })?;
    std::fs::rename(&temp, path).map_err(|source| AtomwriteError::Io { source })?;
    tracing::debug!(
        url = %url,
        bytes = bytes.len(),
        path = %path.display(),
        "OCR model fetched"
    );
    Ok(())
}
