---
phase: 14-file-browser
plan: "01"
subsystem: ui
tags: [rust, sidebar, filesystem, state-persistence, dotfiles, ignored_patterns]

# Dependency graph
requires:
  - phase: 12-file-operations
    provides: state.json pattern (load_last_dir/save_last_dir, config_state_path)
  - phase: 13.1-rendering-performance-optimization
    provides: Sidebar viewport virtualization, FileTreeView, tree_dirty flag, mark_tree_dirty
provides:
  - Sidebar with ignored_patterns filtering (only .git hidden by default, dotfiles visible)
  - auto_expand_first_level() — first-level directories expanded on workspace open
  - AppState struct with last_dir + workspace_path + ignored_patterns in state.json
  - load_state() / save_state() replacing load_last_dir() / save_last_dir()
  - Workspace restore at startup via load_workspace_path() + main.rs startup logic
affects:
  - 14-02 (file watcher — calls mark_tree_dirty)
  - 14-03 (Open Folder dialog — will call open_directory + save_state)
  - 14-04 (external file modification — reads workspace_path from state)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ignored_patterns exact-name matching replaces dotfile starts_with filter"
    - "AppState struct + load_state/save_state for atomic multi-field state.json writes"
    - "Sidebar::new_with_patterns() for injecting persisted patterns at construction"

key-files:
  created: []
  modified:
    - core_editor/src/view/sidebar.rs
    - wgpu_client/src/adapter.rs
    - wgpu_client/src/main.rs

key-decisions:
  - "ignored_patterns uses exact name match (p == name), not starts_with — only .git hidden by default, .env/.gitignore visible"
  - "auto_expand_first_level called from both new_with_patterns() and set_base_directory() — always immediate on workspace open"
  - "AppState is private to adapter.rs, load_workspace_path() is the only pub fn for main.rs use"
  - "open_directory() builds app.sidebar directly (Sidebar::new_with_patterns) rather than going through App::open_directory — keeps ignored_patterns injection in adapter layer"

patterns-established:
  - "Pattern: save_state writes all fields atomically — never partial writes that would clobber keys"
  - "Pattern: load_state returns AppState with defaults for missing fields — forward compatible"
  - "Pattern: extract_json_string_field / extract_json_string_array helpers for hand-rolled JSON parsing"

# Metrics
duration: 5min
completed: 2026-03-27
---

# Phase 14 Plan 01: Sidebar Filtering + State Persistence Summary

**Dotfile filter fixed to exact-match ignored_patterns (default [".git"]), state.json extended with workspace_path + ignored_patterns, and startup workspace restore wired via load_workspace_path()**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-03-27T18:48:49Z
- **Completed:** 2026-03-27T18:53:08Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- Fixed critical dotfile filtering bug: sidebar now shows `.env`, `.gitignore`, `.rustfmt.toml` etc.; only `.git` hidden by default
- Extended state.json to persist `workspace_path` and `ignored_patterns` alongside `last_dir` in a single atomic write
- Startup workspace restore: launching muda with no args reopens the last opened workspace automatically
- First-level directory auto-expand: all root-level dirs expanded immediately when a workspace opens (via `auto_expand_first_level()`)
- Added `mark_tree_dirty()` public method for filesystem watcher integration in Plan 02
- Added 5 new sidebar tests covering the new behavior

## Task Commits

Each task was committed atomically:

1. **Task 1: Fix sidebar filtering + add ignored_patterns support** - `5193342` (feat)
2. **Task 2: Refactor state.json persistence + workspace restore at startup** - `e3e065c` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `core_editor/src/view/sidebar.rs` — `ignored_patterns` field, `new_with_patterns()`, `set_ignored_patterns()`, `mark_tree_dirty()`, `auto_expand_first_level()`, updated `read_dir_sorted` signature, 5 new tests
- `wgpu_client/src/adapter.rs` — `AppState` struct, `load_state()`/`save_state()` replacing `load_last_dir()`/`save_last_dir()`, `load_workspace_path()` pub fn, `workspace_path`/`ignored_patterns` fields on `CoreEditorAdapter`, updated `open_directory()`, `handle_file_loaded()`, `handle_file_saved()`
- `wgpu_client/src/main.rs` — startup workspace restore: check `load_workspace_path()` when no path arg provided

## Decisions Made

- `ignored_patterns` uses exact name match (`p == &name`) rather than `contains` or glob — keeps it simple and predictable; `.git` matches exactly, `.gitkeep` is visible
- `auto_expand_first_level()` is called in both `new_with_patterns()` and `set_base_directory()` so it fires on initial load and on Open Folder
- `AppState` is a private module struct — only `load_workspace_path()` is pub, keeping the state.json format internal
- `open_directory()` in adapter constructs `App` directly then sets `app.sidebar` with `new_with_patterns()` rather than using `App::open_directory()` — avoids needing to modify `core_editor::App` API to accept `ignored_patterns`

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 02 (filesystem watcher): `mark_tree_dirty()` is public and ready for watcher integration
- Plan 03 (Open Folder dialog): `open_directory()` in adapter handles ignored_patterns injection; state.json has `workspace_path` slot
- state.json format is stable: three-key object with `last_dir`, `workspace_path`, `ignored_patterns`

---
*Phase: 14-file-browser*
*Completed: 2026-03-27*
