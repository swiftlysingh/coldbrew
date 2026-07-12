# Coldbrew Swift Migration Plan

Status: Draft
Last updated: July 7, 2026

## Purpose
Port Coldbrew from Rust to Swift to grow contributor engagement from the Swift/macOS
developer community — the same population that uses Homebrew. This document defines the
strategy, the module port order, the dependency mapping, and the parity gates that must
pass before cutover.

## Goals
- Behavior-identical `crew` binary in Swift: same commands, same flags, same output contract.
- Same on-disk layout: `~/.coldbrew/` (store, cellar, shims, SQLite db, config) is the
  compatibility contract. A user must be able to swap the Rust binary for the Swift binary
  with no reinstall and no migration step.
- Pay off the integration-test debt as part of the port, not after it.
- Keep Linux working via the Swift Static Linux SDK (musl), tested in CI.

## Non-Goals
- No new features during the port. The Rust tree is feature-frozen except for bug fixes.
- No redesign of the store/cellar/shim model, DB schema, or CLI surface. Ideas discovered
  during the port get filed as issues for after cutover.
- No incremental hybrid binary (Rust core with Swift shell). Clean port, phased by module.

## Strategy: the Rust Binary Is the Executable Spec
The port is driven by a language-agnostic, CLI-level integration harness built **first**
and run against the **Rust** binary to lock in current behavior. Each Swift phase is done
when the Swift binary passes the same suite. This converts the Rust implementation from
dead weight into the specification, and fixes the current gap (integration tests are a
placeholder) as a side effect.

### Harness design (Phase 0)
- Lives in `harness/` at the repo root; owned by neither implementation.
- A local fixture registry: static file server emulating the Homebrew formula API
  (`formula.json`, `formula/{name}.json`) plus a minimal ghcr-compatible endpoint serving
  tiny purpose-built bottles (real tar.gz with real Mach-O/ELF binaries, keg-relative
  paths, and `@@HOMEBREW_PREFIX@@` placeholders so relocation is actually exercised).
- Tests are scenario scripts asserting on: exit codes, stdout/stderr (normalized), and
  the resulting filesystem state under a temp `COLDBREW_HOME` (store entries, hard-link
  counts, shim contents, cellar layout, DB rows, lockfile contents).
- Binary under test selected by env var (`CREW_BIN`). CI runs the suite against Rust on
  every PR from day one; the Swift job is added in Phase 1 and allowed to fail until its
  phase's scenarios are in scope.
- Cross-binary state test: install with Rust binary, then `list`/`upgrade`/`uninstall`/
  `space clean` with Swift binary (and the reverse). This enforces the on-disk contract.

Scenario coverage, in priority order: install (single, with deps, multi-version, `--force`,
skip-deps), uninstall (with dependents warning), upgrade (plan display, `--yes`), list,
which, pin/default, init/lock/install-from-lockfile, update, search/info, tap, space +
space clean (orphaned store pruning, `--dry-run`), doctor, shim resolution against
`coldbrew.toml` and version files (.nvmrc, .python-version, etc.).

## Package Structure
SwiftPM package in `swift/` during migration (monorepo, so the harness can test both
binaries from one checkout); moves to repo root at cutover.

```
swift/
  Package.swift
  Sources/
    ColdbrewKit/        # one library target, folders mirror Rust modules
      Core/             # formula, version, dependency, bottle, package, platform
      Config/           # global, project, lockfile, version_files
      Storage/          # paths, db, cache, store, cellar, shim
      Registry/         # homebrew_api, index, ghcr, tap
      Ops/              # install, uninstall, upgrade, link, relocate, verify, cleanup
    crew/               # executable: CLI parsing, commands, output
  Tests/
    ColdbrewKitTests/   # ported unit tests (Swift Testing)
```

One library target to start; split into per-module targets only if build times or
dependency hygiene demand it. Swift 6 language mode, strict concurrency from day one —
retrofitting Sendable later is worse than starting with it.

