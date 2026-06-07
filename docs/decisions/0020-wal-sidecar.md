# ADR-0020: WAL sidecar file path and shape

- **Status**: Accepted
- **Date**: 2026-06-07
- **Context**: G114 (Crash Recovery Audit) needs a write-ahead log so a process killed mid-`atomic_write` leaves recoverable breadcrumbs. We already use `.<target>.atomwrite.lock` for advisory `flock(2)`. We needed a separate sidecar convention that does not collide with the lock sidecar.
- **Decision**: WAL sidecar is `.atomwrite.journal.<basename>.atomwrite.journal.json`. Each `atomic_write` opens/creates this file and appends a JSONL line per phase. The file lives in the same directory as the target. On success, the final `Committed` line is written and the file is NOT deleted (cheap; humans can `rm` it; future ops overwrite the same path). On process death between `Started` and `Committed`, the sidecar becomes an "orphan journal".
- **Consequences**:
  - **+** Path is deterministic and unique per target, so recovery tools can scan a directory and find orphans by glob.
  - **+** JSONL format (one object per line) is easy to parse with `jaq` and `rg`; no need for a full JSON library at recovery time.
  - **+** Recovery (`recover_orphan_journals(dir)`) is consultative: it reads and reports orphans but never touches the filesystem. The caller decides whether to replay, abort, or clean up.
  - **-** We do not auto-clean committed journals; an unused journal can pile up. A future `atomwrite wal-gc` subcommand can clean entries older than 7 days (deferred to v0.1.13).
  - **-** A pathological process that crashes 1000 times in a row will leave 1000 lines in the sidecar. We cap at 1000 lines per file (deferred to v0.1.13).
- **Alternatives considered**:
  1. Single global `/var/log/atomwrite.wal`. Rejected: not portable; permissions; not per-target.
  2. SQLite WAL. Rejected: requires `libsqlite3-sys` system dep, complicates cross-compile.
  3. Embed in xattr. Rejected: not all filesystems support xattr; FAT32, /tmp on overlayfs return `EOPNOTSUPP`.
- **Trigger to revisit**: If WAL sidecar size becomes a problem in CI, add `--wal-rotate-size N` to roll the sidecar.
