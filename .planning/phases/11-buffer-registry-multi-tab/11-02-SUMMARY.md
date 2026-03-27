---
phase: 11-buffer-registry-multi-tab
plan: 02
subsystem: tab-management
tags: [tab-switching, keybinding, command-dispatch, dirty-protection, mru]

# Dependency graph
requires:
  - phase: 11-01
    provides: tab_order, mru_stack, set_active_view, close_document_protected — all used by the new App methods
provides:
  - App::switch_tab(view_id) — direct tab switch by ViewId (for click handling in Plan 03)
  - App::switch_tab_relative(offset) — wrapping forward/backward cycle via tab_order
  - App::close_active_tab() — dirty-buffer protected close with PendingAction dialog
  - EditorCommand::SwitchTabPrev variant in ora types
  - Ctrl+Shift+Tab keybinding mapping to SwitchTabPrev
  - Adapter dispatches SwitchTab/SwitchTabPrev/CloseTab to real App handlers (no more stubs)
affects:
  - 11-03 (tab click dispatch uses SwitchTab(view_id))
  - any phase adding more keybindings to editor_input.rs

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Tab switching uses set_active_view directly — never can_switch_active (which blocks on dirty)"
    - "Ctrl+Shift+Tab guard placed before Ctrl+Tab in match to ensure correct Rust guard ordering"
    - "dispatch_command early-returns before stub block for real command handlers"

key-files:
  created: []
  modified:
    - core_editor/src/app.rs
    - ora/src/editor_adapter/types.rs
    - wgpu_client/src/adapter.rs
    - ora/src/events/editor_input.rs

key-decisions:
  - "switch_tab_relative uses isize arithmetic with rem_euclid to handle negative offsets safely"
  - "close_active_tab catches all ProtectionError variants (both UnsavedChanges and MultipleUnsavedChanges) and routes to PendingAction dialog"
  - "SwitchTab(0) = cycle next, SwitchTab(view_id) = direct switch, SwitchTabPrev = cycle prev — three-variant design avoids sentinel abuse"

patterns-established:
  - "App tab methods: switch_tab, switch_tab_relative, close_active_tab — direct workspace calls, never dialog-guarded"
  - "Adapter dispatch_command: real handlers matched first with early return, stub block only for unimplemented commands"

# Metrics
duration: ~15min
completed: 2026-03-26
---

# Phase 11 Plan 02: Tab Command Handlers Summary

**SwitchTab/CloseTab wired end-to-end: keyboard to adapter to App to Workspace, with Ctrl+Shift+Tab reverse-cycling and dirty-buffer protection on close.**

## Performance

- **Duration:** ~15 min
- **Completed:** 2026-03-26
- **Tasks:** 2 completed
- **Files modified:** 4

## Accomplishments

- Implemented `App::switch_tab`, `switch_tab_relative`, and `close_active_tab` with full dirty-buffer protection
- Added `EditorCommand::SwitchTabPrev` to ora types for backward cycling
- Wired Ctrl+Tab (forward) and Ctrl+Shift+Tab (backward) keybindings to real handlers (no more stubs)
- Ctrl+W (CloseTab) now triggers the save/discard/cancel dialog for unsaved buffers
- Added 7 new tests: 6 App-level tab tests + 1 keybinding test

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement switch_tab, switch_tab_relative, close_active_tab in App** - `7733236` (feat)
2. **Task 2: Wire adapter stubs to real handlers and add Ctrl+Shift+Tab** - `31833cb` (feat)

## Files Created/Modified

- `core_editor/src/app.rs` — Added switch_tab, switch_tab_relative, close_active_tab methods + 6 unit tests
- `ora/src/editor_adapter/types.rs` — Added EditorCommand::SwitchTabPrev variant
- `wgpu_client/src/adapter.rs` — Replaced SwitchTab/CloseTab stubs with real dispatch; SwitchTabPrev added to exhaustive match
- `ora/src/events/editor_input.rs` — Added Ctrl+Shift+Tab → SwitchTabPrev keybinding + test

## Decisions Made

- **switch_tab_relative uses isize rem_euclid**: avoids panic on negative modulo; wraps correctly at both ends.
- **close_active_tab catches all ProtectionError variants**: both `UnsavedChanges` and `MultipleUnsavedChanges` are routed to the existing `PendingAction::CloseDocument` dialog path.
- **Ctrl+Shift+Tab guard ordering**: the `if ctrl && shift` arm is placed before `if ctrl` in the match because Rust evaluates guards top-to-bottom on the same pattern.
- **No new PendingAction variants needed**: `PendingAction::CloseDocument(doc_id)` already existed from prior implementation.

## Deviations from Plan

None — plan executed exactly as written.

## Verification

- `cargo build --workspace` — clean (only pre-existing warnings)
- `cargo test --workspace` — 275 tests pass (114 core_editor + 161 ora)
- SwitchTab(0) → switch_tab_relative(1) (forward cycle via tab_order)
- SwitchTabPrev → switch_tab_relative(-1) (backward cycle via tab_order)
- CloseTab → close_active_tab() → triggers dialog for dirty buffer, closes immediately for clean
- Tab switching never calls can_switch_active

## Next Phase Readiness

Plan 03 (tab click handling in UI) can now call `dispatch_command(SwitchTab(view_id))` and expect it to switch to that specific tab. The infrastructure is complete.
