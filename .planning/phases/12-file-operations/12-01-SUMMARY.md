---
phase: 12-file-operations
plan: 01
subsystem: file-operations
tags: [rust, rfd, dirs, file-dialog, untitled-buffer, pending-file-op, queue-pattern]

# Dependency graph
requires:
  - phase: 11-buffer-registry
    provides: "Multi-tab Workspace with create_view, set_active_view, tab_order"
provides:
  - "rfd 0.15 + dirs 5 dependencies in wgpu_client"
  - "Document::new_with_name() constructor for named untitled documents"
  - "DocumentMetadata::untitled_name field for display name"
  - "Workspace::create_untitled_document() with monotonic counter"
  - "App::new_untitled() creating doc + view + setting active"
  - "PendingFileOp enum (Open/SaveAs/Save) on CoreEditorAdapter"
  - "dialog_open guard + take_pending_file_op() on CoreEditorAdapter"
  - "Working Ctrl+N: creates 'Untitled', 'Untitled (2)', ... tabs"
  - "Ctrl+S on untitled queues SaveAs pending op"
affects:
  - 12-file-operations (plans 02, 03 build Open and Save dialogs on top of PendingFileOp queue)
  - Future: window title, status bar (title() now returns untitled_name for untitled docs)

# Tech tracking
tech-stack:
  added:
    - "rfd 0.15 (default-features = false, features = [\"xdg-portal\"]) — native file dialogs"
    - "dirs 5 — platform home/documents directory paths"
  patterns:
    - "PendingFileOp queue: dispatch_command sets pending_file_op; event loop polls take_pending_file_op() to open dialogs off the main call stack"
    - "dialog_open guard: prevents concurrent file dialogs from being queued"
    - "Untitled counter: monotonically increasing u32 on Workspace, never resets within session"

key-files:
  created: []
  modified:
    - "wgpu_client/Cargo.toml — rfd + dirs dependencies"
    - "core_editor/src/domain/document.rs — untitled_name field, new_with_name(), title() update, set_file_path() clears untitled_name"
    - "core_editor/src/domain/workspace.rs — untitled_counter, create_untitled_document()"
    - "core_editor/src/app.rs — new_untitled() method"
    - "wgpu_client/src/adapter.rs — PendingFileOp enum, new fields, take_pending_file_op(), Ctrl+N/O/SaveAs dispatch handlers"

key-decisions:
  - "PendingFileOp queue pattern chosen over channels/callbacks because dispatch_command returns () and file dialogs must run on the event loop thread"
  - "dialog_open bool guard (not atomic) — single-threaded GUI, no cross-thread contention"
  - "untitled_name stored on DocumentMetadata (not Document root) to keep it grouped with uri/dirty/line_ending"
  - "set_file_path() clears untitled_name — once a file is saved, the untitled name is gone"
  - "Ctrl+S on untitled queues SaveAs instead of showing error — better UX"

patterns-established:
  - "Untitled naming: counter == 1 => 'Untitled', counter > 1 => 'Untitled (N)'"
  - "File op dispatch: set pending_file_op in dispatch_command, consume in event loop via take_pending_file_op()"

# Metrics
duration: 4min
completed: 2026-03-26
---

# Phase 12 Plan 01: File Operations Foundation Summary

**PendingFileOp queue pattern + rfd/dirs deps + Ctrl+N untitled buffer naming — foundation for all subsequent file operations**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-03-26T17:41:26Z
- **Completed:** 2026-03-26T17:45:43Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- Added rfd 0.15 and dirs 5 to wgpu_client for file dialog capability
- Untitled buffer naming ("Untitled", "Untitled (2)", ...) with monotonic session counter in Workspace
- `Document::new_with_name()` and `DocumentMetadata::untitled_name` field; `title()` uses it; `set_file_path()` clears it
- `PendingFileOp` enum + `dialog_open` guard + `take_pending_file_op()` on `CoreEditorAdapter` — event loop polling pattern
- Ctrl+N now creates a real new untitled tab (no longer shows stub message)
- Ctrl+S on an untitled document queues `PendingFileOp::SaveAs`

## Task Commits

Each task was committed atomically:

1. **Task 1: Add dependencies and untitled naming infrastructure** - `2c7bc64` (feat)
2. **Task 2: PendingFileOp queue, dialog_open guard, and Ctrl+N handler** - `5a2b46d` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `wgpu_client/Cargo.toml` - Added rfd 0.15 + dirs 5 dependencies
- `core_editor/src/domain/document.rs` - `untitled_name` field on `DocumentMetadata`, `new_with_name()` constructor, updated `title()`, `set_file_path()` clears name
- `core_editor/src/domain/workspace.rs` - `untitled_counter: u32` field, `create_untitled_document()` method
- `core_editor/src/app.rs` - `new_untitled()` method
- `wgpu_client/src/adapter.rs` - `PendingFileOp` enum, `pending_file_op`/`dialog_open` fields, `take_pending_file_op()`, Ctrl+N/O/SaveAs handlers

## Decisions Made

- **PendingFileOp queue pattern**: `dispatch_command` returns `()` so there's no way to return a future or callback inline. The event loop polls `take_pending_file_op()` each frame — clean separation between command dispatch and dialog lifecycle.
- **dialog_open bool guard**: Single-threaded GUI, no atomic needed. Prevents duplicate dialogs if user triggers command while dialog is already open.
- **untitled_name on DocumentMetadata**: Grouped with uri/dirty/line_ending rather than on Document root — semantically it's metadata about the document's persistence state.
- **set_file_path clears untitled_name**: Once a real path is assigned, the untitled display name is no longer relevant.
- **Ctrl+S on untitled queues SaveAs**: Better UX than showing an error message. Mirrors VS Code behavior.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed missing `untitled_name` field in `Document::from_str` initializer**

- **Found during:** Task 1 (running `cargo check -p core_editor`)
- **Issue:** Adding `untitled_name` to `DocumentMetadata` caused `E0063` (missing field) in the `from_str` constructor which uses struct literal syntax.
- **Fix:** Added `untitled_name: None` to the `DocumentMetadata` struct literal in `Document::from_str`.
- **Files modified:** `core_editor/src/domain/document.rs`
- **Verification:** `cargo check -p core_editor` passes; all 116 tests pass.
- **Committed in:** `2c7bc64` (part of Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - Bug)
**Impact on plan:** Routine struct literal update after adding a new field. No scope creep.

## Issues Encountered

None — compile error was caught immediately during first `cargo check` and fixed inline.

## User Setup Required

None — no external service configuration required. rfd and dirs are compiled into the binary.

## Next Phase Readiness

- **Plan 02 (Open dialog):** `PendingFileOp::Open` queue entry is set by Ctrl+O. Event loop in Plan 02 calls `take_pending_file_op()` and opens `rfd::AsyncFileDialog`. Foundation ready.
- **Plan 03 (Save As dialog):** `PendingFileOp::SaveAs` is queued by Ctrl+S on untitled and Ctrl+Shift+S. Foundation ready.
- **Concern:** The `dialog_open` guard needs to be set/cleared by the event loop (Plan 02/03) — it's only declared here, not yet wired to actual dialog lifecycle.
- **116 tests pass**, no regressions.

---
*Phase: 12-file-operations*
*Completed: 2026-03-26*
