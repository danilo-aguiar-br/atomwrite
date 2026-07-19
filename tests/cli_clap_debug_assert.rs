// SPDX-License-Identifier: MIT OR Apache-2.0

//! Clap command definition integrity: `CommandFactory::debug_assert`.
//!
//! Required by rules_rust_cli_com_clap — catches developer errors in the
//! derive contract (id collisions, conflicting shorts, invalid defaults).

use atomwrite::cli::Cli;
use clap::CommandFactory;

#[test]
fn cli_command_debug_assert() {
    Cli::command().debug_assert();
}
