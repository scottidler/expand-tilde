# expand-tilde

One way to expand a leading `~` in a path.

`~` is shell syntax, not something the OS resolves. `~/.local/bin/claude`
handed to `Command::new` fails `NotFound`; handed to a filesystem call it
creates a directory literally named `~` in the process CWD. Both have shipped.

This exists because eight repos carried seven different implementations that
disagreed on the edge cases — including one that turned `~otheruser` into
`<home>otheruser`.

## Behavior

| input | output |
|---|---|
| `~` | `$HOME` |
| `~/foo` | `$HOME/foo` |
| `~otheruser/foo` | unchanged |
| `/abs/path`, `relative/path` | unchanged |
| no home directory found | unchanged, failure deferred to the caller |

Matching uses `Path::strip_prefix`, which works on path COMPONENTS. That is
what makes `~otheruser` fall through correctly.

It never substitutes a fallback like `.` when `$HOME` is missing — that turns
a loud failure into a silent write to the wrong directory.

## Use

```toml
expand-tilde = { git = "https://github.com/scottidler/expand-tilde", tag = "v0.1.0", version = "0.1.0" }
```

```rust
use expand_tilde::expand_tilde;
let bin = expand_tilde("~/.local/bin/claude");
```

### Config fields (the point of the crate)

Enable the `serde` feature and expand at LOAD time, so every consumer
downstream is correct by construction instead of each call site remembering:

```rust
#[derive(Deserialize)]
struct Config {
    #[serde(deserialize_with = "expand_tilde::deserialize_tilde_pathbuf")]
    creds_path: PathBuf,

    #[serde(default, deserialize_with = "expand_tilde::deserialize_tilde_pathbuf_opt")]
    claude_binary: Option<PathBuf>,
}
```