## Dependency Mapping

| Rust crate | Swift replacement | Notes |
|---|---|---|
| clap, clap_complete | swift-argument-parser | Completions built in. Match help text in harness loosely, not byte-for-byte. |
| tokio | Swift structured concurrency | Stage semaphores → `AsyncSemaphore` + `withThrowingTaskGroup`. CDN racing → first-wins task group. |
| reqwest | async-http-client | Prefer over URLSession for identical macOS/Linux behavior, streaming downloads, and connection pooling. |
| serde / serde_json | Codable | Formula JSON decoding gets explicit `CodingKeys`; keep field tolerance (unknown keys ignored). |
| toml | TOMLKit | Evaluate first; fallback is a small hand-rolled parser (our TOML usage is flat tables only). |
| git2 | shell out to `git` | Taps only need clone/pull. Drops libgit2/openssl linkage entirely; `doctor` checks git presence. |
| indicatif / console / dialoguer | hand-rolled `Output` module | Port `cli/output.rs` semantics: multi-bar progress, byte/duration formatting, prompts. Small, and taste matters here. |
| thiserror / anyhow | `ColdbrewError` enum + typed throws | Mirror `error.rs` variants 1:1 so messages stay identical for the harness. |
| chrono | Foundation `Date` | ISO8601 formatting for DB timestamps must match existing rows. |
| semver + core/version.rs | port version.rs directly | Homebrew versions are not semver; our comparison logic is the spec. Do not adopt a semver package. |
| sha2 / hex | swift-crypto | SHA256 streaming during download, as today. |
| flate2 / tar | shell out to system `tar` (bsdtar/GNU tar) | Extraction is already staged behind a semaphore; process overhead is noise vs I/O. Revisit libarchive bindings only if the deferred-hard-link behavior can't be reproduced via `tar` flags + post-pass. |
| walkdir | `FileManager.enumerator` | Wrap in one helper to centralize symlink/permission handling. |
| directories | `Paths` port | `paths.rs` already owns all layout logic; port it, keep `COLDBREW_HOME`. |
| tracing / tracing-subscriber | swift-log | Same env-filter behavior via `LOG_LEVEL`/`RUST_LOG`-equivalent (`CREW_LOG`). |
| rusqlite (bundled) | SQLite.swift or raw sqlite3 | System libsqlite3 on macOS; link libsqlite3 in the static Linux SDK. Schema unchanged — the DB file is shared state with the Rust binary. |
| nix / libc | Darwin/Glibc + POSIX | Hard links, file modes, `flock` for the shim lock, mach-o writability bit flips before codesign. |
| wiremock / assert_cmd / insta / predicates | harness + Swift Testing | Snapshot-style assertions live in the harness, not in unit tests. |

Shell-outs already in the Rust version (codesign, and whatever `platform.rs`/`main.rs`
invoke) stay shell-outs.

## Port Order and Phase Gates
Each phase gate = ported unit tests pass **and** the harness scenarios that only depend on
completed phases pass with `CREW_BIN` pointing at the Swift binary.

- **Phase 0 — Harness.** Build fixture registry + scenario suite; green against Rust
  binary in CI. This phase is pure value even if the port stalls.
- **Phase 1 — Scaffolding + Core.** SwiftPM package, CI job (macOS + Linux), `Core/`
  ported with its unit tests (version comparison and dependency resolution are the
  highest-risk logic — port their tests first, then the code).
- **Phase 2 — Config.** global/project/lockfile/version_files. Gate: `init`, `lock`
  scenarios.
- **Phase 3 — Storage.** paths, db, store, cache, cellar, shim. Gate: cross-binary state
  scenarios read/write correctly; shim resolution scenarios pass.
- **Phase 4 — Registry.** homebrew_api, index, ghcr (streaming download, CDN racing),
  tap. Gate: `update`, `search`, `info`, `tap` scenarios against fixture registry.
