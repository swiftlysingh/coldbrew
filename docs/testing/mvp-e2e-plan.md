# Coldbrew MVP E2E Test Plan

Status: Draft  
Last updated: January 28, 2026

## Goal
Verify the core user flows work end-to-end with real network data and a clean filesystem.

## Test Environment
- Use a disposable root: `export COLDBREW_HOME=/tmp/coldbrew-e2e`
- Ensure network access is available.
- Use a clean terminal session per run.

## Core Flows

### 1) Update + Search + Info
- `coldbrew update`
- `coldbrew search jq`
- `coldbrew info jq`
**Expected:** index downloads, search returns results, info prints formula details.

### 2) Install + Which + Exec
- `coldbrew install jq`
- `coldbrew which jq`
- `coldbrew exec jq -- --version`
**Expected:** install completes, which resolves to coldbrew shim, exec runs.

### 3) Dependencies
- `coldbrew install ffmpeg` (or another dep-heavy formula)
**Expected:** dependencies are installed and listed.

### 4) Upgrade (Interactive)
- `coldbrew upgrade`
**Expected:** shows plan; if no upgrades, exits cleanly.

### 5) Cache and Clean
- `coldbrew cache info`
- `coldbrew cache list`
- `coldbrew clean --dry-run`
**Expected:** cache stats show; clean shows candidate removals without changes.

### 6) Uninstall
- `coldbrew uninstall jq`
- `coldbrew list`
**Expected:** jq removed; list no longer includes jq.

## Lockfile Flow
- Create a `coldbrew.toml` with 2–3 packages.
- `coldbrew lock`
- `coldbrew install` (from lockfile)
**Expected:** lockfile created, install uses pinned versions.

## Failure Cases
- `coldbrew install does-not-exist`
  - **Expected:** clear error and suggestion.
- `coldbrew info does-not-exist`
  - **Expected:** clear error.

## Success Criteria
- All core flows complete without panic or data corruption.
- Errors are user-friendly and include suggestions where possible.

## Cleanup
- `rm -rf /tmp/coldbrew-e2e`
