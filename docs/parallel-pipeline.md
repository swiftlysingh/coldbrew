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
- Downloads are parallelized (`parallel_downloads`) and bounded separately from extraction.
- Extraction is bounded (`parallel_extractions`) and uses per-sha store locks.
- Install steps (materialize -> relocate -> codesign -> link) can overlap across packages,
  bounded by `parallel_installs`.
- Codesigning is bounded (`parallel_codesigning`, macOS only).
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
- Install concurrency is currently bounded by `parallel_installs`.

## Guardrails and risks
Multi-stage semaphores can introduce new problems if we are not careful:

- Deadlocks: never hold two stage permits at once. Acquire just-in-time and release before moving
  to the next stage.
- Backpressure: use bounded queues between stages so downloads cannot outrun extraction.
- Duplicate work: enforce per-sha locks in the store so we never extract the same bottle twice.
- Retries: keep retry counts bounded and use backoff to avoid stampedes.
- Progress UX: show stage summaries rather than noisy per-file updates.

## Comparison (high level)
- Coldbrew: explicit stage limits and overlap between packages.
- Homebrew (conceptual): focuses on download concurrency; Coldbrew adds stage-specific limits for
  the install pipeline. We are not changing update behavior in this phase.

## Implementation notes
- Settings: `parallel_downloads`, `parallel_extractions`, `parallel_codesigning`,
  `parallel_installs`.
- Stage semaphores live in `src/ops/install.rs`.
- Install stage totals are logged in debug output.
- `crew update` is unchanged (single index file download).
