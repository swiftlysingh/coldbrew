# Coldbrew Core Principles and Homebrew Integration

Status: Draft  
Last updated: January 28, 2026

## Purpose
This document defines product principles, the integration contract with Homebrew, and the
engineering guidelines for building and evolving the Coldbrew CLI.

## Core Principles and What They Mean
- User is always in control: no auto-updates, no forced upgrades, no sudo.
- Explicit over implicit: each command does exactly what it says and nothing more.
- Fast by default: bottles-only, parallelism where safe, and aggressive caching.
- Cross-platform first-class: macOS and Linux are equal citizens with consistent behavior.
- Reproducible via lockfiles: `coldbrew.lock` is authoritative for project installs.

## Homebrew Integration Contract
- Formula metadata: Homebrew JSON API.
  - Index: `https://formulae.brew.sh/api/formula.json`
  - Formula: `https://formulae.brew.sh/api/formula/{name}.json`
- Bottles: prebuilt binaries from GitHub Container Registry (ghcr.io).
- Taps: support existing taps via git clone and use their metadata.
- Version file detection: .nvmrc, .python-version, .ruby-version, .node-version, package.json engines.
- Licensing: Homebrew formula data is BSD-licensed, compatible with reuse.

## Homebrew Frictions and Coldbrew Fixes

### Auto-Updates and Forced Upgrades
**Problem:** Homebrew can update or upgrade implicitly.  
**Coldbrew behavior:**
- `coldbrew install <pkg>` only installs the requested package.
- `coldbrew update` only refreshes the local index.
- `coldbrew upgrade` is interactive and shows a plan before applying.
- `coldbrew upgrade --yes` is for CI only.
- No auto-updates, ever.

### Version Pinning and Multiple Versions
**Problem:** Global upgrades and single-version installs complicate reproducibility.  
**Coldbrew behavior:**
- Multiple versions can coexist: `coldbrew install node@18 node@22`.
- `coldbrew.toml` defines per-project dependencies.
- Auto-detect version files (nvm, python, node, etc.) to reduce friction.
- `coldbrew.lock` pins exact versions and checksums.
- Priority: coldbrew.toml > detected files > global default.

### Install Location and System Conflicts
**Problem:** Global installs and sudo can conflict with system packages.  
**Coldbrew behavior:**
- Install to user space: `~/.coldbrew/`.
- No sudo required; remove all with `rm -rf ~/.coldbrew`.
- Custom root via `COLDBREW_HOME`.

### Dependency Cascades
**Problem:** Upgrading one package can cascade through the graph.  
**Coldbrew behavior:**
- Upgrade only what the user asks for.
- Dependencies stay pinned unless explicitly upgraded.
- Warn before cascades and show impact.
- Opt-in cascade: `coldbrew upgrade --cascade`.
- Tools: `coldbrew deps <pkg>`, `coldbrew dependents <pkg>`.

### Cleanup and Garbage Collection
**Problem:** Auto-cleanups can delete needed versions or caches.  
**Coldbrew behavior:**
- `coldbrew gc` is interactive by default.
- `coldbrew gc --yes` for non-interactive environments.
- Cache cleanup is separate: `coldbrew cache clean`.
- No auto-removal in the background.

### Performance
**Problem:** Slow installs and repeated extraction.  
**Coldbrew behavior:**
- Single static Rust binary.
- Parallel downloads where safe.
- Cached index and bottles.
- Bottles-first always; source builds are opt-in only.

### Cross-Platform
**Problem:** Tools behave differently by OS and architecture.  
**Coldbrew behavior:**
- First-class support: macOS (arm64, x86_64), Linux (x86_64, arm64), WSL.
- Auto-detect OS, arch, and libc for bottle selection.
- Transparent errors when a bottle is unavailable.

## CLI Design and Development Guidelines

### Command Design Rules
- `install` installs only; `update` updates only; `upgrade` upgrades only.
- All commands must have `--help` and be self-documenting.
- Provide explicit dry-run modes for destructive actions where possible.
- Quiet and verbose flags should apply consistently across commands.

### Output and UX Rules
- Human-first output by default, machine-friendly option where needed.
- Always show actionable errors and a suggestion when possible.
- For interactive commands, show the plan before applying changes.

### Version Resolution Order
- `coldbrew.toml` project dependencies.
- Detected version files (nvm, python, node, etc.).
- Global defaults (`coldbrew default <pkg>@<ver>`).
- Latest installed version (last resort).

### Bottle Selection Policy
- Prefer exact OS and arch match.
- Fallback chain: exact OS -> previous OS -> generic.
- If no bottle fits, fail gracefully with a clear message.

### Formula Complexity Policy
- MVP: simple packages only (no post-install hooks).
- For complex formulas, show warnings and manual steps.
- Track complexity in local metadata for transparency.

### Shim Management
- Use shims to select versions per project.
- `coldbrew` should act as a shim when invoked by package name.
- Resolution order matches the version resolution rules above.

### Dependency Isolation
- Each package version has its own dependency tree.
- Do not globally link libraries.
- Use DYLD_LIBRARY_PATH / LD_LIBRARY_PATH for runtime resolution.
- Store dependency maps in per-package metadata.

## Engineering Workflow

### Design First
- For new features, start with a short design note in `docs/design/`.
- Define user-visible behavior and edge cases before code.

### Testing and Quality
- New features require tests for success, failure, and edge cases.
- Run `cargo test` and `cargo clippy` before merging.
- Include benchmark updates when touching performance-sensitive code.

### Implementation Boundaries
- `src/cli` should be thin and focused on argument parsing and output.
- `src/ops` owns orchestration and high-level workflows.
- `src/registry` owns network and API integration.
- `src/storage` owns filesystem layout and cache behavior.
- `src/core` owns data structures and resolution logic.

## Storage Layout (Conceptual)
```
~/.coldbrew/
├── bin/           # Shims / active versions
├── cellar/        # Installed packages by version
├── cache/         # Downloaded bottles and indices
├── taps/          # Cloned tap repos
└── config.toml    # Global config
```

## Open Questions
- Should CDN racing be opt-in or default?
- Do we want persistent GHCR token caching?
- Do we adopt sqlite for metadata and API caching?
