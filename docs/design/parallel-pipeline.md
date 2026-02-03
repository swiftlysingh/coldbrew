# Parallel Install Pipeline (Multi-Stage Semaphores)

Status: Draft  
Last updated: February 3, 2026  
Audience: Intermediate contributors

## Why this doc
We want installs to be faster without surprising users or risking corrupt state. This note explains
how we plan to add multi-stage parallelism (download, extract, codesign) and how that differs from
the current implementation.

## Quick glossary
- Bottle: prebuilt binary archive from Homebrew.
- Store: content-addressable extraction area (by sha256).
- Materialize: copy/clone files from store into the cellar.
- Relocate: rewrite paths inside Mach-O files (macOS only).
- Codesign: apply a signature to Mach-O files (macOS only).

## Current Coldbrew behavior (today)
- Downloads are parallelized with a single semaphore (`parallel_downloads`).
- Each download task does: download -> verify -> extract into store.
- Install steps (materialize -> relocate -> codesign -> link) run serially per package
  in dependency order.
- `crew update` downloads one `formula.json` file; there is no parallel update work.

## Proposed behavior (plan)
Split concurrency by stage and allow overlap across packages:

```
              +---------------------------+
              |   Install plan / order    |
              +---------------------------+
                           |
                           v
      +-----------------------------------------------------+
      |                 PARALLEL PIPELINE                   |
      |                                                     |
      | [Download semaphore]                                |
      |   dl A   dl B   dl C   ...                           |
      |     |     |     |                                    |
      |   verify verify verify                               |
      |     |     |     |                                    |
      | [Extract semaphore]                                  |
      |   xtr A  xtr B  xtr C ...                             |
      |     |     |     |                                    |
      |   materialize (can overlap across packages)          |
      |     |     |     |                                    |
      |   relocate (macOS)                                   |
      |     |     |     |                                    |
      | [Codesign semaphore]                                 |
      |   sign A sign B sign C ...                            |
      |     |     |     |                                    |
      |   link + metadata                                    |
      +-----------------------------------------------------+
```

We still respect dependency order for correctness, but independent packages can overlap in
different stages. Each stage has its own bounded concurrency.

## Concurrency defaults (proposal)
These are defaults only; users can override in config.

- Downloads (network-bound): `min(max(2, cpus * 2), 16)`
- Extractions (CPU + disk): `min(max(1, cpus - 1), 4)`
- Codesigning (disk + securityd): `min(max(1, cpus), 4)`

Rationale:
- Downloads benefit from higher concurrency.
- Extraction and codesign are disk-heavy; caps avoid I/O thrash on fast CPUs.
- We can tune these later using real metrics.

Note:
- Install concurrency is currently bounded by `parallel_extractions` (no separate knob yet).

## Guardrails and risks
Multi-stage semaphores can introduce new problems if we are not careful:

- Deadlocks: never hold two stage permits at once. Acquire just-in-time and release before moving
  to the next stage.
- Backpressure: use bounded queues between stages so downloads cannot outrun extraction.
- Duplicate work: enforce per-sha locks in the store so we never extract the same bottle twice.
- Retries: keep retry counts bounded and use backoff to avoid stampedes.
- Progress UX: show stage summaries rather than noisy per-file updates.

## Comparison (high level)
- Current Coldbrew: parallel downloads only; extract and codesign are not separately bounded.
- Planned Coldbrew: explicit stage limits and overlap between packages.
- Homebrew (conceptual): focuses on download concurrency; this plan adds stage-specific limits for
  our install pipeline. We are not changing update behavior in this phase.

## Implementation notes (future)
- Add settings: `parallel_extractions`, `parallel_codesigning`.
- Wire stage semaphores in `src/ops/install.rs`.
- Keep `crew update` unchanged (single index file download).
