# Coldbrew Performance Design (Zerobrew-Inspired)

Status: Draft  
Last updated: January 28, 2026

## Goals
- Document the current coldbrew implementation (as of today) for install/download/index flow.
- Identify proven performance techniques from zerobrew that fit coldbrew's principles.
- Propose a phased plan to adopt them safely.
- Define metrics and tests to validate wins and avoid regressions.

## Non-Goals
- This is not a complete rewrite spec.
- Does not change UX/CLI semantics (install/update/upgrade remain explicit).
- No source builds (still bottles-only).

## Current Implementation Summary (Coldbrew)

### Entry + Commands
- `src/main.rs` parses CLI args and dispatches to command handlers.
- Command handlers live in `src/cli/commands/*` and mostly call `src/ops/*`.

### Install Flow (today)
- Orchestrator: `src/ops/install.rs`.
- High-level steps:
  1) Load formula index from disk using `Index` (`src/registry/index.rs`).
  2) Resolve dependencies (depth-first, serial).
  3) Choose bottle for platform from formula metadata.
  4) Check download cache (`src/storage/cache.rs`).
  5) Download from GHCR (`src/registry/ghcr.rs`).
  6) Verify checksum (`src/ops/verify.rs`).
  7) Extract bottle directly into cellar (`src/storage/cellar.rs`).
  8) Create shims (`src/storage/shim_manager.rs`).
  9) Write package metadata into cellar (`src/core/package.rs` + `src/storage/cellar.rs`).

### Caching + Registry
- Formula index cached as a full JSON file at `~/.coldbrew/index` (no HTTP conditional caching).
- Bottle cache is keyed by `{name, version, tag}` and stored under `~/.coldbrew/cache/downloads`.
- GHCR token is cached per package in memory (no persistent cache).

### Concurrency
- Config has `parallel_downloads`, `parallel_extractions`, `parallel_codesigning`, and `parallel_installs`.
- Download tasks do download -> verify; extraction is bounded separately.
- Codesigning is bounded during install (macOS only).
- Install steps can overlap across packages (bounded by install concurrency).
- Dependency resolution and formula loading are serial.

### Storage Model
- Bottles are extracted directly into cellar (`~/.coldbrew/Cellar/{name}/{version}`).
- No content-addressable store. Reinstall repeats extraction.

## Zerobrew Techniques Worth Adopting

### A) Content-Addressable Store
**Zerobrew**: extract each bottle once into `store/{sha256}` then materialize into cellar using clonefile/hardlink/copy.  
**Benefit**: warm installs are near-instant; avoids repeated extraction.

### B) Fast Materialization (APFS clonefile)
**Zerobrew**: uses `clonefile` on macOS, falls back to hardlink/copy.  
**Benefit**: near-zero disk overhead and fast installs on APFS.

### C) Parallel + Deduped Downloads
**Zerobrew**: bounded concurrency, inflight dedup by sha256, optional racing across CDN edges.  
**Benefit**: faster installs, avoids duplicate work for shared deps.

### D) Streaming Pipeline
**Zerobrew**: downloads stream; as each bottle finishes, extraction/materialize/linking begins.  
**Benefit**: overlaps I/O and CPU, reduces total wall time.

### E) API HTTP Caching (ETag/Last-Modified)
**Zerobrew**: stores ETag/Last-Modified in sqlite and reuses cached body on 304.  
**Benefit**: faster `update` and fewer bytes downloaded.

### F) Parallel Formula Fetching
**Zerobrew**: fetches dependency formula JSON in parallel batches.  
**Benefit**: reduces latency during dependency planning.

### G) Downloader Tuning
**Zerobrew**: HTTP/2, connection pooling, larger windows for throughput.  
**Benefit**: improved download speeds and multiplexing.

## Proposed Additions to Coldbrew

### Phase 0: Baseline + Benchmarks (small)
- Add benchmark harness for installs (cold vs warm) and record top 20/100 formula set.
- Instrument timing for download, verify, extract, link.
- Output to JSON for local comparison.

### Phase 1: Concurrency + HTTP Caching (small/medium)
- Use `parallel_downloads` to download multiple bottles concurrently.
- Deduplicate inflight downloads by sha256.
- Add optional CDN racing behind a flag (e.g., `--race` or config).
- Add conditional HTTP caching for formula index fetch:
  - Store ETag/Last-Modified next to index JSON.
  - On update, send `If-None-Match` / `If-Modified-Since`.
  - On 304, reuse cached body.

### Phase 2: Blob Cache by SHA (medium)
- Cache blobs by sha256 rather than name/version/tag.
- Write to a temp file then atomic rename.
- On checksum mismatch or extraction failure, delete blob and retry.
- Keep the existing name/version cache as a metadata layer, or replace with sha cache.

### Phase 3: Content-Addressable Store (large)
- Add `store/` and `locks/` under `~/.coldbrew`.
- Extract bottle tarball into `store/{sha256}` once.
- Materialize into cellar using clonefile/hardlink/copy.
- Track refcounts to allow `space clean` to delete unreferenced store entries.

### Phase 4: Streaming Install Pipeline (large)
- Download bottles concurrently and process each as it completes:
  - verify -> store extract -> materialize -> link -> metadata
- Keep overall dependency order for correctness but allow independent packages to pipeline.

## Design Constraints (Coldbrew Principles)
- No auto-updates. Index fetch is explicit (`crew update`).
- Install should not implicitly upgrade.
- `coldbrew.lock` must remain authoritative for project installs.
- Bottles only; no source builds.

## Data Model Changes

### Option A: Minimal (JSON + filesystem)
- Store per-blob metadata JSON: `store/{sha256}/metadata.json`.
- Store refcounts in `~/.coldbrew/store/refcounts.json`.
- Pros: minimal dependencies.  
- Cons: more brittle with concurrent installs.

### Option B: sqlite metadata db (recommended)
- Add `~/.coldbrew/db/coldbrew.sqlite3`.
- Tables:
  - `store_entries(sha256, created_at, size_bytes)`
  - `store_refs(sha256, package, version)`
  - `api_cache(url, etag, last_modified, body, cached_at)` (if adopted)
- Pros: safe concurrency, easier GC and audits.  
- Cons: adds sqlite dependency (already in deps? if not, add).

## Locking + Concurrency
- Use per-store-entry lock files (like zerobrew) to prevent double extraction.
- For downloads, use inflight map keyed by sha256 with broadcast to waiters.
- Ensure `space clean` does not race with installs (global GC lock file).

## Error Handling
- On checksum mismatch: delete blob and retry download (up to N times).
- On extraction failure: delete blob, retry (limit), then surface error with suggestion.
- On store materialize failure: fallback to copy.

## Metrics and Success Criteria
- Cold install speedup vs current baseline (target: 1.5x+ median).
- Warm reinstall speedup (target: 3x+ median).
- `update` should be faster on second run (304 reuse).
- No correctness regressions in dependency resolution or linking.

## Risks
- APFS clonefile only on macOS; fallback behavior must be correct.
- Concurrency increases chance of race conditions without robust locks.
- sqlite introduces new failure modes; must have clear recovery steps.
- Streaming install could make error reporting more complex.

## Open Questions
- Do we want CDN racing enabled by default or opt-in?
- Where should store live: `~/.coldbrew/store` or `/opt/coldbrew/store`?
- Should we persist GHCR tokens across runs?
- Is sqlite acceptable as a dependency for coldbrew?

## Immediate Next Steps
1) Decide on data model (JSON vs sqlite).
2) Implement Phase 1 (parallel downloads + HTTP caching) behind config flags.
3) Add benchmark harness and baseline results.
