---
phase: 09-transitions-integration
plan: "02"
subsystem: ui
tags: [rust, trait, adapter, mirror-types, editor, view-model]

# Dependency graph
requires:
  - phase: 07-editor-chrome-text-editing
    provides: ora views consuming core_editor::view_model types directly
  - phase: 08-advanced-ui-widgets
    provides: complete ora view library (AppLayout, TabBarView, etc.)
provides:
  - ora::editor_adapter::EditorDataSource trait
  - ora::editor_adapter::RenderModel and all mirror presentation types
  - EditorCommand enum with CursorDirection and MoveScope helpers
affects:
  - 09-03 (migrate ora views to consume EditorDataSource instead of core_editor)
  - 09-04 (wgpu_client implements EditorDataSource for core_editor::app::App)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Mirror type pattern: ora defines structurally identical copies of core_editor types; From conversions live in wgpu_client
    - Adapter trait pattern: EditorDataSource trait abstracts over editor backend; ora views receive &dyn EditorDataSource
    - Zero-dependency boundary: editor_adapter module contains no core_editor imports (only doc comments)

key-files:
  created:
    - ora/src/editor_adapter/types.rs
    - ora/src/editor_adapter/mod.rs
  modified:
    - ora/src/lib.rs

key-decisions:
  - "Mirror types defined in ora with no From impls — conversions deferred to wgpu_client (keeps ora dependency-free)"
  - "EditorCommand defined in types.rs alongside presentation types — single module owns the full adapter surface"
  - "CursorDirection and MoveScope defined as ora-native enums mirroring core_editor::commands::Direction and MoveScope"

patterns-established:
  - "Adapter boundary: views use &dyn EditorDataSource, never &core_editor::app::App"
  - "Mirror type isolation: ora types are identical in shape to core_editor types but fully standalone"

# Metrics
duration: 2min
completed: 2026-03-25
---

# Phase 09 Plan 02: EditorDataSource Trait and Mirror Types Summary

**ora::editor_adapter module with EditorDataSource trait and all 14 mirror presentation types from core_editor::view_model, establishing a zero-dependency adapter boundary**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-25T11:24:25Z
- **Completed:** 2026-03-25T11:26:32Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- Created `ora/src/editor_adapter/types.rs` with all 14 presentation types mirroring `core_editor::view_model`: TextStyle, VisualPosition, StyledSpan, LinePresentation, GutterModel, StatusPresentation, CaretPresentation, DialogPresentation, FileEntryPresentation, SidebarPresentation, TabPresentation, FileTreeNode, FileTreePresentation, TabBarPresentation, RenderModel — plus EditorCommand, CursorDirection, MoveScope
- Created `ora/src/editor_adapter/mod.rs` with the `EditorDataSource` trait (build_render_model, dispatch_command, window_title)
- Added `pub mod editor_adapter` to `ora/src/lib.rs` — trait accessible at `ora::editor_adapter::EditorDataSource`
- `cargo check -p ora` compiles clean; zero `use core_editor::` imports in editor_adapter module

## Task Commits

Each task was committed atomically:

1. **Task 1: Create mirror presentation types in ora** - `a53cef0` (feat)
2. **Task 2: Create EditorDataSource trait and module exports** - `1d56923` (feat)

## Files Created/Modified

- `ora/src/editor_adapter/types.rs` - All mirror presentation types + EditorCommand enum
- `ora/src/editor_adapter/mod.rs` - EditorDataSource trait definition and re-exports
- `ora/src/lib.rs` - Added `pub mod editor_adapter`

## Decisions Made

- Mirror types carry no `From` conversions — those belong in `wgpu_client` where both type namespaces are available. This keeps ora free of core_editor.
- `EditorCommand` defined alongside presentation types in `types.rs` — one file owns the full adapter surface.
- `CursorDirection` and `MoveScope` defined as ora-native enums to mirror `core_editor::commands::Direction` and `MoveScope` without importing them.

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- `ora::editor_adapter::EditorDataSource` and all mirror types are ready
- Plan 09-03 can now migrate ora views (TextAreaView, TabBarView, SidebarView, etc.) to accept `&dyn EditorDataSource` instead of `&core_editor::view_model::RenderModel`
- Plan 09-04 (wgpu_client implements EditorDataSource) depends on both 09-02 (this plan) and 09-03

---
*Phase: 09-transitions-integration*
*Completed: 2026-03-25*
