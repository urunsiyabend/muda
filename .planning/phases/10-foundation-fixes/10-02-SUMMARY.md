---
phase: 10-foundation-fixes
plan: 02
subsystem: ui
tags: [rust, editor, keybindings, traits, wgpu, ora]

# Dependency graph
requires:
  - phase: 10-foundation-fixes plan 01
    provides: foundation fixes groundwork (FIX-01, FIX-02)
provides:
  - 9 new EditorCommand variants for v2 keybindings (FIX-03)
  - Ctrl+W/O/N/F/H/G/Shift+S keybindings with stub status bar messages
  - BufferDataSource, CommandDispatcher, WindowDataSource sub-traits (FIX-04)
  - Blanket EditorDataSource super-trait impl for backward compatibility
affects: [10-foundation-fixes, phase-11-tabs, phase-12-file-ops, phase-14-find-replace]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Stub-first command pattern: add variant + keybinding + status message before implementing"
    - "Sub-trait decomposition: monolithic trait split into focused sub-traits with blanket super-trait"
    - "Interior mutability via unsafe field pointer for pending_status_message in build_render_model"

key-files:
  created: []
  modified:
    - ora/src/editor_adapter/types.rs
    - ora/src/editor_adapter/mod.rs
    - ora/src/events/editor_input.rs
    - wgpu_client/src/adapter.rs

key-decisions:
  - "Ctrl+Shift+S -> SaveAs guard placed before Ctrl+S -> Save in match arm ordering"
  - "pending_status_message field on CoreEditorAdapter bridges &self build_render_model with &mut self dispatch_command"
  - "EditorDataSource kept as blanket super-trait so all existing Box<dyn EditorDataSource> usages compile unchanged"
  - "v2 stub commands return early from dispatch_command before to_core_command to avoid exhaustiveness issues"

patterns-established:
  - "Trait split pattern: BufferDataSource (render) + CommandDispatcher (commands) + WindowDataSource (chrome) -> blanket EditorDataSource"
  - "Match arm guard ordering: more-specific guards (with shift) before less-specific (without)"

# Metrics
duration: 4min
completed: 2026-03-26
---

# Phase 10 Plan 02: v2 Commands + Trait Split Summary

**9 new EditorCommand variants with keybindings and stub status bar feedback, plus EditorDataSource split into BufferDataSource/CommandDispatcher/WindowDataSource sub-traits with blanket super-trait**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-03-25T23:21:51Z
- **Completed:** 2026-03-25T23:26:04Z
- **Tasks:** 2/2
- **Files modified:** 4

## Accomplishments

- Added 9 new `EditorCommand` variants: `SaveAs`, `OpenFile`, `New`, `SwitchTab(u64)`, `CloseTab`, `Find`, `Replace`, `ReplaceAll`, `GoToLine`
- Wired keybindings: Ctrl+W->CloseTab, Ctrl+O->OpenFile, Ctrl+N->New, Ctrl+F->Find, Ctrl+H->Replace, Ctrl+G->GoToLine, Ctrl+Tab->SwitchTab(0), Ctrl+Shift+S->SaveAs (priority over Ctrl+S->Save)
- Stub handlers in `CoreEditorAdapter::dispatch_command` show informative status bar messages via `pending_status_message` field
- Split monolithic `EditorDataSource` into `BufferDataSource` + `CommandDispatcher` + `WindowDataSource`, with blanket impl preserving all existing usages
- 9 new keybinding tests added (160 total, all passing)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add v2 EditorCommand variants + keybindings + stub handlers (FIX-03)** - `4f060b1` (feat)
2. **Task 2: Split EditorDataSource into focused sub-traits (FIX-04)** - `9984ce7` (refactor)

## Files Created/Modified

- `ora/src/editor_adapter/types.rs` - Added 9 new `EditorCommand` variants
- `ora/src/editor_adapter/mod.rs` - Replaced monolithic trait with `BufferDataSource`, `CommandDispatcher`, `WindowDataSource` sub-traits + blanket `EditorDataSource` impl
- `ora/src/events/editor_input.rs` - Added Ctrl+W/O/N/F/H/G keybindings, Ctrl+Shift+S->SaveAs priority, Ctrl+Tab->SwitchTab, plus 9 new tests
- `wgpu_client/src/adapter.rs` - Added `pending_status_message` field, stub handlers, split impl into 3 blocks, updated imports

## Decisions Made

- **Ctrl+Shift+S priority**: In Rust match arms, `"s" | "S" if shift =>` placed before `"s" | "S" =>` so SaveAs is caught first. The `Key::Character` on Windows delivers "S" (uppercase) when Shift is held, so the pattern `"s" | "S"` covers both.
- **pending_status_message bridge**: `build_render_model` takes `&self` but `dispatch_command` takes `&mut self`. Added `pending_status_message: Option<String>` to the struct; stub handlers set it in `dispatch_command`, `build_render_model` takes it via the existing unsafe interior-mutability pattern already used for `app.build_render_model`.
- **Blanket super-trait**: `EditorDataSource` kept as a marker super-trait with no additional methods. The blanket `impl<T> EditorDataSource for T where T: BufferDataSource + CommandDispatcher + WindowDataSource {}` means no explicit `impl EditorDataSource` is needed in `adapter.rs` — backward compatible.
- **v2 commands in to_core_command**: Added exhaustiveness arms returning `None` for all 9 stub variants, but they are intercepted earlier in `dispatch_command` before `to_core_command` is called.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- **Unsafe cast lint**: The initial refactor of `build_render_model` tried `&mut *(self as *const CoreEditorAdapter as *mut CoreEditorAdapter)` for the whole struct, which triggered `invalid_reference_casting` (deny-level lint). Reverted to the existing pattern of casting individual fields separately, matching the pre-existing approach for `app`.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All v2 keybindings recognized without panic (FIX-03 complete)
- Sub-trait architecture in place for future phases to add focused methods (FIX-04 complete)
- Phase 11 (tabs) can add `switch_tab`, `close_tab` methods to `CommandDispatcher`
- Phase 12 (file ops) can add file I/O methods to `CommandDispatcher` with result callback design
- Phase 14 (find/replace) can add search methods to `BufferDataSource`

---
*Phase: 10-foundation-fixes*
*Completed: 2026-03-26*
