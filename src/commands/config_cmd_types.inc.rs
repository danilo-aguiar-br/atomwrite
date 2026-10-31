// SPDX-License-Identifier: MIT OR Apache-2.0

// Argument and output types of `config`, split out of `config_cmd.rs` so the
// verb stays below the 500-line ceiling.

/// Arguments for `config`.
#[derive(Args, Debug)]
pub struct ConfigArgs {
    /// Config action to run.
    #[command(subcommand)]
    pub action: ConfigAction,
}

/// Config actions.
#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Print the config file of one scope.
    Path(ConfigPathArgs),
    /// List every key with its effective value and where the value comes from.
    List,
    /// Show one key with its effective value and where the value comes from.
    Get(ConfigGetArgs),
    /// Write one key into the config file of a scope.
    Set(ConfigSetArgs),
    /// Remove one key from the config file of a scope.
    Unset(ConfigUnsetArgs),
    /// Load the effective config and report whether it is usable.
    Doctor,
}

/// Config file a key is written to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ConfigScope {
    /// The XDG `config.toml` of the current user.
    Global,
    /// The `.atomwrite.toml` at the workspace root.
    Workspace,
}

/// Arguments for `config path`.
#[derive(Args, Debug)]
pub struct ConfigPathArgs {
    /// Scope whose config file is printed (required; there is no default).
    #[arg(long, value_enum, required = true)]
    pub scope: ConfigScope,
}

/// Arguments for `config get`.
#[derive(Args, Debug)]
pub struct ConfigGetArgs {
    /// Dotted key, for example `fuzzy.max_content_bytes`.
    pub key: String,
}

/// Arguments for `config set`.
#[derive(Args, Debug)]
pub struct ConfigSetArgs {
    /// Dotted key, for example `fuzzy.max_content_bytes`.
    pub key: String,
    /// New value; its type and range come from the key.
    ///
    /// The attribute stays: a stolen flag lands here as a value the key's type
    /// rejects, so the write is refused instead of storing the wrong value.
    #[arg(allow_hyphen_values = true)]
    pub value: String,
    /// Scope whose config file is written (required; there is no default).
    #[arg(long, value_enum, required = true)]
    pub scope: ConfigScope,
    /// Report the planned write and leave the file untouched.
    #[arg(long, help = "Report the planned change and write nothing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,
}

/// Arguments for `config unset`.
#[derive(Args, Debug)]
pub struct ConfigUnsetArgs {
    /// Dotted key to remove, for example `fuzzy.max_content_bytes`.
    pub key: String,
    /// Scope whose config file is changed (required; there is no default).
    #[arg(long, value_enum, required = true)]
    pub scope: ConfigScope,
    /// Report the planned removal and leave the file untouched.
    #[arg(long, help = "Report the planned change and write nothing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,
}

/// Where the effective value of a key comes from.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Origin {
    /// The built-in value.
    Default,
    /// The global config file.
    Global,
    /// The workspace config file.
    Workspace,
    /// The file named by `--config`.
    Explicit,
}

impl Origin {
    fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Global => "global",
            Self::Workspace => "workspace",
            Self::Explicit => "explicit",
        }
    }
}

impl From<ConfigScope> for Origin {
    fn from(scope: ConfigScope) -> Self {
        match scope {
            ConfigScope::Global => Self::Global,
            ConfigScope::Workspace => Self::Workspace,
        }
    }
}

/// One key as `config list` and `config get` report it.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct KeyEntry {
    /// Dotted key.
    pub key: String,
    /// Effective value; absent when the key has no value at all.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    /// `default`, `global`, `workspace` or `explicit` (`--config`).
    pub origin: &'static str,
    /// Built-in value; absent when the key has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// `integer`, `float`, `boolean` or `string`.
    pub kind: &'static str,
    /// Scopes the key can be written in.
    pub scopes: Vec<&'static str>,
    /// File the effective value was read from; absent for the default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Terminal event of `config`.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ConfigOutput {
    /// Event type discriminator: `"config"`.
    pub r#type: &'static str,
    /// Subcommand name.
    pub command: &'static str,
    /// `path`, `list`, `get`, `set` or `unset`.
    pub action: &'static str,
    /// Scope written or printed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<&'static str>,
    /// Config file acted on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_path: Option<String>,
    /// Whether that file exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exists: Option<bool>,
    /// Dotted key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Effective value (`get`) or the value written (`set`); the effective value
    /// after `unset` depends on the other scopes, so `unset` omits it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    /// Where the value comes from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<&'static str>,
    /// Built-in value of the key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Value the scope file held before `set` or `unset`; absent when it held none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_value: Option<serde_json::Value>,
    /// Whether `set` or `unset` changed the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changed: Option<bool>,
    /// Present and true when `--dry-run` suppressed the write.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,
    /// Every key (`list`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keys: Option<Vec<KeyEntry>>,
    /// `pass` when `doctor` loaded every layer without error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<&'static str>,
    /// Config files `doctor` found, highest priority first.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_files: Option<Vec<String>>,
    /// Problems that did not stop the run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    /// Wall-clock time in milliseconds.
    pub elapsed_ms: u64,
}

impl ConfigOutput {
    fn new(action: &'static str) -> Self {
        Self {
            r#type: "config",
            command: "config",
            action,
            scope: None,
            config_path: None,
            exists: None,
            key: None,
            value: None,
            origin: None,
            default: None,
            old_value: None,
            changed: None,
            dry_run: None,
            keys: None,
            status: None,
            config_files: None,
            warnings: Vec::new(),
            elapsed_ms: 0,
        }
    }
}
