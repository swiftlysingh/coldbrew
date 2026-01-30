# Coldbrew Performance Plan (SQLite + Zerobrew Techniques)

Status: Draft

## Decisions
- Use rusqlite 0.37.0 with bundled SQLite (libsqlite3-sys 0.35.x).
- Keep the local formula index (skip parallel formula fetching).
- CDN racing remains opt-in (default off).

## Out of Scope (Tracked Separately)
- Store pruning as a `crew clean` category is tracked in https://github.com/swiftlysingh/coldbrew/issues/36.

## Phase 1: DB Foundation + HTTP Cache + Parallel Downloads
1) Add rusqlite dependency and db paths (`~/.coldbrew/db/coldbrew.sqlite3`).
2) Implement DB init + migrations (WAL, foreign keys).
3) Add `api_cache` table and wire conditional GET for the index (ETag/Last-Modified).
4) Use `settings.parallel_downloads` with bounded concurrency and inflight dedupe by sha256.
5) Add optional CDN racing flag (off by default).

## Phase 2: SHA Blob Cache
1) Cache bottles by sha256 with atomic temp-write + rename.
2) Retry on checksum mismatch or extraction failure (bounded).
3) Optional `blob_cache` metadata table for size/created_at.

## Phase 3: Content-Addressable Store + Fast Materialization
1) Add `store/` and `locks/` under `~/.coldbrew`.
2) Extract each bottle once into `store/{sha256}`.
3) Materialize to cellar via clonefile on macOS, fallback to hardlink/copy.
4) Store metadata tables (`store_entries`, `store_refs`) for future cleanup support.

## Phase 4: Streaming Install Pipeline
1) Pipeline per package: download -> verify -> store extract -> materialize -> link.
2) Respect dependency order while allowing independent packages to overlap.
3) Tighten download UX: aggregate progress for parallel downloads, minimize noisy per-file bars.

## Phase 5: Downloader Tuning
1) Enable HTTP/2 and connection pooling.
2) Tune flow windows and concurrency defaults based on metrics.

## Cross-Cutting Requirements
- Locking model for concurrent installs (per-store lock files).
- Error handling: bounded retries with actionable suggestions.
- Metrics hooks for step timings + bytes downloaded.
