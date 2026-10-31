// SPDX-License-Identifier: MIT OR Apache-2.0
// Reading the `--join` table from stdin.
// Included from output_shape.inc.rs.

/// Whether the subcommand reads stdin for its own input.
///
/// `--join` also reads stdin, and two readers on one pipe would split the bytes
/// between them, so a run that asks for both is refused before either reads.
/// The list follows the dispatch table: only the verbs handed the stdin stream
/// can consume it, and some of them only under a flag or an argument.
pub fn command_consumes_stdin(command: &crate::cli::Commands) -> bool {
    use crate::cli::Commands;
    match command {
        Commands::Write(_)
        | Commands::Edit(_)
        | Commands::EditLoop(_)
        | Commands::Apply(_)
        | Commands::Extract(_) => true,
        Commands::Hash(args) => args.stdin,
        Commands::Calc(args) => args.stdin || args.expression.is_none(),
        Commands::Regex(args) => args.stdin || args.examples.is_empty(),
        Commands::Diff(args) => {
            args.file_a.as_os_str() == "-" || args.file_b.as_os_str() == "-"
        }
        Commands::Batch(args) => args.file.is_none(),
        _ => false,
    }
}

/// Read the `--join` tables once, from `stdin`.
///
/// Returns no table (and reads nothing) when no `--join` was given.
///
/// # Errors
///
/// Returns `INVALID_INPUT` when the command already consumes stdin, stdin is a
/// terminal, the table is larger than [`crate::constants::JOIN_TABLE_MAX_BYTES`],
/// a line is not a JSON object, or a join key is absent from the table. A
/// cancelled read returns the typed cancel error.
pub fn read_join_tables(
    stdin: &mut impl std::io::Read,
    stdin_is_tty: bool,
    command_consumes_stdin: bool,
    specs: &[JoinSpec],
) -> anyhow::Result<Vec<JoinTable>> {
    use crate::constants::{JOIN_REASON_MARKER, JOIN_TABLE_MAX_BYTES};
    let refuse = |why: &str| -> anyhow::Error {
        crate::error::AtomwriteError::InvalidInput {
            reason: format!("{JOIN_REASON_MARKER} {why}"),
        }
        .into()
    };
    if specs.is_empty() {
        return Ok(Vec::new());
    }
    if command_consumes_stdin {
        return Err(refuse(
            "reads its table from stdin, but this command already reads stdin; \
             pass the table to a command that does not (search, list, count, ...)",
        ));
    }
    if stdin_is_tty {
        return Err(refuse(
            "reads its table from stdin, but stdin is a terminal; pipe an NDJSON table into the command",
        ));
    }
    // The shared reader owns the chunk size, the cap and the cancellation
    // token. Overflow comes back as `FileTooLarge`, which this path rewords as
    // the `--join` refusal the rest of the surface uses.
    let raw = match crate::commands::read_stdin_content_cancellable(
        stdin,
        Some(JOIN_TABLE_MAX_BYTES),
        crate::commands::StdinCapOrigin::LocalMaxSize,
        true,
        None,
    ) {
        Ok((bytes, _)) => bytes,
        Err(err) => {
            let too_large = err
                .downcast_ref::<crate::error::AtomwriteError>()
                .is_some_and(|e| {
                    matches!(e, crate::error::AtomwriteError::InvalidInput { reason } if reason.contains("stdin exceeds max size"))
                });
            if too_large {
                return Err(refuse(&format!(
                    "table on stdin is larger than {JOIN_TABLE_MAX_BYTES} bytes"
                )));
            }
            return Err(err);
        }
    };
    let text = String::from_utf8(raw).map_err(|_| refuse("table on stdin is not valid UTF-8"))?;
    Ok(parse_join_stdin(&text, specs)?)
}
