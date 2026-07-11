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

## Cursor Cloud specific instructions

This repo has two implementations: the Rust `crew` binary (primary, at repo root) and the
in-progress Swift port (`swift/`, tracked by `docs/swift-migration-plan.md`). Both toolchains
are set up in the cloud VM.

### Toolchains
- **Rust** `1.93.0`, pinned by `rust-toolchain.toml`. Standard commands are in the "Build & Test"
  section above (`cargo build|test|clippy|fmt`).
- **Swift** `6.3.3`, installed via `swiftly` at `~/.local/share/swiftly`. `~/.bashrc` sources
  `~/.local/share/swiftly/env.sh`, so interactive shells get `swift` on `PATH`. Non-interactive
  shells may not source `~/.bashrc`; if `swift: command not found`, first run
  `. ~/.local/share/swiftly/env.sh`. Swift commands: `swift build|test|run --package-path swift`
  (matches `.github/workflows/ci.yml`), or `cd swift` first.

### Non-obvious caveats
- The Rust build links OpenSSL via `git2` → `openssl-sys`; `libssl-dev` + `pkg-config` must be
  present (already installed in the VM). Without them, `cargo build` fails at `openssl-sys`.
- **Linux bottle execution limitation:** the full `crew install` pipeline works end-to-end on
  Linux (index fetch, dependency resolution, bottle download, extraction, linking, shim creation,
  SQLite tracking — verify with `crew list` / `crew which`). However, Homebrew's Linux bottles
  ship binaries whose ELF interpreter is the unrelocated placeholder `@@HOMEBREW_PREFIX@@/lib/ld.so`,
  and coldbrew does not rewrite the ELF interpreter on Linux. So an installed foreign bottle binary
  (e.g. `jq`) cannot actually be *executed* on the VM, even though the install itself succeeds.
  Non-execution flows (`update`, `search`, `info`, `install`, `list`, `which`, `doctor`, `init`,
  `lock`) all work. Treat this as an app-level limitation, not an environment problem.
- Shims (`~/.coldbrew/bin/<tool>`) are `sh` scripts that `exec crew exec ...`, so the `crew`
  binary must be on `PATH` for a shimmed tool to resolve.
