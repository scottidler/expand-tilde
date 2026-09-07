//! One way to expand a leading `~` in a path.
//!
//! `~` is shell syntax, not something the OS resolves. A config value like
//! `~/.local/bin/claude` handed straight to `Command::new` fails with
//! `NotFound`; handed to a filesystem call it creates a directory literally
//! named `~` in the process's current directory. Both are real bugs that have
//! shipped.
//!
//! This crate exists because eight repos had seven different implementations
//! of this five-line function, and they disagreed on the edge cases. The
//! behavior here is deliberate:
//!
//! - A leading `~` expands, whether bare (`~`) or prefixed (`~/foo`).
//! - `~otheruser` passes through UNCHANGED. Resolving another user's home
//!   means a `/etc/passwd` lookup for a case that never fires in practice.
//! - No home directory found -> the path passes through unchanged, deferring
//!   the failure to whatever tries to use it next. It never silently
//!   substitutes a fallback like `.`, which would write into the current
//!   directory instead of erroring.
//!
//! The matching is done with [`Path::strip_prefix`], which operates on path
//! COMPONENTS. That is what makes `~otheruser` fall through correctly: a
//! naive `strip_prefix("~/")` string match misses a bare `~`, and a
//! `replacen("~", home, 1)` turns `~otheruser` into `<home>otheruser`.

use std::path::{Path, PathBuf};

/// Expand a leading `~` (bare, or followed by `/...`) to the current user's
/// home directory.
///
/// Any other path, including `~user`, passes through unchanged. See the crate
/// docs for why.
///
/// ```
/// use expand_tilde::expand_tilde;
/// assert_eq!(expand_tilde("/usr/bin/claude"), std::path::PathBuf::from("/usr/bin/claude"));
/// ```
pub fn expand_tilde(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    match path.strip_prefix("~") {
        Ok(rest) => match dirs::home_dir() {
            Some(home) => home.join(rest),
            None => path.to_path_buf(),
        },
        Err(_) => path.to_path_buf(),
    }
}

#[cfg(feature = "serde")]
mod serde_support {
    use super::expand_tilde;
    use serde::{Deserialize, Deserializer};
    use std::path::PathBuf;

    /// `#[serde(deserialize_with = "expand_tilde::deserialize_tilde_pathbuf")]`
    ///
    /// Expands at config LOAD time, so every consumer downstream is correct by
    /// construction rather than each call site having to remember. This is the
    /// reason the crate carries a serde feature at all: the function is
    /// trivial, the convention is the valuable part.
    pub fn deserialize_tilde_pathbuf<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = PathBuf::deserialize(deserializer)?;
        Ok(expand_tilde(raw))
    }

    /// [`deserialize_tilde_pathbuf`] for `Option<PathBuf>` fields.
    pub fn deserialize_tilde_pathbuf_opt<'de, D>(
        deserializer: D,
    ) -> Result<Option<PathBuf>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = Option::<PathBuf>::deserialize(deserializer)?;
        Ok(raw.map(expand_tilde))
    }
}

#[cfg(feature = "serde")]
pub use serde_support::{deserialize_tilde_pathbuf, deserialize_tilde_pathbuf_opt};

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> PathBuf {
        dirs::home_dir().expect("test host has a home directory")
    }

    #[test]
    fn bare_tilde_expands_to_home() {
        assert_eq!(expand_tilde("~"), home());
    }

    #[test]
    fn prefixed_tilde_expands() {
        assert_eq!(expand_tilde("~/.local/bin/claude"), home().join(".local/bin/claude"));
    }

    #[test]
    fn absolute_path_passes_through() {
        assert_eq!(expand_tilde("/usr/bin/claude"), PathBuf::from("/usr/bin/claude"));
    }

    #[test]
    fn relative_path_passes_through() {
        assert_eq!(expand_tilde("relative/path"), PathBuf::from("relative/path"));
    }

    #[test]
    fn bare_name_passes_through() {
        assert_eq!(expand_tilde("claude"), PathBuf::from("claude"));
    }

    #[test]
    fn other_users_home_passes_through_unchanged() {
        // The case every string-matching implementation got wrong.
        // `replacen("~", home, 1)` would produce `<home>otheruser/bin`.
        assert_eq!(
            expand_tilde("~otheruser/bin/claude"),
            PathBuf::from("~otheruser/bin/claude")
        );
    }

    #[test]
    fn tilde_not_in_leading_position_passes_through() {
        assert_eq!(expand_tilde("/opt/~/weird"), PathBuf::from("/opt/~/weird"));
    }

    #[test]
    fn expanding_twice_is_idempotent() {
        let once = expand_tilde("~/.config");
        assert_eq!(expand_tilde(&once), once);
    }
}
