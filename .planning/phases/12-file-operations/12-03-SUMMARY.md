---
phase: 12-file-operations
plan: 03
subsystem: file-operations
tags: [rust, save, save-as, rfd, async-dialog, last-dir, state-json, file-persistence]

# Dependency graph
requires:
  - phase: 12-02
    provides: FileOpDataSource trait, PendingFileOp queue, spawn_open_dialog, rfd AsyncFileDialog pattern
provides:
  - Save As dialog via rfd::AsyncFileDialog::save_file (Ctrl+Shift+S)
  - Silent save for named files via PendingFileOp::Save (Ctrl+S)
  - handle_file_saved callback: writes disk, updates file_path, clears dirty, registers buffer registry
  - Last-used directory persistence in platform config dir (state.json)
  - Open dialog pre-populated with last-used directory
affects: [13-sidebar-navigation, future-find-replace]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "PendingFileOp::Save routes through event loop for path-check branching (named = silent, untitled = Save As)"
    - "handle_file_saved callback pattern: same adapter callback for both Save As and future drag-save"
    - "Manual JSON serialization for state.json (no serde dep, minimal parse with string search)"
    - "last_dir field on CoreEditorAdapter loaded at init, updated on every successful open/save"

key-files:
  created: []
  modified:
    - ora/src/editor_adapter/mod.rs
    - ora/src/platform/event_loop.rs
    - wgpu_client/src/adapter.rs
    - core_editor/src/domain/workspace.rs

key-decisions:
  - "PendingFileOp::Save queued by dispatch_command; event loop owns the has_path decision — avoids duplicating logic in adapter"
  - "handle_file_saved calls app.save_as() synchronously (file writes are fast for typical editor files)"
  - "last_dir stored as PathBuf field on CoreEditorAdapter, persisted to state.json via minimal manual JSON (no serde)"
  - "register_path_for_active_doc added to Workspace to keep buffer registry consistent after Save As"
  - "save_active_doc wraps app.save() with Result<bool,String> so event loop can stay ignorant of core_editor types"

patterns-established:
  - "FileOpDataSource trait is the only channel between event loop and adapter for file callbacks"
  - "All dialog-open state changes (set/clear) happen inside the spawned async future, not in spawn method"

# Metrics
duration: 4min
completed: 2026-03-26
---

# Phase 12 Plan 03: Save and Save As Summary

**Ctrl+S / Ctrl+Shift+S file saving with native rfd save dialog, silent save for named files, and last-used directory persistence via state.json**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-03-26T17:58:34Z
- **Completed:** 2026-03-26T18:01:53Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Ctrl+S on a named file saves silently — dirty indicator disappears, status bar shows "File saved"
- Ctrl+S on an untitled buffer and Ctrl+Shift+S open the native Save As dialog via rfd::AsyncFileDialog::save_file
- Save As path is written to disk, document's file_path updated, dirty flag cleared, buffer registry updated
- Last-used directory persisted to platform config dir (`%APPDATA%/muda/state.json` on Windows) and restored at startup
- Both Open and Save As dialogs pre-populated with last-used directory

## Task Commits

Each task was committed atomically:

1. **Task 1 + 2: Save/SaveAs flow + last-dir persistence** - `219b19d` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `ora/src/editor_adapter/mod.rs` - Added `handle_file_saved`, `active_doc_has_path`, `save_active_doc`, `last_opened_directory` to `FileOpDataSource` trait
- `ora/src/platform/event_loop.rs` - `poll_pending_file_ops` handles `PendingFileOp::Save` (branching) and `PendingFileOp::SaveAs`; new `spawn_save_as_dialog` method; `spawn_open_dialog` now uses `set_directory`
- `wgpu_client/src/adapter.rs` - `config_state_path`, `load_last_dir`, `save_last_dir` free functions; `last_dir` field on `CoreEditorAdapter`; full trait implementation of all new methods
- `core_editor/src/domain/workspace.rs` - Added `register_path_for_active_doc` to insert canonical path into `path_to_doc` buffer registry

## Decisions Made

- **PendingFileOp::Save via event loop**: `dispatch_command` now queues `PendingFileOp::Save` instead of calling `app.save()` directly. The event loop owns the has-path decision — this avoids duplicating the "named vs untitled" branching in the adapter.
- **Synchronous save_as in callback**: `handle_file_saved` calls `app.save_as()` synchronously inside the async future. File writes are fast for typical editor-sized files, and core_editor's `save_as` is `std::fs::write` already.
- **Manual JSON for state.json**: Avoided pulling in serde for a two-field file. A simple string search extracts `"last_dir"` and manual escaping writes it back.
- **Tasks 1 and 2 committed together**: The `last_opened_directory` trait method is required to compile `FileOpDataSource` (Task 1 foundation), making them architecturally inseparable in a single commit.

## Deviations from Plan

None — plan executed exactly as written. Tasks 1 and 2 were committed in one commit because the `last_opened_directory` trait method (Task 2) is part of the `FileOpDataSource` trait interface that Task 1 also modifies, making a clean split-commit impossible without a temporary stub.

## Issues Encountered

None.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Full file operation loop (open → edit → save) is now complete for Phase 12
- Phase 12 is done: Ctrl+N (new untitled), Ctrl+O (open), Ctrl+S (save/save-as), Ctrl+Shift+S (save-as)
- Ready for Phase 13 (sidebar navigation) or Phase 14 (selection editing)
- Known gap: window close (X button) does not check dirty documents — exits without a save dialog (tracked in STATE.md Known Issues)

---
*Phase: 12-file-operations*
*Completed: 2026-03-26*