- **Phase 5 — Ops.** verify → relocate → link → uninstall → cleanup → upgrade → install
  (last: it composes everything; port the staged pipeline with TaskGroup + semaphores and
  keep the per-stage timing metrics). Gate: full install/uninstall/upgrade/clean scenarios.
- **Phase 6 — CLI.** Commands, output, doctor, completions, man page regen. Gate: entire
  harness green on Swift binary, macOS and Linux.
- **Phase 7 — Parity + performance.** Benchmark Swift vs Rust on the fixture registry and
  on real installs (small: `jq`; large graph: `ffmpeg`; cold and warm cache). Acceptance:
  within 10% wall-clock of Rust on install; startup latency under 50ms (shims execute on
  every binary invocation — this is the one place Swift startup cost could actually hurt;
  measure early in Phase 3, not here).
- **Phase 8 — Cutover.** Swift package moves to repo root; Rust tree archived on
  `rust-legacy` branch (kept installable); release workflow builds macOS arm64/x86_64 +
  Linux x86_64/aarch64 (static SDK); README, docs, and install instructions updated;
  final release from Rust tree tagged as the last Rust version.

## Risks and Mitigations
- **Shim startup latency.** Shims run on every user command. Measure a hello-world
  swift-argument-parser binary in Phase 1; if >50ms, the shim path gets a minimal
  fast-path entry point that skips full CLI parsing.
- **Linux regression.** Static Linux SDK is the plan, but Foundation behavior differences
  are real. Linux harness job is required-green from Phase 1, not a cutover checkbox.
- **tar semantics.** Deferred hard links during extraction was a deliberate perf fix.
  Reproduce it explicitly (extract without links, post-pass linking) rather than trusting
  tar flag behavior across bsdtar/GNU tar; harness asserts on link counts.
- **Output drift breaking the harness.** Normalize aggressively (strip ANSI, timings,
  progress frames) so assertions target contract, not cosmetics.
- **Contributor gap during migration.** Monorepo keeps main releasable from the Rust tree
  the whole time; label Swift phases as `good-first-port` issues — the port itself is the
  community onboarding funnel.

## Working Agreements
- Feature freeze on `src/` (Rust) except bug fixes; any bug fixed in Rust gets a harness
  scenario before the fix, so the Swift port inherits it.
- Port module-by-module with the Rust file open beside the Swift file; keep names and
  structure recognizably parallel until cutover, then idiomatic cleanup passes are fair game.
- Unit tests are ported (not rewritten from scratch) so behavioral intent carries over.

## PR Execution Plan

All migration PRs target a long-lived `swift-main` branch. `main` stays Rust-stable until
the final cutover PR. Work happens on `codex/swift-*` branches, preferably in separate
worktrees so parallel agents do not overwrite each other.

### Wave 0 — Foundation

1. **PR 001: Swift Main Bootstrap** (`codex/swift-bootstrap`)
   - Add `swift/Package.swift`, `ColdbrewKit`, and an empty Swift `crew`.
   - Add the first Swift build check.
   - Acceptance: Swift package builds on macOS.
2. **PR 002: Executable-Spec Harness Skeleton** (`codex/swift-harness-skeleton`)
   - `CREW_BIN` selects the binary under test.
   - Each scenario gets an isolated temp `COLDBREW_HOME`.
   - Normalize ANSI, timings, and progress noise.
   - Scenarios: `--help`, no command, invalid command, `init`, empty `list`.
   - Acceptance: green against Rust.
3. **PR 003: CLI Contract Matrix** (`codex/swift-cli-contract`)
   - Cover global `--quiet`/`--verbose`, every public command, and hidden `exec`.
   - Acceptance: every current command/flag is covered or explicitly deferred.
4. **PR 004: CI Matrix for Migration** (`codex/swift-ci-matrix`)
   - Keep Rust fmt/clippy/test required.
   - Add required harness-against-Rust check.
   - Add Swift macOS build; add Linux Swift when available.
   - Harness-against-Swift may be allowed to fail until relevant phases land.

