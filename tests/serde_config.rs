//! The serde wrappers are the reason this crate exists rather than everyone
//! copying five lines. These prove a config field expands at LOAD time.

use expand_tilde::{deserialize_tilde_pathbuf, deserialize_tilde_pathbuf_opt};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Config {
    #[serde(deserialize_with = "deserialize_tilde_pathbuf")]
    creds_path: PathBuf,
    #[serde(default, deserialize_with = "deserialize_tilde_pathbuf_opt")]
    claude_binary: Option<PathBuf>,
    #[serde(default, deserialize_with = "deserialize_tilde_pathbuf_opt")]
    voice_profile: Option<PathBuf>,
}

fn home() -> PathBuf {
    dirs::home_dir().expect("home")
}

#[test]
fn config_paths_expand_at_load() {
    let yaml = r#"
creds-path: ~/.config/app/creds.json
claude-binary: ~/.local/bin/claude
voice-profile: ~/Claude/writing/VOICE.md
"#;
    let cfg: Config = serde_yaml::from_str(yaml).expect("parse");
    assert_eq!(cfg.creds_path, home().join(".config/app/creds.json"));
    assert_eq!(cfg.claude_binary, Some(home().join(".local/bin/claude")));
    assert_eq!(cfg.voice_profile, Some(home().join("Claude/writing/VOICE.md")));
}

#[test]
fn absent_optional_stays_none() {
    let yaml = "creds-path: /etc/app/creds.json\n";
    let cfg: Config = serde_yaml::from_str(yaml).expect("parse");
    assert_eq!(cfg.creds_path, PathBuf::from("/etc/app/creds.json"));
    assert_eq!(cfg.claude_binary, None);
    assert_eq!(cfg.voice_profile, None);
}
