# ctoken

cli utility to count tokens in project files — like `cloc` for lines, but for LLM context estimation. Useful for understanding how much context a project or directory would consume when feeding it to a coding agent.

## Install

### macOS (Apple Silicon)

```bash
brew tap RimantasZ/ctoken
brew install ctoken
```

### Linux (Debian / Ubuntu)

Download the `.deb` from the [latest release](https://github.com/RimantasZ/ctoken/releases/latest):

```bash
curl -LO https://github.com/RimantasZ/ctoken/releases/latest/download/ctoken_amd64.deb
sudo apt install ./ctoken_amd64.deb
```

### Windows

Download `ctoken-x86_64-windows.zip` from the [latest release](https://github.com/RimantasZ/ctoken/releases/latest), extract it, and add the folder to your `PATH`.

### From source (any platform)

Requires [Rust](https://rustup.rs) 1.70+:

```bash
cargo install --git https://github.com/RimantasZ/ctoken
```

Or clone and build locally:

```bash
git clone https://github.com/RimantasZ/ctoken
cd ctoken
cargo build --release
# binary at target/release/ctoken
```

## Quick examples

```bash
# Default: token count by immediate subdirectory
ctoken .

# Group by file extension
ctoken . -t

# Use a built-in language profile
ctoken . -p rust

# Match only markdown files
ctoken . -m '**/*.md'

# Walk recursively, per-directory breakdown
ctoken . --recursive

# Just the total token count
ctoken . -s

# JSON output
ctoken . --json

# Count tokens in a single file
ctoken src/main.rs

# Show each file as it's processed
ctoken . -v
```

## Flags

| Short | Long | Arg | Description |
|---|---|---|---|
| `-h` | `--help` | — | Print help and exit |
| | `--version` | — | Print version and exit |
| `-t` | `--type` | — | Group by file extension instead of by subdirectory |
| `-g` | `--gitignore` | `on`\|`off` | Honor `.gitignore`. Default `on` |
| `-m` | `--match` | `<GLOB>` | Glob pattern restricting included files. Repeatable |
| `-p` | `--profile` | `<NAME>` | Use named profile from `~/.config/ctoken/profiles.toml` |
| | `--recreate-profiles` | — | Rewrite built-in profile entries in `profiles.toml` (interactive) |
| | `--recursive` | — | Walk recursively; per-directory table grouped by file type |
| | `--recursive-with-dir` | — | Same as `--recursive`, but includes child directory rollups |
| `-v` | `--verbose` | — | Log each file processed |
| `-s` | `--sum` | — | Print only the grand total (single integer) |
| | `--json` | — | Emit JSON instead of a table. Incompatible with `--recursive*` |
| | `--encoding` | `<NAME>` | Tiktoken encoding (see below) |

## Encoding options

ctoken uses [tiktoken-rs](https://github.com/zurawiki/tiktoken-rs) to estimate actual tokens in files, 
and supports these encoding used by OpenAI models

| Name | Models |
|---|---|
| `cl100k_base` (default) | gpt-4, gpt-3.5-turbo, text-embedding-ada-002, text-embedding-3-* |
| `o200k_base` | GPT-5 series, o1/o3/o4 series, gpt-4o, gpt-4.5, gpt-4.1, codex-* |
| `p50k_base` | Code models, text-davinci-002, text-davinci-003 |
| `p50k_edit` | Edit models like text-davinci-edit-001, code-davinci-edit-001 |
| `r50k_base` | GPT-3 models like davinci |

*Note:* for different LLM providers, token calculation might skughtly differ. Therefore this tool should be used for rough comparison (e.g. "how much this file/folder is bigger in terms of tokens than that one"), rather than precise estimation.

## Profile system

On first run, `ctoken` creates `~/.config/ctoken/profiles.toml` with built-in profiles for common project types: `java`, `c_cpp`, `typescript`, `python`, `rust`, `go`.

```bash
# Use a profile
ctoken . -p typescript

# Restore built-in profiles to defaults (prompts before changing)
ctoken . --recreate-profiles
```

Edit `~/.config/ctoken/profiles.toml` directly to add custom profiles or tweak existing ones:

```toml
[myproject]
include = ["**/*.rs", "**/*.toml", "docs/**/*.md"]
exclude = ["target/**"]
```

New built-in profiles added in later versions are appended automatically without overwriting your customizations.

## Performance notes

- Uses all CPU cores for tokenization (via rayon).
- Files are read fully into memory. Very large files (50+ MB) will use proportionate RAM.
- Binary files are detected by extension or by scanning the first 8 KB for NUL bytes, and skipped.
- Symlinks are never followed.

## License

Apache-2.0
