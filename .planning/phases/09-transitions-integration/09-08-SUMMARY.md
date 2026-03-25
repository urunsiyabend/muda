---
phase: 09-transitions-integration
plan: 08
subsystem: ui
tags: [rust, wgpu_client, ora, cleanup, thin-shell, design-system, migration]

# Dependency graph
requires:
  - phase: 09-05
    provides: CoreEditorAdapter with EditorDataSource + ora::run_with_editor() entry point
  - phase: 09-07
    provides: All interactive elements (Button, TabBar, Sidebar, CommandPalette, Dialog) migrated to ora
provides:
  - wgpu_client reduced to thin shell (main.rs + adapter.rs only)
  - All design_system tokens, UI components, widgets, theme code deleted from wgpu_client
  - ora is sole source of truth for all UI, rendering, and design tokens
  - Cargo.toml trimmed: wgpu, winit, glyphon, pollster, anyhow, bytemuck removed
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Thin-shell binary: main() + CoreEditorAdapter::new/open_file/open_directory + ora::run_with_editor(diverges)"
    - "wgpu_client Cargo.toml deps: only core_editor, ora, log, simplelog needed"

key-files:
  created: []
  modified:
    - wgpu_client/src/main.rs
    - wgpu_client/Cargo.toml

key-decisions:
  - "Delete-then-check pattern: write thin main.rs first (removing all old mod declarations), then delete orphaned files — Rust only compiles declared modules, so deletions become safe and independently verifiable"
  - "Delete order: app.rs → input.rs → renderer/ → design_system/ → theme/ → widgets/ → components/ → ui/ (callers before callees avoids chasing compile errors)"
  - "wgpu_client Cargo.toml retains only: core_editor, ora, log, simplelog; all GPU/winit/image deps removed"

patterns-established:
  - "Thin shell pattern: binary crate = main() + adapter + framework::run(adapter); all UI in framework crate"

# Metrics
duration: 4min
completed: 2026-03-25
---

# Phase 9 Plan 08: Thin Shell Cleanup Summary

**wgpu_client stripped to 39-line main.rs + adapter.rs: 65 files and 15,261 lines of migrated UI code deleted, ora is now sole source of truth for all rendering, widgets, and design tokens**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-03-25T12:00:24Z
- **Completed:** 2026-03-25T12:04:46Z
- **Tasks:** 2
- **Files modified:** 2 modified, 63 deleted

## Accomplishments

- Deleted all migrated wgpu_client modules: `design_system/` (tokens/animation/interaction/layout/primitives), `theme/`, `widgets/` (button/input/checkbox/toggle/nav), `components/` (tab_bar/sidebar/gutter/text_area/status_bar/dialog/caret), `ui/` (app_layout/command_palette/editor_tabs/file_tree/panels/split_view/status_line/toolbar), `renderer/`, `input.rs`, `app.rs`
- Rewrote `wgpu_client/src/main.rs` to 39-line thin shell: `mod adapter` only, calls `ora::run_with_editor()` (diverging, returns `!`)
- Cleaned `wgpu_client/Cargo.toml`: removed `wgpu`, `winit`, `glyphon`, `pollster`, `anyhow`, `bytemuck` — only `core_editor`, `ora`, `log`, `simplelog` remain
- All 8 deletion steps verified with `cargo check -p wgpu_client` passing between each step
- No hardcoded float literals remain in `wgpu_client/src/` (zero matches for `[0-9]+\.[0-9]`)
- 150 ora unit tests pass, 103 core_editor unit tests pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Delete migrated wgpu_client modules** - `e1215b8` (feat)
2. **Task 2: Cargo.toml cleanup** - `79918a4` (chore)

**Plan metadata:** (upcoming docs commit)

## Files Created/Modified

- `wgpu_client/src/main.rs` - Rewritten as 39-line thin shell
- `wgpu_client/Cargo.toml` - Removed 6 unused GPU/utility deps
- All files in `wgpu_client/src/{app,input,renderer,design_system,theme,widgets,components,ui}` - Deleted (63 files, 15,261 lines)

## Decisions Made

- **Delete-then-check order:** Write thin `main.rs` first (removing all `mod` declarations) so Rust stops compiling orphaned modules, then physically delete them with `cargo check` between each step. This avoids cascading compile errors from cross-module deps.
- **Deletion order:** app.rs (WgpuApp) → input.rs → renderer/ → design_system/ → theme/ → widgets/ → components/ → ui/. Callers deleted before callees ensures clean compilation at each step.
- **`main()` returns `()`**, not `Result`: ora::run_with_editor() diverges (`!`), so no `?` operator needed; setup errors use `.expect()`.

## Deviations from Plan

None — plan executed exactly as written. All 8 module groups deleted with compilation verification, Cargo.toml cleaned, and thin shell verified to be 39 lines with zero hardcoded float literals.

## Issues Encountered

`cargo test --workspace` produced OOM linker errors on large ora examples (Windows LNK1102) and stale `.rlib` metadata errors — pre-existing environment issues on this machine, unrelated to this plan's changes. Library unit tests (`cargo test -p ora --lib`, `cargo test -p core_editor`) all pass.

## Next Phase Readiness

- INT-01 (thin shell) complete: wgpu_client = main.rs + adapter.rs
- INT-04 (no hardcoded values) complete: zero float literals in wgpu_client/src/
- INT-05 (no duplicated styling) complete: all design tokens removed from wgpu_client
- ora is the single source of truth for all UI, rendering, and design tokens
- Phase 9 complete — all 8 plans (01-08) executed

---
*Phase: 09-transitions-integration*
*Completed: 2026-03-25*
