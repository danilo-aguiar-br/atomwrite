// SPDX-License-Identifier: MIT OR Apache-2.0
// `--help` examples of every subcommand, included from cli.rs.
//
// Each constant is the `after_long_help` of one subcommand: two copyable
// invocations. The test at the bottom parses every line against the real clap
// tree, so an example cannot name a flag or a subcommand that does not exist.

const EX_READ: &str = r#"EXAMPLES:
  atomwrite --workspace . read --head 20 src/main.rs
  atomwrite --workspace . read --lines 10:30 src/main.rs
  atomwrite --workspace . read --format raw src/main.rs src/cli.rs
  atomwrite --workspace . read --format raw 'src/*.rs'"#;
const EX_WRITE: &str = r#"EXAMPLES:
  printf 'hello\n' | atomwrite --workspace . write notes.txt
  printf 'more\n' | atomwrite --workspace . write --append notes.txt"#;
const EX_EDIT: &str = r#"EXAMPLES:
  atomwrite --workspace . edit src/main.rs --old foo --new bar
  atomwrite --workspace . edit src/main.rs --old foo --new bar --replace-all --fuzzy off"#;
const EX_SEARCH: &str = r#"EXAMPLES:
  atomwrite --workspace . search TODO src/
  atomwrite --workspace . search -F 'fn main' --include '*.rs' src/"#;
const EX_REPLACE: &str = r#"EXAMPLES:
  atomwrite --workspace . replace --dry-run old_name new_name src/
  atomwrite --workspace . replace -F --fuzzy off old_name new_name src/"#;
const EX_HASH: &str = r#"EXAMPLES:
  atomwrite --workspace . hash src/main.rs
  atomwrite --workspace . hash --recursive src/"#;
const EX_DELETE: &str = r#"EXAMPLES:
  atomwrite --workspace . delete --plan old.txt
  atomwrite --workspace . delete --recursive --plan tmp/"#;
const EX_COUNT: &str = r#"EXAMPLES:
  atomwrite --workspace . count --by-extension src/
  atomwrite --workspace . count --by-size --top 5 src/"#;
const EX_DIFF: &str = r#"EXAMPLES:
  atomwrite --workspace . diff --unified a.txt b.txt
  atomwrite --workspace . diff --stat a.txt b.txt"#;
const EX_MOVE: &str = r#"EXAMPLES:
  atomwrite --workspace . move old.txt new.txt
  atomwrite --workspace . move --force --dry-run a.txt b.txt"#;
const EX_COPY: &str = r#"EXAMPLES:
  atomwrite --workspace . copy a.txt copy_of_a.txt
  atomwrite --workspace . copy --recursive src/ src_copy/"#;
const EX_LIST: &str = r#"EXAMPLES:
  atomwrite --workspace . list --long --depth 2 src/
  atomwrite --workspace . list --count-by-ext ."#;
const EX_EXTRACT: &str = r#"EXAMPLES:
  printf '{"a":1,"b":2}\n' | atomwrite --workspace . extract --field a
  printf 'alpha beta gamma\n' | atomwrite --workspace . extract --stdin 1"#;
const EX_CALC: &str = r#"EXAMPLES:
  atomwrite --workspace . calc "2 hours to seconds"
  atomwrite --workspace . calc "15% of 200""#;
const EX_REGEX: &str = r#"EXAMPLES:
  atomwrite --workspace . regex abc-12 xyz-99 id-7 --digits
  atomwrite --workspace . regex --digits --repetitions user1 user22 user333"#;
const EX_TRANSFORM: &str = r#"EXAMPLES:
  atomwrite --workspace . transform --dry-run -p '$A == false' -r '!$A' -l rust src/
  atomwrite --workspace . transform --dry-run -p 'println!($$$ARGS)' -r 'log::info!($$$ARGS)' -l rust src/"#;
const EX_SCOPE: &str = r#"EXAMPLES:
  atomwrite --workspace . scope src/ --language rust --query comments --delete --dry-run
  atomwrite --workspace . scope src/ -l rust --query strings --action upper --dry-run"#;
const EX_BATCH: &str = r#"EXAMPLES:
  echo '{"op":"write","target":"a.txt","content":"x"}' | atomwrite --workspace . batch --transaction
  atomwrite --workspace . batch --dry-run --file ops.ndjson"#;
const EX_BACKUP: &str = r#"EXAMPLES:
  atomwrite --workspace . backup src/main.rs --retention 3
  atomwrite --workspace . backup --dry-run src/main.rs"#;
const EX_ROLLBACK: &str = r#"EXAMPLES:
  atomwrite --workspace . rollback --latest --verify src/main.rs
  atomwrite --workspace . rollback --dry-run src/main.rs"#;
const EX_APPLY: &str = r#"EXAMPLES:
  atomwrite --workspace . apply --dry-run src/main.rs < change.diff
  atomwrite --workspace . apply --format full notes.txt < new_notes.txt"#;
const EX_SET: &str = r#"EXAMPLES:
  atomwrite --workspace . set Cargo.toml package.name demo
  atomwrite --workspace . set config.json server.port 8080"#;
