# ADR-0083 — `--hidden` follows the flag in every walking verb


## Status
- Accepted (2026-11-03)
- Amends ADR-0082, which kept `hash` and `copy` out of the hidden rule, and completes ADR-0077, which owns the shared walk


## Context
- `hash` walked with `WalkerFlavor::AlwaysVisitHidden` in `src/commands/hash.rs`, and that flavor reaches `src/commands/mod.rs` as `.hidden(false)`
- `semantic-search` used the same flavor in `src/commands/semantic_search.rs`, and `copy -r` used `WholeTree`, whose name promised a visit of everything
- The `ignore` crate enables hidden filtering by default, so a verb that fixes `hidden(false)` makes the global flag inert by construction, and the caller cannot tell a covered dotfile from an absent one
- The comment in `src/commands/mod.rs` justified the flavor by saying a hash or a tree copy that skipped `.env` answers the wrong question, and that reasoning held while no flag asked for the dotfile on purpose
- The explicit path rule already answered the case the comment protected: a path named in the invocation is read regardless of the walk policy
- `sparse list` changed its default in the previous round and kept no way back inside the verb for a caller who wanted the hidden inventory


## Decision
- `hash` and `semantic-search` use `RespectGlobalFlags`, so `--hidden` decides and the default hides dotfiles
- `WholeTree` becomes `CopyTree`, and it returns `!global_hidden` from `skips_hidden` while keeping `honors_gitignore` false, so `copy -r` still copies what `.gitignore` lists
- A path named in the invocation is still read without the flag, because the invocation already named the subject
- `sparse list` and `sparse outline` take `--all`, which selects `AlwaysVisitHidden` inside the verb, and the default keeps hiding
- The rename is followed everywhere the variant appears, so no call site keeps the old promise in its name


## Consequences
- A script that hashed, copied or searched a tree without `--hidden` now receives the smaller inventory, and this is a breaking change published in the changelog and the migration guide
- The same tree answers the same file count in every walking verb, so a number from one predicts the others
- `copy -r` keeps copying a path listed in `.gitignore`, because the copy answer is the tree the caller named and not the tree git tracks
- `hash .env` and `copy .env` keep working, so the explicit target rule survives the policy change
- A caller who wants the old `copy -r` inventory passes `--hidden`, and a caller who wants the old `sparse list` inventory passes `--all`


## Alternatives
- Keep the flavor and document that `--hidden` is inert in the three verbs: rejected, because a flag that changes nothing is the divergence this round closes
- Remove `--hidden` from the global surface and let every verb keep its own policy: rejected, because it would multiply the policy and leave the explicit path rule undocumented
- Make `copy -r` honor `.gitignore` in the same change: rejected, because the copy answer has always been the named tree, and this round is about the hidden rule alone


## Trigger to revisit
- A verb whose answer must include dotfiles even when nobody asked, which would need its own flavor and its own reason
- An `ignore` release that changes the default of hidden filtering, which would make this decision describe a behavior the crate no longer has
