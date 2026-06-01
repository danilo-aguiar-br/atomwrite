// SPDX-License-Identifier: MIT OR Apache-2.0

//! Math expression evaluation and unit conversion via fend.
//! Workload: CPU-bound (expression parsing + evaluation).

use std::io::{BufRead, Read, Write};
use std::time::Instant;

use anyhow::Result;

use crate::cli::CalcArgs;
use crate::error::AtomwriteError;
use crate::ndjson_types::CalcOutput;
use crate::output::NdjsonWriter;

/// Evaluate math expressions and unit conversions via the fend engine.
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` if the expression cannot be evaluated.
#[tracing::instrument(skip_all, fields(command = "calc"))]
pub fn cmd_calc(
    args: &CalcArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let mut ctx = fend_core::Context::new();

    if args.stdin || args.expression.is_none() {
        let reader = std::io::BufReader::with_capacity(crate::constants::BUF_CAPACITY, stdin);
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            evaluate_one(trimmed, &mut ctx, writer)?;
        }
    } else if let Some(ref expr) = args.expression {
        evaluate_one(expr, &mut ctx, writer)?;
    }

    Ok(())
}

fn evaluate_one(
    expression: &str,
    ctx: &mut fend_core::Context,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();

    let result =
        fend_core::evaluate(expression, ctx).map_err(|e| AtomwriteError::InvalidInput {
            reason: format!("calc error: {e}"),
        })?;

    let output = CalcOutput {
        r#type: "calc",
        expression: expression.to_owned(),
        result: result.get_main_result().to_owned(),
        elapsed_ms: start.elapsed().as_millis() as u64,
    };

    writer.write_event(&output)?;
    Ok(())
}
