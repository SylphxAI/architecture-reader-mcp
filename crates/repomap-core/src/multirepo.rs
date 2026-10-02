//! Team: one graph across several repository roots (T2).
//!
//! This is the hook the binary calls once a request names more than one root
//! and a Team licence is present. The join itself (package identity across
//! npm, Cargo, Go and Python manifests) lands here; until then the hook answers
//! [`NotJoined`] and the binary falls back to the current root.

use serde_json::Value;
use std::fmt;
use std::path::PathBuf;

/// A tool answer over the joined graph, in the same shape as a single-root answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Joined {
    pub text: String,
    pub json: Value,
}

/// The roots could not be joined (not implemented yet, or a root failed to index).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotJoined;

impl fmt::Display for NotJoined {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("not yet joined")
    }
}

impl std::error::Error for NotJoined {}

/// Answer `tool` (canonical name: `map`, `search`, `context`, `trace`, `impact`,
/// `db`) with `args` over the graph joined from `roots`. `roots` holds every
/// root named by the request, current root first, already resolved.
///
/// Stub: always [`NotJoined`].
pub fn join_workspace(roots: &[PathBuf], tool: &str, args: &Value) -> Result<Joined, NotJoined> {
    let _ = (roots, tool, args);
    Err(NotJoined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_is_not_joined() {
        let r = join_workspace(
            &[PathBuf::from("a"), PathBuf::from("b")],
            "impact",
            &Value::Null,
        );
        assert_eq!(r.unwrap_err().to_string(), "not yet joined");
    }
}
