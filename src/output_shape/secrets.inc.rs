// SPDX-License-Identifier: MIT OR Apache-2.0
// Secret needles, key markers and envelope key redaction.
// Included from output_shape.inc.rs.

/// Substrings that mark a string value as a secret (memmem, case-sensitive).
const SECRET_NEEDLES: &[&str] = &[
    "-----BEGIN",
    "Bearer ",
    "bearer ",
    // Cloud and forge issuance prefixes. Each one is long enough to be
    // unambiguous: a short prefix would match ordinary words (`sk-` alone sits
    // inside `task-`), and a needle that over-matches turns an opt-in that is
    // supposed to protect data into a tool that corrupts it.
    "AKIA",            // AWS access key id
    "ASIA",            // AWS temporary access key id
    "ghp_",            // GitHub personal access token
    "gho_",            // GitHub OAuth token
    "ghs_",            // GitHub App server token
    "github_pat_",     // GitHub fine-grained token
    "xoxb-",           // Slack bot token
    "xoxp-",           // Slack user token
    "xoxa-",           // Slack app token
    "sk-live-",        // Stripe live secret key
    "sk_test_",        // Stripe test secret key
    "sk_live_",        // Stripe live secret key (underscore spelling)
    "AIza",            // Google API key
    "ya29.",           // Google OAuth access token
    "sk-ant-api",      // Anthropic API key
];

/// Suffix markers for object keys whose value is always a secret.
const SECRET_KEY_MARKERS: &[&str] = &[
    "token",
    "secret",
    "password",
    "passwd",
    "apikey",
    "api_key",
    "credential",
    "authorization",
];

const REDACTED: &str = "[redacted]";

/// Envelope keys whose value is a secret, matched on the **normalized** name.
///
/// Normalization drops case and the separators `-`, `_`, `.`, `:`, and space,
/// then requires the WHOLE normalized name to equal one of these. Equality,
/// not substring, is the point: a substring rule would redact `token_count`,
/// `secret_count`, or `password_policy`, which are ordinary data keys, and a
/// floor that destroys legitimate payload is worse than no floor because it
/// silently corrupts the agent's input.
///
/// This list is the floor: it applies on **every** emit, with or without
/// `--redact`. The flag adds the content-shaped rules on top; it never turns
/// the floor off. A new data key added by another author is covered the moment
/// its name matches, and nobody has to remember to opt in.
pub const ENVELOPE_SECRET_KEY_NAMES: &[&str] = &[
    "accesstoken",
    "apitoken",
    "apikey",
    "authheader",
    "authorization",
    "authtoken",
    "bearertoken",
    "clientsecret",
    "cookie",
    "credential",
    "credentials",
    "idtoken",
    "password",
    "passwd",
    "privatekey",
    "refreshtoken",
    "secret",
    "secretkey",
    "setcookie",
    "token",
];

/// Fold a JSON key to its comparable form: lowercase, separators removed.
fn normalize_key_name(key: &str) -> String {
    key.chars()
        .filter(|c| !matches!(c, '-' | '_' | '.' | ':' | ' '))
        .flat_map(char::to_lowercase)
        .collect()
}

/// Redact envelope values whose **key name** declares them secret.
///
/// Applied to every emitted envelope, independent of `--redact`. Returns the
/// number of values replaced. Nested objects and arrays are walked, because a
/// secret can sit one level down (`{"error": {"api_token": …}}`).
pub fn scrub_envelope_keys(v: &mut Value) -> usize {
    match v {
        Value::Object(map) => {
            let mut count = 0;
            for (key, val) in map.iter_mut() {
                if ENVELOPE_SECRET_KEY_NAMES.contains(&normalize_key_name(key).as_str()) {
                    *val = Value::String(REDACTED.to_string());
                    count += 1;
                } else {
                    count += scrub_envelope_keys(val);
                }
            }
            count
        }
        Value::Array(items) => items.iter_mut().map(scrub_envelope_keys).sum(),
        _ => 0,
    }
}