### Wave 1 — Parallel Swift Modules

These can run in parallel after PR 001 lands:

5. **PR 005: Swift Core** (`codex/swift-core`) ports formula, bottle, dependency,
   package, platform, and version logic with unit tests.
6. **PR 006: Swift Config** (`codex/swift-config`) ports global/project config,
   lockfile, and version-file detection.
7. **PR 007: Swift Storage Paths + DB** (`codex/swift-storage-db`) preserves
   `COLDBREW_HOME`, DB path, `user_version = 3`, WAL settings, and schema.
8. **PR 008: Swift CLI Surface** (`codex/swift-cli-surface`) matches command and flag
   names with placeholder command bodies where needed.
9. **PR 009: Swift Output + Errors** (`codex/swift-output-errors`) ports user-facing
   output helpers and error suggestions.
10. **PR 010: Swift Registry Lite** (`codex/swift-registry-lite`) ports fixture-backed
    formula index and single-formula lookup before real network behavior.

### Wave 2 — Parallel Harness Deepening

These can run alongside Wave 1:

11. **PR 011: Fixture Registry** (`codex/swift-fixture-registry`) adds local formula API
    fixtures and tiny bottles for `hello`, `dep`, `uses-dep`, and a multi-version package.
12. **PR 012: Install/List/Uninstall Harness** (`codex/swift-install-harness`) covers
    single install, dependency install, `--force`, `--skip-deps`, `list`, `which`,
    uninstall, and uninstall `--with-deps`.
13. **PR 013: Shim + Hidden Exec Harness** (`codex/swift-exec-harness`) covers shim
    contents, `crew exec`, version resolution order, and dependency library paths.
14. **PR 014: DB + Cross-Binary Harness** (`codex/swift-cross-binary-harness`) covers
    Rust→Swift and Swift→Rust shared state plus schema fingerprinting.

### Wave 3 — Composition

15. **PR 015: Storage Store/Cache/Cellar/Shims** depends on PR 005, 006, 007, and 013.
16. **PR 016: Download + Verify** depends on PR 010, 011, and 015.
17. **PR 017: Extract + Relocate** depends on PR 016.
18. **PR 018: Link/Unlink/Pin/Default/Which** depends on PR 015.
19. **PR 019: Install** depends on PR 016, 017, and 018; port this last because it
    composes the whole pipeline.
20. **PR 020: Uninstall/Cleanup/Space** depends on PR 019.
21. **PR 021: Upgrade** depends on PR 019 and 020.
22. **PR 022: Tap/Doctor/Completions** depends on PR 008 and 010.

### Wave 4 — Finalization

23. **PR 023: Full macOS Parity** requires the full harness green against Swift and real
    `jq` then `ffmpeg` smoke installs.
24. **PR 024: Linux Static SDK** makes Swift Linux CI required where fixture support exists.
25. **PR 025: Performance + Startup Gate** checks install time and shim startup latency.
26. **PR 026: Cutover Prep** moves Swift to repo root, updates docs and release workflow,
    and prepares the `rust-legacy` archive branch.
27. **PR 027: Merge `swift-main` to `main`** contains no new implementation work.

### Parallel Agent Ownership

- Agent A: PR 001 and 004 bootstrap/CI.
- Agent B: PR 002 and 003 harness core and CLI matrix.
- Agent C: PR 011 and 012 fixture/install harness.
- Agent D: PR 013 and 014 exec/compat harness.
- Agent E: PR 005 core models.
- Agent F: PR 006 config.
- Agent G: PR 007 storage DB.
- Agent H: PR 008 CLI surface.
- Agent I: PR 009 output/errors.
- Agent J: PR 010 registry.
- Agent K: integration keeper for rebases, conflicts, dependency tracking, and parity status.
