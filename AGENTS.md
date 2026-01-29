# AGENTS.md

A Homebrew-compatible package manager in Rust. Downloads bottles from Homebrew's infrastructure, manages multiple versions, supports project-level lockfiles.

## Core Principles

- **User is always in control**: No auto-updates, no surprises, no sudo
- **Explicit over implicit**: `install` just installs, `update` just refreshes index, `upgrade` is interactive
- **Bottles first**: Always use prebuilt binaries (source build is `--build-from-source`, not implemented yet)
- **Reproducible**: `coldbrew.lock` pins exact versions and checksums

## Discovering Commands

**Use `--help` to discover commands and flags.** The CLI is self-documenting:

```bash
crew --help              # List all commands
crew install --help      # Show flags for install
crew cache --help        # Show cache subcommands
```

Do not memorize commands. Always check `--help` for the current interface.

## Build & Test

```bash
cargo build                  # Debug build
cargo build --release        # Release build
cargo test                   # Run all tests
cargo clippy                 # Lint
cargo fmt                    # Format
```

Always run `cargo test` and `cargo clippy` before committing.

## Project Layout

```
src/cli/commands/    # One file per command (install.rs, search.rs, etc.)
src/core/            # Data structures (Formula, Package, Version, Platform)
src/registry/        # Homebrew API client, GHCR downloads, tap management
src/storage/         # Paths, cellar, cache, shims
src/config/          # Global config, project config, lockfiles
src/ops/             # Install/uninstall/upgrade orchestration
src/error.rs         # All error types with user-facing suggestions
```

## Adding a New Command

1. Create `src/cli/commands/{name}.rs` with `pub async fn execute(...) -> Result<()>`
2. Add `pub mod {name};` to `src/cli/commands/mod.rs`
3. Add variant to `Commands` enum in `src/cli/mod.rs`
4. Handle in `match cli.command` block in `src/main.rs`

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `COLDBREW_HOME` | Override default `~/.coldbrew` location |

## Key APIs

| Endpoint | Purpose |
|----------|---------|
| `https://formulae.brew.sh/api/formula.json` | Full formula index (~15MB) |
| `https://formulae.brew.sh/api/formula/{name}.json` | Single formula |
| `https://ghcr.io/token?scope=repository:homebrew/core/{pkg}:pull` | Auth token for bottle downloads |

## Error Handling

All errors use `ColdbrewError` from `src/error.rs`. Each variant should implement `suggestion()` to help users fix the issue.

## GitHub Issues

- **#1-#8**: MVP phases (completed)
- **#9-#18**: Post-MVP roadmap (source builds, casks, parallel install)
