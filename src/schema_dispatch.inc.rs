// SPDX-License-Identifier: MIT OR Apache-2.0
// Schema dispatch, included by `schema_dispatch.inc.rs` inclusion from lib.rs.
//
// The surface is split in two so neither file carries it whole:
// `schema_names.inc.rs` holds the name tables, `schema_build.inc.rs` holds the
// builders that turn a name into a schema. `include!` places every item at the
// crate root, so `crate::SCHEMA_NAMES` and `crate::emit_schema_by_name` resolve
// without a re-export.

include!("schema_names.inc.rs");
include!("schema_build.inc.rs");
