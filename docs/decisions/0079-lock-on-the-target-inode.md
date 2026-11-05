# ADR-0079 — The lock is taken on the target inode

## Status
Accepted (2026-11-03), supersedes the sidecar requirement of ADR-0031

## Context
- The exclusive lock was taken on a sidecar file `.<name>.atomwrite.lock` created beside the target.
- Creating that name needs write permission on the directory, and an in-place write never needs it: the file's own permission is enough.
- A caller that owned a file inside a directory it could not write to was refused with exit 13, and the message named the directory while the request named the file.
- The refusal looked like security, so nobody looked for the defect: the lock protected a name instead of the object the verb mutates.
- Reentrancy was keyed on the sidecar path, so two spellings that reached the same file through different links shared no lock at all.

## Decision
### 1. The subject of the lock is the inode
- `acquire_exclusive` opens the target read and write and locks that handle with `std::fs::File::try_lock`, which is `flock` on Unix and `LockFileEx` on Windows.
- The inode that is locked is the inode that is written, so an in-place write holds the lock on the exact object it mutates.
- A target that does not exist has no inode to lock, and the sidecar carries that case, which is the new-file write.
- A target that cannot be opened for writing falls back to the sidecar and lets the write fail later on its own terms.

### 2. The sidecar becomes compatibility, not correctness
- The sidecar is still taken after the inode, best effort, so an older atomwrite that only knows the sidecar remains mutually excluded with this one.
- A sidecar that cannot be created no longer aborts a run the inode lock already covers, and the failure is logged at debug level.
- The sidecar stays for the two-version migration window and leaves the correctness proof.
- A sidecar handle that no longer answers to its path is retaken, because the writer that released it unlinked the file, and a lock on an unreachable inode excludes nobody.
- The retake is bounded, and a directory this process cannot write to still leaves the run to the inode lock alone.

### 3. Reentrancy is keyed on the target path
- The thread-local table is keyed on the resolved target path instead of the sidecar path, so nesting is free for the object being written.
- A second thread of the same process asks the OS and waits, exactly as another process would.

### 4. The swap revalidates the inode
- `LockGuard::target_unchanged` compares the locked handle with the path through `same_file::Handle`, which is already a dependency.
- `atomic_write_based` calls it immediately before the strategy dispatch, and a path that now answers to another inode fails with `STATE_DRIFT` (exit 82) before anything is written.
- A refusal discards the journal, because nothing was swapped and there is nothing to recover.
- The check is skipped when `expected_base` is `Some`, because a verb that read the file under this same lock already proved the content it is about to replace.
- `acquire_exclusive` compares the locked inode against the path with `target_is_same_inode` right after the lock lands.
- A locked inode that no longer answers to the path is a swap that happened between the open and the lock, so the acquisition restarts from a fresh open of the path.
- The restart is bounded by `LOCK_STALE_INODE_MAX_RESTARTS`, and the last attempt keeps whatever handle it holds instead of looping forever.

### 5. A read-only directory selects the in-place strategy
- The strategy used to be `Rename` for any regular file, and a rename needs a tempfile beside the target.
- `parent_is_writable` asks `access(2)` for `W_OK`, and an unwritable directory selects `InPlace`, which is the only strategy that can run there.
- The result is that a file the caller owns is written even when its directory is read-only, and the exit code is no longer 13 for that case.

### 6. The verbs that read before they write pass the base
- `apply`, `set`, `del` and `edit` hand the BLAKE3 digest of the content they read to `atomic_write_based`.
- That digest is the safety net on filesystems where the advisory lock does not exist.

## Consequences
- A write into a directory the process cannot write to succeeds through the in-place path, and the sidecar is not created there.
- Two writers on the same file still serialize, and a lost update is refused instead of being silent.
- Windows remains unvalidated: locking the target handle may block the rename through a sharing violation, and that needs proof on a real machine.
- NFS keeps its old limitation, because `flock` is advisory there and may be ignored; the base digest is the net.
- `ARCHITECTURE.md`, `docs/CROSS_PLATFORM.md`, `CHANGELOG.md` and its `.pt-BR` twin state the new model and the two limits above.
