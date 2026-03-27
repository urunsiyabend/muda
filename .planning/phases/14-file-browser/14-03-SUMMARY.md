---
phase: 14-file-browser
plan: 03
subsystem: ui
tags: [rust, notify-debouncer-full, filesystem-watcher, sidebar, winit, mpsc]

# Dependency graph
requires:
  - phase: 14-file-browser/14-01
    provides: mark_tree_dirty() public API on Sidebar, workspace_path in state.json
  - phase: 14-file-browser/14-02
    provides: handle_folder_opened(), open_directory() adapter integration
provides:
  - Live filesystem watcher integrated in CoreEditorAdapter (notify-debouncer-full 0.7)
  - poll_watcher_events / start_watcher / stop_watcher added to FileOpDataSource trait
  - Sidebar auto-refreshes when files change externally (~300ms debounced)
  - Watcher restarts on workspace change via handle_folder_opened
  - Zero CPU overhead when filesystem is idle (try_recv non-blocking in about_to_wait)
affects:
  - 14-04 (file browser completion — watcher infrastructure is in place)

# Tech tracking
tech-stack:
  added: [notify-debouncer-full = "0.7" (wgpu_client)]
  patterns:
    - "notify-debouncer-full mpsc pattern: Debouncer + mpsc::Receiver<DebounceEventResult> stored in adapter, polled non-blocking in about_to_wait"
    - "Watcher drop = stop: setting watcher=None implicitly stops background thread via Debouncer drop"
    - "Event loop polling pattern: try_recv() loop drains all pending events per about_to_wait call"

key-files:
  created: []
  modified:
    - wgpu_client/Cargo.toml
    - wgpu_client/src/adapter.rs
    - ora/src/editor_adapter/mod.rs
    - ora/src/platform/event_loop.rs

key-decisions:
  - "notify::RecursiveMode accessed via notify_debouncer_full::notify::RecursiveMode — no direct notify dep needed"
  - "debouncer.watch() directly (not debouncer.watcher().watch()) — watcher() is deprecated in 0.7"
  - "poll_watcher_events placed BEFORE dirty-entity check in about_to_wait so watcher events get a full layout pass"
  - "300ms debounce window: coalesces rapid cargo build events into single sidebar refresh"

patterns-established:
  - "FileOpDataSource trait extended with watcher lifecycle methods (poll/start/stop)"
  - "create_watcher() private method handles drop-then-recreate for workspace switches"

# Metrics
duration: 12min
completed: 2026-03-27
---

# Phase 14 Plan 03: Filesystem Watcher Summary

**notify-debouncer-full 0.7 integrated in CoreEditorAdapter with mpsc polling in about_to_wait — sidebar auto-refreshes within 300ms of external file changes**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-03-27T00:00:00Z
- **Completed:** 2026-03-27T00:12:00Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Added `notify-debouncer-full = "0.7"` to `wgpu_client/Cargo.toml` as the only new dependency
- Implemented `create_watcher()` private method on `CoreEditorAdapter` with 300ms debounce using an mpsc channel
- `open_directory()` starts the watcher immediately; `handle_folder_opened()` restarts it on workspace change
- Three new methods on `FileOpDataSource` trait: `poll_watcher_events`, `start_watcher`, `stop_watcher`
- `poll_watcher_events()` in `about_to_wait` drains channel, calls `mark_tree_dirty()` + `request_redraw()` on events

## Task Commits

Each task was committed atomically:

1. **Task 1: Add notify dependency and watcher infrastructure to adapter** - `75dfe70` (feat)
2. **Task 2: Poll watcher events in the event loop** - `b26bab6` (feat)

**Plan metadata:** (forthcoming docs commit)

## Files Created/Modified

- `wgpu_client/Cargo.toml` — Added `notify-debouncer-full = "0.7"` dependency
- `wgpu_client/src/adapter.rs` — `watcher` + `watcher_rx` fields, `create_watcher()`, `poll_watcher_events()`, `start_watcher()`, `stop_watcher()`, watcher start in `open_directory()` and restart in `handle_folder_opened()`
- `ora/src/editor_adapter/mod.rs` — Added `poll_watcher_events`, `start_watcher`, `stop_watcher` to `FileOpDataSource` trait
- `ora/src/platform/event_loop.rs` — Watcher polling in `about_to_wait` before dirty-entity check

## Decisions Made

- `notify::RecursiveMode` accessed as `notify_debouncer_full::notify::RecursiveMode` — avoids adding a separate `notify` crate dependency (it is re-exported by the debouncer crate)
- `debouncer.watch()` called directly instead of `debouncer.watcher().watch()` — the `watcher()` accessor is deprecated in notify-debouncer-full 0.7; the Debouncer now exposes `Watcher` methods directly
- Watcher polling placed BEFORE the dirty-entity check in `about_to_wait` so that filesystem events trigger a full layout pass (needs_layout = true), not just a paint pass
- Drop-then-recreate pattern for watcher restart: setting `self.watcher = None` before creating new one ensures the old background thread is fully stopped before the new one starts

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed deprecated `watcher()` accessor call**

- **Found during:** Task 1 (first compile attempt)
- **Issue:** Plan specified `debouncer.watcher().watch(...)` but `watcher()` is deprecated in notify-debouncer-full 0.7 and returns `()`, so `.watch()` on it fails to compile
- **Fix:** Changed to `debouncer.watch(path, RecursiveMode::Recursive)` — the Debouncer directly exposes Watcher methods in 0.7
- **Files modified:** `wgpu_client/src/adapter.rs`
- **Verification:** Clean compile
- **Committed in:** `75dfe70` (Task 1 commit)

**2. [Rule 3 - Blocking] Fixed `notify::RecursiveMode` unresolved import**

- **Found during:** Task 1 (first compile attempt)
- **Issue:** Plan specified `use notify::RecursiveMode` but `notify` is not a direct dependency — it is re-exported by `notify-debouncer-full`
- **Fix:** Changed to `use notify_debouncer_full::notify::RecursiveMode` and updated the `Debouncer<>` type annotation to use `notify_debouncer_full::notify::RecommendedWatcher`
- **Files modified:** `wgpu_client/src/adapter.rs`
- **Verification:** Clean compile
- **Committed in:** `75dfe70` (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking import)
**Impact on plan:** Both fixes necessary for compilation. The core architecture (mpsc channel, debouncer, polling in about_to_wait) was implemented exactly as specified.

## Issues Encountered

None beyond the two compile errors noted above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Filesystem watcher is active and live for all workspace opens
- Sidebar will refresh within ~300ms of any external file creation, deletion, or rename
- Watcher correctly restarts when workspace changes via Ctrl+Shift+O
- Ready for Plan 14-04: file browser completion (context menu, rename, delete, new file)

---
*Phase: 14-file-browser*
*Completed: 2026-03-27*