const EX_GET: &str = r#"EXAMPLES:
  atomwrite --workspace . get Cargo.toml package.version
  atomwrite --workspace . get config.json server.port"#;
const EX_DEL: &str = r#"EXAMPLES:
  atomwrite --workspace . del Cargo.toml features.old
  atomwrite --workspace . del --force-missing config.json server.debug"#;
const EX_CASE: &str = r#"EXAMPLES:
  atomwrite --workspace . case --subvert user_id userId --to camel --dry-run src/main.rs
  atomwrite --workspace . case --subvert UserId user-id --to kebab src/main.rs"#;
const EX_QUERY: &str = r#"EXAMPLES:
  atomwrite --workspace . query --kinds src/main.rs
  atomwrite --workspace . query -Q '(function_item name: (identifier) @name)' src/main.rs"#;
const EX_OUTLINE: &str = r#"EXAMPLES:
  atomwrite --workspace . outline src/main.rs
  atomwrite --workspace . outline --kind function_item --positions src/main.rs"#;
const EX_WAL_STATS: &str = r#"EXAMPLES:
  atomwrite --workspace . wal-stats
  atomwrite --workspace . wal-stats --dry-run"#;
const EX_WAL_HEAL: &str = r#"EXAMPLES:
  atomwrite --workspace . wal-heal --dry-run .
  atomwrite --workspace . wal-heal --threshold-secs 600 ."#;
const EX_SEMANTIC_MERGE: &str = r#"EXAMPLES:
  atomwrite --workspace . semantic-merge --base base.txt --ours ours.txt --theirs theirs.txt --output merged.txt
  atomwrite --workspace . semantic-merge --base base.txt --ours ours.txt --theirs theirs.txt --output merged.txt --fail-on-conflict"#;
const EX_SPARSE: &str = r#"EXAMPLES:
  atomwrite --workspace . sparse list src/ --max-files 50 --include '*.rs'
  atomwrite --workspace . sparse outline src/ --max-files 10"#;
const EX_RECIPE: &str = r#"EXAMPLES:
  atomwrite --workspace . recipe list
  atomwrite --workspace . recipe run search-replace-verify --pattern old --replacement new --path src --dry-run"#;
const EX_STAT: &str = r#"EXAMPLES:
  atomwrite --workspace . stat src/main.rs
  atomwrite --workspace . stat Cargo.toml"#;
const EX_AGENT_SURFACE: &str = r#"EXAMPLES:
  atomwrite --workspace . agent-surface
  atomwrite --workspace . --select version,integration agent-surface"#;
const EX_WATCH: &str = r#"EXAMPLES:
  atomwrite --workspace . watch src/ --max-events 100 --idle-exit-ms 500
  atomwrite --workspace . watch src/ --checksum --max-events 10"#;
const EX_EXTRACT_ARTIFACT: &str = r#"EXAMPLES:
  atomwrite --workspace . extract-artifact report.pdf
  atomwrite --workspace . extract-artifact --max-text-chars 200 logo.png"#;
const EX_CODEMOD: &str = r#"EXAMPLES:
  atomwrite --workspace . codemod --rules rules.yaml --dry-run src/
  atomwrite --workspace . codemod --rules rules.yaml src/"#;
const EX_SEMANTIC_SEARCH: &str = r#"EXAMPLES:
  atomwrite --workspace . semantic-search "atomic write" src/ --k 10
  atomwrite --workspace . semantic-search "fsync rename" src/ --min-score 0.1"#;
const EX_DOCTOR: &str = r#"EXAMPLES:
  atomwrite --workspace . doctor
  atomwrite --workspace . doctor --strict"#;
const EX_SCRATCH: &str = r#"EXAMPLES:
  atomwrite --workspace . scratch init .atomwrite/scratch
  atomwrite --workspace . --select relative,already_existed scratch init .atomwrite/scratch"#;
const EX_LOCALE: &str = r#"EXAMPLES:
  atomwrite --workspace . locale
  atomwrite --workspace . locale --set pt-BR"#;
const EX_COMMANDS: &str = r#"EXAMPLES:
  atomwrite --workspace . commands
  atomwrite --workspace . commands --include-globals"#;
const EX_COMPLETIONS: &str = r#"EXAMPLES:
  atomwrite --workspace . completions bash
  atomwrite --workspace . completions fish --install"#;
const EX_PRUNE_BACKUPS: &str = r#"EXAMPLES:
  atomwrite --workspace . prune-backups --max-age-secs 7200 --dry-run src/main.rs
  atomwrite --workspace . prune-backups --max-count 3 src/main.rs"#;
const EX_EDIT_LOOP: &str = r#"EXAMPLES:
  echo '[{"old":"foo","new":"bar"}]' | atomwrite --workspace . edit-loop src/main.rs
  echo '[{"old":"foo","new":"bar"}]' | atomwrite --workspace . edit-loop --syntax-check rust src/main.rs"#;
const EX_VERIFY: &str = r#"EXAMPLES:
  atomwrite --workspace . verify src/main.rs "$CHECKSUM"
  atomwrite --workspace . verify Cargo.toml "$(atomwrite --workspace . hash Cargo.toml | jaq -r .checksum)""#;
