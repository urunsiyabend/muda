---
phase: 12-file-operations
plan: 02
subsystem: ui
tags: [rfd, async-dialog, file-io, buffer-registry, dedup, utf8, bom]

# Dependency graph
requires:
  - phase: 12-01
    provides: PendingFileOp queue pattern, dialog_open guard, CoreEditorAdapter struct fields

provides:
  - "Ctrl+O native file picker via rfd::AsyncFileDialog"
  - "Async file reading on background thread with yield_now() for UI responsiveness"
  - "UTF-8 validation with user-friendly error message on failure"
  - "UTF-8 BOM stripping on load"
  - "Buffer Registry deduplication via open_document_with_content"
  - "FileOpDataSource trait on EditorDataSource for clean polymorphism"
  - "PendingFileOp relocated to ora crate (authoritative source)"

affects: [12-03, event_loop, adapter, workspace]

# Tech tracking
tech-stack:
  added:
    - "rfd 0.15 (AsyncFileDialog) added to ora/Cargo.toml"
    - "futures-lite 2 (yield_now) added to ora/Cargo.toml"
  patterns:
    - "FileOpDataSource sub-trait pattern: file-op methods isolated from buffer/command/window traits"
    - "LocalExecutor async task pattern: Rc<RefCell<dyn Trait>> captured in !Send future"
    - "Background thread + mpsc channel pattern for blocking file I/O from async context"
    - "poll_pending_file_ops() called after every dispatch_command to bridge sync→async boundary"

key-files:
  created: []
  modified:
    - "core_editor/src/domain/workspace.rs - open_document_with_content method"
    - "ora/src/editor_adapter/types.rs - PendingFileOp enum added"
    - "ora/src/editor_adapter/mod.rs - FileOpDataSource trait + EditorDataSource super-trait updated"
    - "ora/Cargo.toml - rfd and futures-lite dependencies"
    - "wgpu_client/src/adapter.rs - FileOpDataSource impl, local PendingFileOp removed"
    - "ora/src/platform/event_loop.rs - poll_pending_file_ops, spawn_open_dialog methods"

key-decisions:
  - "FileOpDataSource is a separate sub-trait (not merged into CommandDispatcher) — follows the sub-trait split pattern from FIX-04"
  - "PendingFileOp relocated from wgpu_client to ora so the event loop can match on it without depending on wgpu_client"
  - "open_document_with_content does NOT create a view — view creation remains the adapter's responsibility for clean separation"
  - "spawn_open_dialog uses LocalExecutor (not Send executor) enabling Rc<RefCell<...>> capture in async block safely"
  - "Background thread + mpsc::try_recv + yield_now loop used for file I/O — avoids blocking the async executor"
  - "Task::detach() used on spawned future — dialog result delivered via adapter callbacks, no await needed"

patterns-established:
  - "Platform file dialog pattern: dispatch_command queues op → poll_pending_file_ops dispatches → LocalExecutor future handles async I/O → adapter callbacks deliver result"
  - "Dedup pattern: open_document_with_content returns (doc_id, was_existing); adapter activates existing view or creates new one"

# Metrics
duration: 25min
completed: 2026-03-26
---

# Phase 12 Plan 02: Open File Dialog Integration Summary

**Ctrl+O opens native OS file picker via rfd AsyncFileDialog, reads files on a background thread, validates UTF-8, strips BOM, and deduplicates via Buffer Registry — UI stays responsive throughout**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-03-26T17:56:34Z
- **Completed:** 2026-03-26T18:21:00Z
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments

- Workspace gains `open_document_with_content` — accepts pre-read content with path, Buffer Registry dedup, does not create view
- `FileOpDataSource` trait added to `EditorDataSource` super-trait, providing clean polymorphic interface for file-op callbacks
- `PendingFileOp` relocated from `wgpu_client` to `ora::editor_adapter::types` so the event loop can use it without a circular dep
- Event loop gains `poll_pending_file_ops` (called after each `dispatch_command`) and `spawn_open_dialog` (async task on LocalExecutor)
- Async dialog future handles multi-select, per-file background I/O, UTF-8 validation, BOM stripping, and dialog_open guard cleanup

## Task Commits

Each task was committed atomically:

1. **Task 1: handle_file_loaded, handle_file_error, open_document_with_content** - `4ef9fce` (feat)
2. **Task 2: FileOpDataSource trait, PendingFileOp relocation, rfd to ora** - `113811a` (feat)
3. **Task 3: Event loop poll_pending_file_ops + spawn_open_dialog** - `923df9d` (feat)

## Files Created/Modified

- `core_editor/src/domain/workspace.rs` - Added `open_document_with_content(path, content) -> (DocumentId, bool)`: canonicalizes path, deduplicates via `path_to_doc`, creates Document via `from_str`, does NOT create view
- `ora/src/editor_adapter/types.rs` - Added `PendingFileOp { Open, SaveAs, Save }` enum (relocated from wgpu_client)
- `ora/src/editor_adapter/mod.rs` - Added `FileOpDataSource` trait with five methods; `EditorDataSource` super-trait updated to include it; added `use std::path::PathBuf`
- `ora/Cargo.toml` - Added `rfd = "0.15"` and `futures-lite = "2"` dependencies
- `wgpu_client/src/adapter.rs` - Removed local `PendingFileOp`; imports `PendingFileOp` and `FileOpDataSource` from ora; implements `FileOpDataSource` on `CoreEditorAdapter`
- `ora/src/platform/event_loop.rs` - Added `Rc` import and `PendingFileOp` import; added `poll_pending_file_ops` and `spawn_open_dialog` methods; calls `poll_pending_file_ops` after each `dispatch_command`

## Decisions Made

- **FileOpDataSource as separate sub-trait**: Following the FIX-04 sub-trait split pattern. Keeps buffer, command, window, and file-op concerns orthogonal.
- **PendingFileOp in ora**: The event loop lives in ora and needs to match on `PendingFileOp` variants. Moving it to ora eliminates a circular dependency that would arise if event_loop imported from wgpu_client.
- **open_document_with_content does not create view**: Follows single-responsibility — workspace manages document lifecycle; the adapter (caller) manages view lifecycle. This mirrors the pattern used by `open_document`.
- **LocalExecutor + Rc capture**: `AppContext.executor` is `LocalExecutor<'static>` (confirmed in context.rs:37). This does not require `Send`, so capturing `Rc<RefCell<dyn EditorDataSource>>` in the async block is safe on the single-threaded GUI thread.
- **Background thread + try_recv + yield_now**: File I/O is blocking; spawning a `std::thread` and polling `mpsc::Receiver::try_recv()` with `futures_lite::future::yield_now()` between polls gives cooperative I/O without blocking the executor.
- **Task::detach()**: The spawned future delivers its result via adapter callbacks, not by returning a value. Detaching is the correct pattern.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None — rfd's `xdg-portal` feature is Linux-specific but compiles on Windows without issue (the feature is simply inactive on non-Linux platforms). Full workspace compiled first attempt.

## Next Phase Readiness

- Ctrl+O file open is complete end-to-end
- Plan 03 (Save As / Save) can now implement `PendingFileOp::SaveAs` and `PendingFileOp::Save` handlers in `poll_pending_file_ops` — the stub arms are already present
- `FileOpDataSource` trait provides the interface Plan 03 needs for its own callbacks (`handle_save_as_completed`, etc.)

---
*Phase: 12-file-operations*
*Completed: 2026-03-26*