/// The part of `config --help` that is not the key table.
///
/// The table above the `SCOPES` block is generated from `KEYS` by
/// `Cli::parser`, because a hand-written copy listed 22 keys while the table
/// listed 35. This half stays a literal: it describes the scopes, which are a
/// closed enum no key can add to.
pub(crate) const EX_CONFIG_SCOPES: &str = r#"SCOPES:
  global     the user config.toml in the XDG config directory
  workspace  the .atomwrite.toml at the workspace root
  Scope "both" accepts either file. A "global" key is rejected in the workspace scope (exit 65).
  `set`, `unset` and `path` require --scope. `list` and `get` do not take it.

ORIGIN of a value in `list` and `get`: default, global, workspace, or explicit when --config names the file.

EXAMPLES:
  atomwrite --workspace . config list
  atomwrite --workspace . config get fuzzy.max_content_bytes
  atomwrite --workspace . config set fuzzy.threshold 0.9 --scope workspace"#;

#[cfg(test)]
mod examples_tests {
    use clap::{CommandFactory, Parser};

    use super::*;

    /// Split one shell-like line into words, honouring single and double quotes.
    fn words(line: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut cur = String::new();
        let mut quote: Option<char> = None;
        let mut started = false;
        for ch in line.chars() {
            match (quote, ch) {
                (Some(q), c) if c == q => quote = None,
                (Some(_), c) => cur.push(c),
                (None, '\'' | '"') => {
                    quote = Some(ch);
                    started = true;
                }
                (None, c) if c.is_whitespace() => {
                    if started || !cur.is_empty() {
                        out.push(std::mem::take(&mut cur));
                        started = false;
                    }
                }
                (None, c) => cur.push(c),
            }
        }
        if started || !cur.is_empty() {
            out.push(cur);
        }
        out
    }

    /// Every `atomwrite ...` invocation of an examples text, up to a pipe or redirect.
    fn invocations(text: &str) -> Vec<Vec<String>> {
        text.lines()
            .filter_map(|line| line.find("atomwrite ").map(|at| &line[at..]))
            .map(|call| {
                let mut call = words(call);
                if let Some(end) = call.iter().position(|w| w == "|" || w == "<") {
                    call.truncate(end);
                }
                call
            })
            .collect()
    }

    #[test]
    fn every_example_parses_against_the_real_cli() {
        let all: [(&str, &str); 47] = [
            ("read", EX_READ),
            ("write", EX_WRITE),
            ("edit", EX_EDIT),
            ("search", EX_SEARCH),
            ("replace", EX_REPLACE),
            ("hash", EX_HASH),
            ("delete", EX_DELETE),
            ("count", EX_COUNT),
            ("diff", EX_DIFF),
            ("move", EX_MOVE),
            ("copy", EX_COPY),
            ("list", EX_LIST),
            ("extract", EX_EXTRACT),
            ("calc", EX_CALC),
            ("regex", EX_REGEX),
            ("transform", EX_TRANSFORM),
            ("scope", EX_SCOPE),
            ("batch", EX_BATCH),
            ("backup", EX_BACKUP),
            ("rollback", EX_ROLLBACK),
            ("apply", EX_APPLY),
            ("set", EX_SET),
            ("get", EX_GET),
            ("del", EX_DEL),
            ("case", EX_CASE),
            ("query", EX_QUERY),
            ("outline", EX_OUTLINE),
            ("wal-stats", EX_WAL_STATS),
            ("wal-heal", EX_WAL_HEAL),
            ("semantic-merge", EX_SEMANTIC_MERGE),
            ("sparse", EX_SPARSE),
            ("recipe", EX_RECIPE),
            ("stat", EX_STAT),
            ("agent-surface", EX_AGENT_SURFACE),
            ("watch", EX_WATCH),
            ("extract-artifact", EX_EXTRACT_ARTIFACT),
            ("codemod", EX_CODEMOD),
            ("semantic-search", EX_SEMANTIC_SEARCH),
            ("doctor", EX_DOCTOR),
            ("scratch", EX_SCRATCH),
            ("locale", EX_LOCALE),
            ("commands", EX_COMMANDS),
            ("completions", EX_COMPLETIONS),
            ("prune-backups", EX_PRUNE_BACKUPS),
            ("edit-loop", EX_EDIT_LOOP),
            ("verify", EX_VERIFY),
            ("config", EX_CONFIG_SCOPES),
        ];
        let known: Vec<String> = Cli::command()
            .get_subcommands()
            .map(|c| c.get_name().to_string())
            .collect();
        assert_eq!(known.len(), all.len(), "one examples constant per subcommand");
        for (name, text) in all {
            assert!(known.iter().any(|k| k == name), "unknown subcommand {name}");
            let calls = invocations(text);
            assert!(calls.len() >= 2, "{name}: needs at least two examples");
            for call in calls {
                assert!(
                    call.iter().any(|w| w == name),
                    "{name}: example does not call it: {call:?}"
                );
                if let Err(e) = Cli::try_parse_from(&call) {
                    panic!("{name}: example {call:?} does not parse: {e}");
                }
            }
        }
    }
}
